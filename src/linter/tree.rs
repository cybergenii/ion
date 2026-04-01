use crate::linter::diagnostic::Diagnostic;
use crate::linter::rules::modern;
use anyhow::Result;
use std::fs;
use std::path::Path;
use tree_sitter::Parser;

pub fn run_tree_sitter_checks(file: &Path) -> Result<Vec<Diagnostic>> {
    let content = fs::read_to_string(file)?;
    run_tree_sitter_checks_with_content(file, &content)
}

pub fn run_tree_sitter_checks_with_content(file: &Path, content: &str) -> Result<Vec<Diagnostic>> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_cpp::language())
        .map_err(|e| anyhow::anyhow!("failed to set C++ grammar: {e}"))?;
    let _ = parser.parse(content, None);
    Ok(modern::run_modern_checks(file, content))
}
