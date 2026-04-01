pub mod graph;
pub mod semver_utils;

use anyhow::Result;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;

use crate::lockfile::{hash_manifest, Lockfile, LockedPackage};
use crate::manifest::{Dependency, DetailedDependency, Manifest};
use crate::registry::{DependencySpec, GitRev, PackageInfo, RegistryManager};

use self::graph::{DependencyGraph, ResolvedNode};

/// The result of resolving all dependencies.
#[derive(Debug)]
pub struct ResolvedGraph {
    /// Packages in topological order (dependencies before dependents)
    pub packages: Vec<ResolvedNode>,
}

/// Resolve all dependencies from a manifest.
///
/// Fast path: if a lockfile exists and the manifest hasn't changed,
/// return the locked versions directly without querying the registry.
///
/// Slow path: query each registry, build the dep graph, topologically sort,
/// detect conflicts, then return the resolved set.
pub async fn resolve(
    manifest: &Manifest,
    registry: &RegistryManager,
    existing_lock: Option<&Lockfile>,
    force_update: bool,
) -> Result<ResolvedGraph> {
    // Fast path: lockfile is fresh
    if !force_update {
        if let Some(lock) = existing_lock {
            let manifest_str = toml::to_string(manifest).unwrap_or_default();
            let current_hash = hash_manifest(&manifest_str);
            if lock.is_fresh(&current_hash) {
                println!("{}", "  Lockfile is up to date, using cached resolution.".dimmed());
                return Ok(ResolvedGraph {
                    packages: lock
                        .packages
                        .iter()
                        .map(|p| ResolvedNode {
                            name: p.name.clone(),
                            version: p.version.clone(),
                            source_uri: p.source.clone(),
                            cmake_targets: p.cmake_targets.clone(),
                            direct_deps: p.dependencies.clone(),
                        })
                        .collect(),
                });
            }
        }
    }

    println!("{}", "  Resolving dependencies...".cyan());

    // Collect all top-level dependency specs
    let mut all_specs: Vec<(String, DependencySpec, bool)> = Vec::new();
    for (name, dep) in &manifest.dependencies {
        let spec = parse_dependency_spec(name, dep);
        all_specs.push((name.clone(), spec, false));
    }
    for (name, dep) in &manifest.dev_dependencies {
        let spec = parse_dependency_spec(name, dep);
        all_specs.push((name.clone(), spec, true));
    }

    if all_specs.is_empty() {
        println!("{}", "  No dependencies to resolve.".dimmed());
        return Ok(ResolvedGraph { packages: vec![] });
    }

    let pb = ProgressBar::new(all_specs.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("  {spinner:.cyan} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("█▉▊▋▌▍▎▏  "),
    );

    // Resolve each top-level dep from the registry
    let mut resolved_map: HashMap<String, PackageInfo> = HashMap::new();
    let mut queue: Vec<(String, DependencySpec, bool)> = all_specs;

    while !queue.is_empty() {
        let mut next_queue = Vec::new();

        for (name, spec, is_dev) in queue.drain(..) {
            pb.set_message(format!("resolving {}", name.cyan()));

            let info = registry.resolve_dep(&spec).await?;

            // Check for version conflicts
            if let Some(existing) = resolved_map.get(&info.name) {
                if existing.version != info.version {
                    // Version conflict — fail with a clear message
                    anyhow::bail!(
                        "Dependency conflict: '{}' is required at both version '{}' and '{}'. \
                         Specify an explicit version in ion.toml to resolve this.",
                        info.name, existing.version, info.version
                    );
                }
            } else {
                // Recursively resolve transitive dependencies
                for dep in &info.dependencies {
                    if dep.optional {
                        continue;
                    }
                    if !resolved_map.contains_key(&dep.name) {
                        let transitive_spec = DependencySpec::Ion {
                            name: dep.name.clone(),
                            version_req: dep.version_req.clone(),
                            features: vec![],
                        };
                        next_queue.push((dep.name.clone(), transitive_spec, is_dev));
                    }
                }
                resolved_map.insert(info.name.clone(), info);
            }

            pb.inc(1);
        }

        queue = next_queue;
    }

    pb.finish_with_message("resolved".green().to_string());

    // Build and sort the dependency graph
    let mut graph = DependencyGraph::new();
    for (_, info) in &resolved_map {
        let deps: Vec<String> = info.dependencies.iter().map(|d| d.name.clone()).collect();
        graph.add_node(info.name.clone(), info.version.clone(), deps);
    }

    let sorted = graph.topological_sort()?;

    let packages = sorted
        .into_iter()
        .map(|node_name| {
            let info = &resolved_map[&node_name];
            ResolvedNode {
                name: info.name.clone(),
                version: info.version.clone(),
                source_uri: info.source_uri.clone(),
                cmake_targets: info.cmake_targets.clone(),
                direct_deps: info
                    .dependencies
                    .iter()
                    .map(|d| format!("{} {}", d.name, d.version_req))
                    .collect(),
            }
        })
        .collect();

    Ok(ResolvedGraph { packages })
}

/// Parse a manifest Dependency entry into a typed DependencySpec
pub fn parse_dependency_spec(name: &str, dep: &Dependency) -> DependencySpec {
    match dep {
        Dependency::Simple(version_req) => DependencySpec::Ion {
            name: name.to_string(),
            version_req: version_req.clone(),
            features: vec![],
        },
        Dependency::Detailed(detail) => {
            // Git dependency
            if let Some(git_url) = &detail.git {
                let rev = if let Some(tag) = &detail.tag {
                    GitRev::Tag(tag.clone())
                } else if let Some(branch) = &detail.branch {
                    GitRev::Branch(branch.clone())
                } else if let Some(rev) = &detail.rev {
                    GitRev::Commit(rev.clone())
                } else {
                    GitRev::Branch("main".to_string())
                };
                return DependencySpec::Git {
                    name: name.to_string(),
                    url: git_url.clone(),
                    rev,
                };
            }
            // Conan dependency
            if let Some(conan_ref) = &detail.conan {
                return DependencySpec::Conan {
                    reference: conan_ref.clone(),
                };
            }
            // vcpkg dependency
            if let Some(vcpkg_port) = &detail.vcpkg {
                return DependencySpec::Vcpkg {
                    port: vcpkg_port.clone(),
                    features: detail.features.clone(),
                };
            }
            // Local path dependency
            if let Some(path) = &detail.path {
                return DependencySpec::Local {
                    name: name.to_string(),
                    path: std::path::PathBuf::from(path),
                };
            }
            // Default: Ion registry
            DependencySpec::Ion {
                name: name.to_string(),
                version_req: detail.version.clone(),
                features: detail.features.clone(),
            }
        }
    }
}
