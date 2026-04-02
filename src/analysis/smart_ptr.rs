//! Heuristic smart-pointer and ownership checks (textual; complements libclang rules).

use crate::linter::diagnostic::{Diagnostic, Severity};
use std::path::Path;

pub fn smart_ptr_checks(file: &Path, source: &str) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    out.extend(check_smart_get(file, source));
    out.extend(check_raw_from_smart(file, source));
    out.extend(check_move_after_use(file, source));
    out.extend(check_shared_cycle_hint(file, source));
    out
}

fn check_smart_get(file: &Path, source: &str) -> Vec<Diagnostic> {
    let mut v = Vec::new();
    for (idx, line) in source.lines().enumerate() {
        let line_no = (idx + 1) as u32;
        if !line.contains(".get()") {
            continue;
        }
        if line.trim_start().starts_with("//") {
            continue;
        }
        // Storing raw pointer from get()
        let risky = line.contains("auto*")
            || line.contains(" auto *")
            || line.contains("T*")
            || (line.contains('=') && line.contains('*') && line.contains(".get()"));
        if !risky {
            continue;
        }
        let col = line.find(".get()").map(|i| (i + 1) as u32).unwrap_or(1);
        v.push(Diagnostic {
            rule: "memory/smart-get",
            severity: Severity::Info,
            message: "`.get()` yields a raw pointer; ensure it does not outlive the smart pointer"
                .to_string(),
            file: file.to_path_buf(),
            line: line_no,
            column: col,
            span: None,
            suggestion: Some("Prefer references or pass the smart pointer by const&".to_string()),
            fix: None,
            note: Some("Heuristic check".to_string()),
        });
    }
    v
}

fn check_raw_from_smart(file: &Path, source: &str) -> Vec<Diagnostic> {
    let mut v = Vec::new();
    for (idx, line) in source.lines().enumerate() {
        let line_no = (idx + 1) as u32;
        if line.trim_start().starts_with("//") {
            continue;
        }
        if !line.contains(".get()") {
            continue;
        }
        if line.contains("return ") && line.contains(".get()") {
            let col = line.find("return").map(|i| (i + 1) as u32).unwrap_or(1);
            v.push(Diagnostic {
                rule: "memory/raw-from-smart",
                severity: Severity::Warning,
                message: "Returning raw pointer from `.get()` may escape smart-pointer lifetime"
                    .to_string(),
                file: file.to_path_buf(),
                line: line_no,
                column: col,
                span: None,
                suggestion: Some(
                    "Return `const std::shared_ptr<T>&` / `const T&` or document lifetime contract"
                        .to_string(),
                ),
                fix: None,
                note: None,
            });
        }
    }
    v
}

fn check_move_after_use(file: &Path, source: &str) -> Vec<Diagnostic> {
    let mut v = Vec::new();
    let lines: Vec<&str> = source.lines().collect();
    for i in 0..lines.len() {
        let line = lines[i];
        if line.trim_start().starts_with("//") {
            continue;
        }
        let Some(pos) = line.find("std::move(") else {
            continue;
        };
        let rest = &line[pos + 10..];
        let var = rest
            .split(')')
            .next()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let Some(var) = var else { continue };
        for (j, next) in lines
            .iter()
            .enumerate()
            .take(lines.len().min(i + 12))
            .skip(i + 1)
        {
            let next = *next;
            if next.trim_start().starts_with("//") {
                continue;
            }
            if next.contains(var) && !next.contains("std::move(") {
                let line_no = (j + 1) as u32;
                let col = next.find(var).map(|k| (k + 1) as u32).unwrap_or(1);
                v.push(Diagnostic {
                    rule: "memory/move-after-use",
                    severity: Severity::Warning,
                    message: format!(
                        "`{var}` may be used after `std::move` (moved-from state is often invalid)"
                    ),
                    file: file.to_path_buf(),
                    line: line_no,
                    column: col,
                    span: None,
                    suggestion: Some(
                        "Avoid using the moved-from object unless it is documented as safe"
                            .to_string(),
                    ),
                    fix: None,
                    note: Some("Heuristic: verify move semantics for this type".to_string()),
                });
                break;
            }
        }
    }
    v
}

fn check_shared_cycle_hint(file: &Path, source: &str) -> Vec<Diagnostic> {
    let mut v = Vec::new();
    let lines: Vec<&str> = source.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if !(line.contains("class ") || line.contains("struct ")) {
            continue;
        }
        let shared_in_window = lines
            .iter()
            .skip(i)
            .take(48)
            .filter(|l| l.contains("shared_ptr"))
            .count();
        if shared_in_window < 2 {
            continue;
        }
        v.push(Diagnostic {
            rule: "memory/shared-cycle-hint",
            severity: Severity::Info,
            message: "Multiple `shared_ptr` members may form reference cycles; consider `weak_ptr` for back-edges"
                .to_string(),
            file: file.to_path_buf(),
            line: (i + 1) as u32,
            column: 1,
            span: None,
            suggestion: Some(
                "Use `std::weak_ptr` for back-references or restructure ownership".to_string(),
            ),
            fix: None,
            note: Some("Heuristic: verify with design intent".to_string()),
        });
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn detects_smart_get() {
        let s = "auto* x = p.get();";
        let d = smart_ptr_checks(Path::new("t.cpp"), s);
        assert!(d.iter().any(|x| x.rule == "memory/smart-get"));
    }

    #[test]
    fn detects_return_get() {
        let s = "return sp.get();";
        let d = smart_ptr_checks(Path::new("t.cpp"), s);
        assert!(d.iter().any(|x| x.rule == "memory/raw-from-smart"));
    }
}
