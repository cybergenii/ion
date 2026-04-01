use anyhow::{Context, Result};
use regex::Regex;
use std::path::Path;

use crate::lockfile::LockedPackage;

use super::config_gen::cmake_canonical_name;

const ION_DEPS_BEGIN: &str = "# === ION DEPS BEGIN ===";
const ION_DEPS_END: &str = "# === ION DEPS END ===";
const ION_LINKS_BEGIN: &str = "# === ION LINKS BEGIN ===";
const ION_LINKS_END: &str = "# === ION LINKS END ===";

/// Manages the `CMakeLists.txt` file, preserving user content while
/// updating only Ion-managed sections.
pub struct CmakeGenerator {
    cmake_path: std::path::PathBuf,
}

impl CmakeGenerator {
    pub fn new(project_root: &Path) -> Self {
        Self {
            cmake_path: project_root.join("CMakeLists.txt"),
        }
    }

    /// Re-generate the Ion-managed sections of CMakeLists.txt.
    ///
    /// Ion manages two marker blocks:
    /// 1. `ION DEPS BEGIN/END` — find_package() calls + CMAKE_PREFIX_PATH
    /// 2. `ION LINKS BEGIN/END` — target_link_libraries() entries
    pub fn update(&self, packages: &[LockedPackage], ion_cmake_dir: &Path) -> Result<()> {
        if !self.cmake_path.exists() {
            anyhow::bail!("CMakeLists.txt not found at {}", self.cmake_path.display());
        }

        let content = std::fs::read_to_string(&self.cmake_path)
            .context("Failed to read CMakeLists.txt")?;

        let updated = self.update_deps_block(&content, packages, ion_cmake_dir)?;
        let updated = self.update_links_block(&updated, packages)?;

        std::fs::write(&self.cmake_path, updated)
            .context("Failed to write CMakeLists.txt")?;

        Ok(())
    }

    fn update_deps_block(
        &self,
        content: &str,
        packages: &[LockedPackage],
        ion_cmake_dir: &Path,
    ) -> Result<String> {
        let new_block = self.generate_deps_block(packages, ion_cmake_dir);

        if content.contains(ION_DEPS_BEGIN) {
            // Replace existing block
            replace_between(content, ION_DEPS_BEGIN, ION_DEPS_END, &new_block)
        } else {
            // Inject after the last cmake_minimum_required / project() call
            let inject_after = find_inject_point_after_project(content);
            let mut result = content.to_string();
            result.insert_str(inject_after, &format!("\n{}\n", new_block));
            Ok(result)
        }
    }

    fn update_links_block(&self, content: &str, packages: &[LockedPackage]) -> Result<String> {
        let new_block = self.generate_links_block(packages);

        if content.contains(ION_LINKS_BEGIN) {
            replace_between(content, ION_LINKS_BEGIN, ION_LINKS_END, &new_block)
        } else {
            // Find target_link_libraries call and inject before the closing paren
            if let Some(updated) = inject_into_target_link_libraries(content, &new_block) {
                Ok(updated)
            } else {
                // No target_link_libraries found — append a placeholder at end
                Ok(format!(
                    "{}\n# Add link libraries to your target:\n# target_link_libraries(${{PROJECT_NAME}} PRIVATE\n{}\n# )\n",
                    content, new_block
                ))
            }
        }
    }

    fn generate_deps_block(&self, packages: &[LockedPackage], ion_cmake_dir: &Path) -> String {
        if packages.is_empty() {
            return format!("{}\n{}", ION_DEPS_BEGIN, ION_DEPS_END);
        }

        let mut lines = vec![
            ION_DEPS_BEGIN.to_string(),
            "# Ion-managed dependencies — do not edit this block manually.".to_string(),
            "# Run `ion install` or `ion add/remove` to update.".to_string(),
            format!("list(APPEND CMAKE_PREFIX_PATH \"{}\")", ion_cmake_dir.display()),
            String::new(),
        ];

        let runtime_pkgs: Vec<_> = packages.iter().filter(|p| !p.dev_only).collect();
        let dev_pkgs: Vec<_> = packages.iter().filter(|p| p.dev_only).collect();

        if !runtime_pkgs.is_empty() {
            lines.push("# Runtime dependencies".to_string());
            for pkg in &runtime_pkgs {
                let canonical = cmake_canonical_name(&pkg.name);
                lines.push(format!("find_package({} {} REQUIRED)", canonical, pkg.version));
            }
        }

        if !dev_pkgs.is_empty() {
            lines.push(String::new());
            lines.push("# Development/test dependencies".to_string());
            for pkg in &dev_pkgs {
                let canonical = cmake_canonical_name(&pkg.name);
                lines.push(format!(
                    "if(BUILDING_TESTS)\n    find_package({} {} REQUIRED)\nendif()",
                    canonical, pkg.version
                ));
            }
        }

        lines.push(ION_DEPS_END.to_string());
        lines.join("\n")
    }

