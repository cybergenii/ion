use crate::linter::diagnostic::{Diagnostic, Fix, Severity};
use lsp_types::{
    Diagnostic as LspDiagnostic, DiagnosticSeverity, Position, Range, TextEdit, Url, WorkspaceEdit,
};
use std::collections::HashMap;

pub fn to_lsp_diagnostic(d: &Diagnostic) -> LspDiagnostic {
    LspDiagnostic {
        range: to_lsp_range(
            d.line,
            d.span.map(|s| s.0).unwrap_or(d.column),
            d.span.map(|s| s.1).unwrap_or(d.column + 1),
        ),
        severity: Some(match d.severity {
            Severity::Error => DiagnosticSeverity::ERROR,
            Severity::Warning => DiagnosticSeverity::WARNING,
            Severity::Info => DiagnosticSeverity::INFORMATION,
            Severity::Hint => DiagnosticSeverity::HINT,
        }),
        code: None,
        code_description: None,
        source: Some("ion".to_string()),
        message: format!("[{}] {}", d.rule, d.message),
        related_information: None,
        tags: None,
        data: None,
    }
}

pub fn to_lsp_range(line: u32, col_start: u32, col_end: u32) -> Range {
    Range {
        start: Position::new(line.saturating_sub(1), col_start.saturating_sub(1)),
        end: Position::new(line.saturating_sub(1), col_end.saturating_sub(1)),
    }
}

pub fn to_workspace_edit(fix: &Fix, uri: &Url) -> WorkspaceEdit {
    let edit = match fix {
        Fix::Replace {
            line,
            col_start,
            col_end,
            replacement,
        } => TextEdit {
            range: to_lsp_range(*line, *col_start as u32, *col_end as u32),
            new_text: replacement.clone(),
        },
        Fix::InsertAfter { line, text } => TextEdit {
            range: to_lsp_range(*line + 1, 1, 1),
            new_text: format!("{text}\n"),
        },
        Fix::DeleteLine { line } => TextEdit {
            range: to_lsp_range(*line, 1, u32::MAX / 2),
            new_text: String::new(),
        },
    };
    let mut map = HashMap::new();
    map.insert(uri.clone(), vec![edit]);
    WorkspaceEdit {
        changes: Some(map),
        document_changes: None,
        change_annotations: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::diagnostic::Diagnostic;
    use std::path::PathBuf;

    #[test]
    fn test_to_lsp_diagnostic_maps_fields() {
        let d = Diagnostic {
            rule: "memory/leak",
            severity: Severity::Warning,
            message: "msg".to_string(),
            file: PathBuf::from("a.cpp"),
            line: 3,
            column: 5,
            span: Some((5, 10)),
            suggestion: None,
            fix: None,
            note: None,
        };
        let out = to_lsp_diagnostic(&d);
        assert_eq!(out.range.start.line, 2);
        assert_eq!(out.range.start.character, 4);
        assert!(out.message.contains("memory/leak"));
    }
}
