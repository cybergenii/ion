use crate::linter::diagnostic::{Diagnostic, Fix};
use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub struct Fixer;

impl Fixer {
    pub fn apply_all(&self, diagnostics: &[Diagnostic]) -> Result<()> {
        let mut grouped: HashMap<PathBuf, Vec<(&'static str, Fix)>> = HashMap::new();
        for d in diagnostics {
            if let Some(fix) = d.fix.clone() {
                grouped
                    .entry(d.file.clone())
                    .or_default()
                    .push((d.rule, fix));
            }
        }
        for (file, mut fixes) in grouped {
            let mut lines: Vec<String> = fs::read_to_string(&file)?
                .lines()
                .map(ToOwned::to_owned)
                .collect();
            fixes.sort_by_key(|(_, f)| std::cmp::Reverse(line_of(f)));
            for (rule, fix) in fixes {
                match fix {
                    Fix::Replace {
                        line,
                        col_start,
                        col_end,
                        replacement,
                    } => {
                        if let Some(target) = lines.get_mut((line.saturating_sub(1)) as usize) {
                            let start = col_start.saturating_sub(1).min(target.len());
                            let end = col_end.saturating_sub(1).min(target.len());
                            if start <= end {
                                target.replace_range(start..end, &replacement);
                                println!("[ion] fixed {rule} in {}:{line}", file.display());
                            }
                        }
                    }
                    Fix::InsertAfter { line, text } => {
                        let idx = line as usize;
                        if idx <= lines.len() {
                            lines.insert(idx, text);
                            println!("[ion] fixed {rule} in {}:{line}", file.display());
                        }
                    }
                    Fix::DeleteLine { line } => {
                        let idx = (line.saturating_sub(1)) as usize;
                        if idx < lines.len() {
                            lines.remove(idx);
                            println!("[ion] fixed {rule} in {}:{line}", file.display());
                        }
                    }
                }
            }
            fs::write(&file, lines.join("\n"))?;
        }
        Ok(())
    }
}

fn line_of(fix: &Fix) -> u32 {
    match fix {
        Fix::Replace { line, .. } | Fix::InsertAfter { line, .. } | Fix::DeleteLine { line } => {
            *line
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::diagnostic::{Diagnostic, Severity};
    use std::path::PathBuf;

    #[test]
    fn test_apply_replace_fix() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file = dir.path().join("a.cpp");
        fs::write(&file, "int* p = NULL;\n").expect("write");
        let d = Diagnostic {
            rule: "modern/nullptr",
            severity: Severity::Hint,
            message: "replace".to_string(),
            file: PathBuf::from(&file),
            line: 1,
            column: 10,
            span: None,
            suggestion: None,
            fix: Some(Fix::Replace {
                line: 1,
                col_start: 10,
                col_end: 14,
                replacement: "nullptr".to_string(),
            }),
            note: None,
        };
        let fixer = Fixer;
        fixer.apply_all(&[d]).expect("fix apply");
        let content = fs::read_to_string(&file).expect("read");
        assert!(content.contains("nullptr"));
    }

    #[test]
    fn test_apply_c_cast_static_cast_fix() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file = dir.path().join("cast.cpp");
        fs::write(&file, "auto x = (int)y;\n").expect("write");
        let d = Diagnostic {
            rule: "modern/c-cast",
            severity: Severity::Info,
            message: "c-style cast".to_string(),
            file: PathBuf::from(&file),
            line: 1,
            column: 1,
            span: None,
            suggestion: None,
            fix: Some(Fix::Replace {
                line: 1,
                col_start: 11,
                col_end: 17,
                replacement: "static_cast<int>(y)".to_string(),
            }),
            note: None,
        };
        let fixer = Fixer;
        fixer.apply_all(&[d]).expect("fix apply");
        let content = fs::read_to_string(&file).expect("read");
        assert!(content.contains("static_cast<int>(y)"));
    }
}