    fn generate_links_block(&self, packages: &[LockedPackage]) -> String {
        if packages.is_empty() {
            return format!("    {}\n    {}", ION_LINKS_BEGIN, ION_LINKS_END);
        }

        let runtime_pkgs: Vec<_> = packages.iter().filter(|p| !p.dev_only).collect();

        if runtime_pkgs.is_empty() {
            return format!("    {}\n    {}", ION_LINKS_BEGIN, ION_LINKS_END);
        }

        let mut lines = vec![
            format!("    {}", ION_LINKS_BEGIN),
            "    # Ion-managed link targets".to_string(),
        ];

        for pkg in &runtime_pkgs {
            for target in &pkg.cmake_targets {
                lines.push(format!("    {}", target));
            }
        }

        lines.push(format!("    {}", ION_LINKS_END));
        lines.join("\n")
    }

    /// Check if CMakeLists.txt has the Ion marker blocks
    pub fn has_ion_markers(&self) -> bool {
        if let Ok(content) = std::fs::read_to_string(&self.cmake_path) {
            content.contains(ION_DEPS_BEGIN)
        } else {
            false
        }
    }

    /// Add Ion markers to a CMakeLists.txt that doesn't have them yet.
    /// Also finds and annotates target_link_libraries.
    pub fn inject_markers(&self, packages: &[LockedPackage], ion_cmake_dir: &Path) -> Result<()> {
        self.update(packages, ion_cmake_dir)
    }
}

/// Replace content between two marker strings
fn replace_between(content: &str, begin: &str, end: &str, new_block: &str) -> Result<String> {
    let begin_pos = content.find(begin).ok_or_else(|| {
        anyhow::anyhow!("Could not find marker '{}' in CMakeLists.txt", begin)
    })?;
    let end_pos = content[begin_pos..].find(end).ok_or_else(|| {
        anyhow::anyhow!("Could not find closing marker '{}' in CMakeLists.txt", end)
    })? + begin_pos + end.len();

    Ok(format!(
        "{}{}\n{}",
        &content[..begin_pos],
        new_block,
        &content[end_pos..]
    ))
}

/// Find the position right after the project() call to inject deps
fn find_inject_point_after_project(content: &str) -> usize {
    let re = Regex::new(r"(?i)project\([^)]*\)\s*\n").unwrap();
    if let Some(m) = re.find(content) {
        return m.end();
    }
    // Fallback: inject at end
    content.len()
}

/// Inject into an existing target_link_libraries block
fn inject_into_target_link_libraries(content: &str, new_block: &str) -> Option<String> {
    // Find target_link_libraries( ... ) block
    let re = Regex::new(r"(?s)(target_link_libraries\([^)]*)(PUBLIC|PRIVATE|INTERFACE)(\s*)([^)]*)\)").unwrap();

    if let Some(cap) = re.captures(content) {
        let full_match = cap.get(0)?.as_str();
        let before = &content[..cap.get(0)?.start()];
        let after = &content[cap.get(0)?.end()..];

        // Check if ION_LINKS markers are already there
        if full_match.contains(ION_LINKS_BEGIN) {
            return None; // Already has markers, replace_between handles it
        }

        let vis = cap.get(2)?.as_str();
        let ws = cap.get(3)?.as_str();
        let existing_libs = cap.get(4)?.as_str();

        let new_target_link = format!(
            "{}({}{}{}\n{}\n    {})",
            &cap.get(1)?.as_str(),
            vis,
            ws,
            existing_libs.trim_end(),
            new_block,
            ""
        );

        return Some(format!("{}{}{}", before, new_target_link, after));
    }
    None
}
