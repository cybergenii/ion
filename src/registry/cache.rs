use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use super::{DownloadResult, PackageInfo};

/// On-disk package cache: ~/.cache/ion/packages/{source}/{name}/{version}/
pub struct PackageCache {
    root: PathBuf,
}

impl PackageCache {
    pub fn new(cache_dir: &Path) -> Result<Self> {
        let root = cache_dir.to_path_buf();
        fs::create_dir_all(&root).with_context(|| {
            format!("Failed to create cache directory: {}", root.display())
        })?;
        Ok(Self { root })
    }

    /// Return the extracted path for a package if it's already cached and valid.
    pub fn get_cached(&self, info: &PackageInfo) -> Result<Option<DownloadResult>> {
        let pkg_dir = self.package_dir(info);
        let meta_path = pkg_dir.join("meta.json");
        let src_dir = pkg_dir.join("src");

        if !meta_path.exists() || !src_dir.exists() {
            return Ok(None);
        }

        // Load cached metadata
        let meta_content = fs::read_to_string(&meta_path)?;
        let cached_meta: CachedMeta = serde_json::from_str(&meta_content)?;

        // Derive include/lib paths from what we know
        let include_dirs = find_include_dirs(&src_dir);
        let lib_files = find_lib_files(&src_dir);

        Ok(Some(DownloadResult {
            checksum: cached_meta.checksum,
            extracted_path: src_dir,
            include_dirs,
            lib_files,
            cmake_targets: info.cmake_targets.clone(),
        }))
    }

    /// Store a downloaded archive, verify its checksum, extract it, and return a DownloadResult.
    pub fn store(
        &self,
        info: &PackageInfo,
        archive_bytes: &[u8],
        expected_checksum: Option<&str>,
    ) -> Result<DownloadResult> {
        let pkg_dir = self.package_dir(info);
        let src_dir = pkg_dir.join("src");
        fs::create_dir_all(&src_dir)?;

        // Compute SHA-256
        let mut hasher = Sha256::new();
        hasher.update(archive_bytes);
        let checksum = format!("sha256:{}", hex::encode(hasher.finalize()));

        // Verify if expected provided
        if let Some(expected) = expected_checksum {
            if checksum != expected {
                anyhow::bail!(
                    "Checksum mismatch for {}@{}: expected {}, got {}",
                    info.name, info.version, expected, checksum
                );
            }
        }

        // Store raw archive
        let archive_path = pkg_dir.join("archive.tar.gz");
        fs::write(&archive_path, archive_bytes)?;

        // Extract archive
        self.extract_archive(&archive_path, &src_dir)?;

        // Write metadata
        let meta = CachedMeta {
            name: info.name.clone(),
            version: info.version.clone(),
            source: info.source_uri.clone(),
            checksum: checksum.clone(),
        };
        fs::write(pkg_dir.join("meta.json"), serde_json::to_string_pretty(&meta)?)?;

        let include_dirs = find_include_dirs(&src_dir);
        let lib_files = find_lib_files(&src_dir);

        Ok(DownloadResult {
            checksum,
            extracted_path: src_dir,
            include_dirs,
            lib_files,
            cmake_targets: info.cmake_targets.clone(),
        })
    }

    /// Store a zip archive (used by vcpkg, some GitHub releases)
    pub fn store_zip(
        &self,
        info: &PackageInfo,
        archive_bytes: &[u8],
        expected_checksum: Option<&str>,
    ) -> Result<DownloadResult> {
        let pkg_dir = self.package_dir(info);
        let src_dir = pkg_dir.join("src");
        fs::create_dir_all(&src_dir)?;

        let mut hasher = Sha256::new();
        hasher.update(archive_bytes);
        let checksum = format!("sha256:{}", hex::encode(hasher.finalize()));

        if let Some(expected) = expected_checksum {
            if checksum != expected {
                anyhow::bail!(
                    "Checksum mismatch for {}@{}: expected {}, got {}",
                    info.name, info.version, expected, checksum
                );
            }
        }

        let archive_path = pkg_dir.join("archive.zip");
        fs::write(&archive_path, archive_bytes)?;
        self.extract_zip(&archive_path, &src_dir)?;

        let meta = CachedMeta {
            name: info.name.clone(),
            version: info.version.clone(),
            source: info.source_uri.clone(),
            checksum: checksum.clone(),
        };
        fs::write(pkg_dir.join("meta.json"), serde_json::to_string_pretty(&meta)?)?;

        let include_dirs = find_include_dirs(&src_dir);
        let lib_files = find_lib_files(&src_dir);

        Ok(DownloadResult {
            checksum,
            extracted_path: src_dir,
            include_dirs,
            lib_files,
            cmake_targets: info.cmake_targets.clone(),
        })
    }

