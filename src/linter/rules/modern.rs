use crate::linter::diagnostic::{Diagnostic, Fix, Severity};
use std::path::Path;

pub fn run_modern_checks(file: &Path, content: &str) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    out.extend(check_null_literal(file, content));
    out.extend(check_c_casts(file, content));
    out.extend(check_printf(file, content));
    out.extend(check_index_for_loop(file, content));
    out.extend(check_push_back(file, content));
    out
}

pub fn check_null_literal(file: &Path, content: &str) -> Vec<Diagnostic> {
    content
        .lines()
        .enumerate()
        .filter_map(|(idx, line)| {
            let col = line.find("NULL")?;
            Some(Diagnostic {
                rule: "modern/nullptr",
                severity: Severity::Hint,
                message: "Use `nullptr` instead of `NULL`".to_string(),
                file: file.to_path_buf(),
                line: (idx + 1) as u32,
                column: (col + 1) as u32,
                span: Some(((col + 1) as u32, (col + 5) as u32)),
                suggestion: Some("Replace `NULL` with `nullptr`".to_string()),
                fix: Some(Fix::Replace {
                    line: (idx + 1) as u32,
                    col_start: col + 1,
                    col_end: col + 5,
                    replacement: "nullptr".to_string(),
                }),
                note: None,
            })
        })
        .collect()
}

pub fn check_c_casts(file: &Path, content: &str) -> Vec<Diagnostic> {
    content
        .lines()
        .enumerate()
        .filter_map(|(idx, line)| {
            if line.contains("static_cast<") || !line.contains('(') || !line.contains(')') {
                return None;
            }
            if !(line.contains(") ") || line.contains(")(")) {
                return None;
            }
            Some(Diagnostic {
                rule: "modern/c-cast",
                severity: Severity::Info,
                message: "C-style cast detected".to_string(),
                file: file.to_path_buf(),
                line: (idx + 1) as u32,
                column: 1,
                span: None,
                suggestion: Some("Prefer `static_cast<Type>(expr)`".to_string()),
                fix: None,
                note: None,
            })
        })
        .collect()
}

pub fn check_printf(file: &Path, content: &str) -> Vec<Diagnostic> {
    content
        .lines()
        .enumerate()
        .filter_map(|(idx, line)| {
            let col = line.find("printf(").or_else(|| line.find("sprintf("))?;
            Some(Diagnostic {
                rule: "modern/printf",
                severity: Severity::Info,
                message: "C stdio formatting function detected".to_string(),
                file: file.to_path_buf(),
                line: (idx + 1) as u32,
                column: (col + 1) as u32,
                span: None,
                suggestion: Some("Prefer `std::format` (C++20) where possible".to_string()),
                fix: None,
                note: None,
            })
        })
        .collect()
}

pub fn check_index_for_loop(file: &Path, content: &str) -> Vec<Diagnostic> {
    content
        .lines()
        .enumerate()
        .filter_map(|(idx, line)| {
            if !(line.contains("for (int i") && line.contains("i++")) {
                return None;
            }
            Some(Diagnostic {
                rule: "modern/range-for",
                severity: Severity::Hint,
                message: "Index-based for-loop may be replaced by range-for".to_string(),
                file: file.to_path_buf(),
                line: (idx + 1) as u32,
                column: 1,
                span: None,
                suggestion: Some("Use `for (auto& item : container)` when index not required".to_string()),
                fix: None,
                note: None,
            })
        })
        .collect()
}

pub fn check_push_back(file: &Path, content: &str) -> Vec<Diagnostic> {
    content
        .lines()
        .enumerate()
        .filter_map(|(idx, line)| {
            let col = line.find("push_back(")?;
            if !line[col..].contains('(') || !line[col..].contains(')') {
                return None;
            }
            if !line[col..].contains("::") && !line[col..].contains(")") {
                return None;
            }
            Some(Diagnostic {
                rule: "modern/emplace-back",
                severity: Severity::Hint,
                message: "Constructing temporary in `push_back`".to_string(),
                file: file.to_path_buf(),
                line: (idx + 1) as u32,
                column: (col + 1) as u32,
                span: None,
                suggestion: Some("Consider `emplace_back(...)`".to_string()),
                fix: None,
                note: None,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn modern_null_literal_detects_null() {
        let diags = check_null_literal(Path::new("a.cpp"), "int* p = NULL;");
        assert!(!diags.is_empty());
    }

    #[test]
    fn modern_printf_detects_printf() {
        let diags = check_printf(Path::new("a.cpp"), "printf(\"%d\", x);");
        assert!(!diags.is_empty());
    }
}
