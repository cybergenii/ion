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

/// Simple `(Type)expr` on one line → `static_cast<Type>(expr)` when `Type` is a single identifier.
pub fn try_static_cast_fix(line: &str) -> Option<(usize, usize, String)> {
    if line.contains("static_cast") || line.contains("reinterpret_cast") {
        return None;
    }
    let open = line.find('(')?;
    let after = &line[open + 1..];
    let close_rel = after.find(')')?;
    let ty = &after[..close_rel];
    if ty.is_empty() || !ty.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return None;
    }
    let tail = after[close_rel + 1..].trim_start();
    let end_expr = tail
        .find(|c: char| c.is_whitespace() || c == ';' || c == ',' || c == ')')
        .unwrap_or(tail.len());
    let expr = &tail[..end_expr];
    if expr.is_empty() {
        return None;
    }
    let replacement = format!("static_cast<{ty}>({expr})");
    let col_start = open + 1;
    let col_end = open + 1 + close_rel + 1 + end_expr + 1;
    Some((col_start, col_end, replacement))
}

pub fn check_c_casts(file: &Path, content: &str) -> Vec<Diagnostic> {
    content
        .lines()
        .enumerate()
        .filter_map(|(idx, line)| {
            let (cs, ce, repl) = try_static_cast_fix(line)?;
            Some(Diagnostic {
                rule: "modern/c-cast",
                severity: Severity::Info,
                message: "C-style cast detected".to_string(),
                file: file.to_path_buf(),
                line: (idx + 1) as u32,
                column: cs as u32,
                span: None,
                suggestion: Some("Prefer `static_cast<Type>(expr)`".to_string()),
                fix: Some(Fix::Replace {
                    line: (idx + 1) as u32,
                    col_start: cs,
                    col_end: ce,
                    replacement: repl,
                }),
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
                suggestion: Some(
                    "Use `for (auto& item : container)` when index not required".to_string(),
                ),
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

    #[test]
    fn try_static_cast_fix_rewrites_int_cast() {
        let (_, _, r) = try_static_cast_fix("auto x = (int)y;").expect("expected");
        assert_eq!(r, "static_cast<int>(y)");
    }

    #[test]
    fn check_c_casts_emits_fix() {
        let diags = check_c_casts(Path::new("a.cpp"), "auto x = (int)y;");
        assert!(diags.iter().any(|d| d.fix.is_some()));
    }
}