    /// Return the directory where the Ion-generated CMake config files are stored.
    /// This path is added to CMAKE_PREFIX_PATH so find_package() can find them.
    pub fn cmake_dir(&self) -> PathBuf {
        self.root.join("cmake")
    }

    /// The directory for a specific package version
    pub fn package_dir(&self, info: &PackageInfo) -> PathBuf {
        // Sanitize source to be filesystem safe
        let safe_source = info.source.replace("://", "_").replace('/', "_");
        self.root
            .join("packages")
            .join(&safe_source)
            .join(&info.name)
            .join(&info.version)
    }

    /// Return the extracted source directory for a package (if cached)
    pub fn src_dir(&self, info: &PackageInfo) -> PathBuf {
        self.package_dir(info).join("src")
    }

    fn extract_archive(&self, archive_path: &Path, dest: &Path) -> Result<()> {
        let file = fs::File::open(archive_path)?;
        let decoder = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(decoder);

        // Extract stripping the top-level directory
        for entry in archive.entries()? {
            let mut entry = entry?;
            let entry_path = entry.path()?;

            // Strip the first path component (top-level dir in the archive)
            let stripped = entry_path
                .components()
                .skip(1)
                .collect::<std::path::PathBuf>();

            if stripped.as_os_str().is_empty() {
                continue;
            }

            let out_path = dest.join(&stripped);
            if entry.header().entry_type().is_dir() {
                fs::create_dir_all(&out_path)?;
            } else {
                if let Some(parent) = out_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                let mut out_file = fs::File::create(&out_path)?;
                io::copy(&mut entry, &mut out_file)?;
            }
        }
        Ok(())
    }

    fn extract_zip(&self, archive_path: &Path, dest: &Path) -> Result<()> {
        let file = fs::File::open(archive_path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            let entry_path = PathBuf::from(entry.name());

            // Strip top-level directory
            let stripped = entry_path
                .components()
                .skip(1)
                .collect::<std::path::PathBuf>();

            if stripped.as_os_str().is_empty() {
                continue;
            }

            let out_path = dest.join(&stripped);
            if entry.is_dir() {
                fs::create_dir_all(&out_path)?;
            } else {
                if let Some(parent) = out_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                let mut out_file = fs::File::create(&out_path)?;
                let mut buf = Vec::new();
                entry.read_to_end(&mut buf)?;
                out_file.write_all(&buf)?;
            }
        }
        Ok(())
    }

    /// List all cached packages
    pub fn list_cached(&self) -> Result<Vec<CachedMeta>> {
        let packages_dir = self.root.join("packages");
        if !packages_dir.exists() {
            return Ok(vec![]);
        }

        let mut results = Vec::new();
        for source_entry in fs::read_dir(&packages_dir)? {
            for name_entry in fs::read_dir(source_entry?.path())? {
                for ver_entry in fs::read_dir(name_entry?.path())? {
                    let meta_path = ver_entry?.path().join("meta.json");
                    if meta_path.exists() {
                        let content = fs::read_to_string(&meta_path)?;
                        if let Ok(meta) = serde_json::from_str::<CachedMeta>(&content) {
                            results.push(meta);
                        }
                    }
                }
            }
        }
        Ok(results)
    }

    /// Remove a specific package from the cache
    pub fn evict(&self, info: &PackageInfo) -> Result<()> {
        let dir = self.package_dir(info);
        if dir.exists() {
            fs::remove_dir_all(&dir)?;
        }
        Ok(())
    }
}

/// Metadata stored alongside each cached package
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct CachedMeta {
    pub name: String,
    pub version: String,
    pub source: String,
    pub checksum: String,
}

/// Heuristically find include directories in an extracted package tree
fn find_include_dirs(root: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    for candidate in ["include", "include/c++", "inc"] {
        let p = root.join(candidate);
        if p.is_dir() {
            dirs.push(p);
        }
    }
    if dirs.is_empty() {
        // Package may put headers at root (header-only)
        if root.join("*.h").exists() || root.join("*.hpp").exists() {
            dirs.push(root.to_path_buf());
        }
    }
    dirs
}

/// Heuristically find library files in an extracted package tree
fn find_lib_files(root: &Path) -> Vec<PathBuf> {
    let mut libs = Vec::new();
    let candidates = walkdir::WalkDir::new(root)
        .max_depth(4)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file());

    for entry in candidates {
        let path = entry.path();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if matches!(ext, "a" | "lib" | "so" | "dylib" | "dll") {
                libs.push(path.to_path_buf());
            }
        }
    }
    libs
}
