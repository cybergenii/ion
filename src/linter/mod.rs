pub mod diagnostic;
pub mod engine;
pub mod fixer;
pub mod reporter;
pub mod rules;
pub mod tree;
pub mod watcher;

use crate::analysis::dataflow;
use crate::analysis::smart_ptr;
use anyhow::Result;
use diagnostic::{Diagnostic, Severity};
use engine::LintEngine;
use rayon::prelude::*;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use walkdir::WalkDir;

pub struct LintSummary {
    pub errors: usize,
    pub warnings: usize,
    pub files: usize,
    pub elapsed_secs: f64,
}

pub struct Linter {
    engine: LintEngine,
}

impl Linter {
    pub fn new() -> Self {
        Self {
            engine: LintEngine::new(),
        }
    }

    pub fn semantic_available(&self) -> bool {
        self.engine.semantic_available()
    }

    pub fn discover_source_files(root: &Path) -> Vec<PathBuf> {
        WalkDir::new(root)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
            .map(|e| e.into_path())
            .filter(|p| {
                matches!(
                    p.extension().and_then(|s| s.to_str()),
                    Some("cpp" | "cc" | "cxx" | "h" | "hpp")
                )
            })
            .collect()
    }

    pub fn run_on_files(
        &self,
        files: &[PathBuf],
        filter_rules: Option<&[String]>,
    ) -> Result<Vec<Diagnostic>> {
        let diagnostics = files
            .par_iter()
            .map(|file| self.analyze_one(file, filter_rules, None))
            .collect::<Vec<_>>()
            .into_iter()
            .filter_map(Result::ok)
            .flatten()
            .collect();
        Ok(diagnostics)
    }

    /// Analyze a single file using an in-memory buffer instead of disk (e.g. LSP unsaved documents).
    pub fn analyze_file_with_source(
        &self,
        file: &Path,
        source: &str,
        filter_rules: Option<&[String]>,
    ) -> Result<Vec<Diagnostic>> {
        self.analyze_one(file, filter_rules, Some(source))
    }

    pub fn run(&self, filter_rules: Option<&[String]>) -> Result<(Vec<Diagnostic>, LintSummary)> {
        let start = Instant::now();
        let files = Self::discover_source_files(Path::new("src"));
        let diagnostics = self.run_on_files(&files, filter_rules)?;
        let errors = diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count();
        let warnings = diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .count();
        let elapsed_secs = start.elapsed().as_secs_f64();
        Ok((
            diagnostics,
            LintSummary {
                errors,
                warnings,
                files: files.len(),
                elapsed_secs,
            },
        ))
    }

    fn analyze_one(
        &self,
        file: &Path,
        filter_rules: Option<&[String]>,
        overlay: Option<&str>,
    ) -> Result<Vec<Diagnostic>> {
        let source: std::borrow::Cow<'_, str> = match overlay {
            Some(s) => std::borrow::Cow::Borrowed(s),
            None => std::borrow::Cow::Owned(fs::read_to_string(file)?),
        };
        let source_ref = source.as_ref();

        let mut out = tree::run_tree_sitter_checks_with_content(file, source_ref)?;
        out.extend(dataflow::quick_dataflow_checks(file, source_ref));
        out.extend(smart_ptr::smart_ptr_checks(file, source_ref));
        if self.engine.semantic_available() {
            out.extend(
                self.engine
                    .analyze_file_with_source(file, source_ref, filter_rules)?,
            );
        }
        if let Some(ids) = filter_rules {
            out.retain(|d| ids.iter().any(|r| r.as_str() == d.rule));
        }
        Ok(out)
    }

    pub fn unique_files_count(diagnostics: &[Diagnostic]) -> usize {
        diagnostics
            .iter()
            .map(|d| d.file.clone())
            .collect::<BTreeSet<_>>()
            .len()
    }
}
