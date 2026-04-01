use crate::linter::diagnostic::{Diagnostic, Fix, Severity};
use tower_lsp::lsp_types::{
    CodeDescription, Diagnostic as LspDiagnostic, DiagnosticRelatedInformation, DiagnosticSeverity,
    Location, NumberOrString, Position, Range, TextEdit, Url, WorkspaceEdit,
};
use std::collections::HashMap;

pub fn to_lsp_diagnostic(d: &Diagnostic) -> LspDiagnostic {
    let doc_url = Url::from_file_path(&d.file).ok();
    let related = d.note.as_ref().zip(doc_url.clone()).map(|(note, uri)| {
        vec![DiagnosticRelatedInformation {
            location: Location {
                uri,
                range: to_lsp_range(d.line, d.column, d.column.saturating_add(1)),
            },
            message: note.clone(),
        }]
    });
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
        code: Some(NumberOrString::String(d.rule.to_string())),
        code_description: Some(CodeDescription {
            href: Url::parse("https://github.com/cybergenii/ion").expect("valid url"),
        }),
        source: Some("ion".to_string()),
        message: format!("[{}] {}", d.rule, d.message),
        related_information: related,
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

pub fn ranges_overlap(a: &Range, b: &Range) -> bool {
    if a.start.line > b.end.line || b.start.line > a.end.line {
        return false;
    }
    if a.start.line == b.end.line && a.start.character > b.end.character {
        return false;
    }
    if b.start.line == a.end.line && b.start.character > a.end.character {
        return false;
    }
    true
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
        Fix::DeleteLine { line } => {
            let line_idx = line.saturating_sub(1);
            TextEdit {
                range: Range {
                    start: Position::new(line_idx, 0),
                    end: Position::new(line_idx, u32::MAX / 4),
                },
                new_text: String::new(),
            }
        }
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

    #[test]
    fn test_to_lsp_diagnostic_maps_fields() {
        let tmp = std::env::temp_dir().join("ion-lsp-test-a.cpp");
        let d = Diagnostic {
            rule: "memory/leak",
            severity: Severity::Warning,
            message: "msg".to_string(),
            file: tmp,
            line: 3,
            column: 5,
            span: Some((5, 10)),
            suggestion: None,
            fix: None,
            note: Some("note text".to_string()),
        };
        let out = to_lsp_diagnostic(&d);
        assert_eq!(out.range.start.line, 2);
        assert_eq!(out.range.start.character, 4);
        assert!(out.message.contains("memory/leak"));
        assert!(out.code.is_some());
        assert!(out.related_information.is_some());
    }
}
