use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    pub rule: &'static str,
    pub severity: Severity,
    pub message: String,
    pub file: PathBuf,
    pub line: u32,
    pub column: u32,
    pub span: Option<(u32, u32)>,
    pub suggestion: Option<String>,
    pub fix: Option<Fix>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum Severity {
    Error,
    Warning,
    Info,
    Hint,
}

#[derive(Debug, Clone, Serialize)]
pub enum Fix {
    Replace {
        line: u32,
        col_start: usize,
        col_end: usize,
        replacement: String,
    },
    InsertAfter {
        line: u32,
        text: String,
    },
    DeleteLine {
        line: u32,
    },
}
