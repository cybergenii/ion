//! Textual dataflow checks for raw allocation/free patterns.
//!
//! Function-scoped analysis: allocation/free/use-after-free are tracked **per function body**
//! (heuristic split via brace matching). `memory/use-after-free` clears a freed variable when
//! a simple assignment to that identifier is seen (reallocation).

use crate::linter::diagnostic::{Diagnostic, Severity};
use std::collections::HashSet;
use std::path::Path;

pub fn quick_dataflow_checks(file: &Path, source: &str) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    let lines: Vec<&str> = source.lines().collect();
    if lines.is_empty() {
        return out;
    }
    let ranges = function_ranges(&lines);
    let ranges = if ranges.is_empty() {
        vec![(0, lines.len() - 1)]
    } else {
        ranges
    };
    for (start, end) in ranges {
        out.extend(must_free_in_range(file, &lines, start, end));
        out.extend(use_after_free_in_range(file, &lines, start, end));
    }
    out
}

/// Returns 0-based inclusive line indices `[start, end]` of each function body.
/// Brace depth is tracked from the function line; a range ends when depth returns to 0
/// (including single-line `void f() { }` bodies).
fn function_ranges(lines: &[&str]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < lines.len() {
        if !is_function_like_line(lines[i]) {
            i += 1;
            continue;
        }
        let start = i;
        let mut depth = 0i32;
        let mut j = start;
        loop {
            depth += lines[j].matches('{').count() as i32;
            depth -= lines[j].matches('}').count() as i32;
            if depth == 0 {
                out.push((start, j));
                i = j + 1;
                break;
            }
            j += 1;
            if j >= lines.len() {
                i = start + 1;
                break;
            }
        }
    }
    out
}

fn is_function_like_line(line: &str) -> bool {
    let t = line.trim_start();
    if t.starts_with("//") || t.starts_with('#') {
        return false;
    }
    let t = t.trim_start();
    if t.starts_with("if ")
        || t.starts_with("for ")
        || t.starts_with("while ")
        || t.starts_with("switch ")
        || t.starts_with("else")
        || t.starts_with("catch ")
        || t.starts_with("do ")
    {
        return false;
    }
    t.contains(')') && t.contains('{')
}

fn line_one_based(idx: usize) -> u32 {
    (idx + 1) as u32
}

fn must_free_in_range(
    file: &Path,
    lines: &[&str],
    start: usize,
    end: usize,
) -> Vec<Diagnostic> {
    let mut allocated: HashSet<(String, u32)> = HashSet::new();
    let mut freed: HashSet<String> = HashSet::new();
    for (idx, line) in lines.iter().enumerate().take(end + 1).skip(start) {
        let line = *line;
        if let Some(v) = allocation_var(line) {
            allocated.insert((v, line_one_based(idx)));
        }
        if let Some(v) = free_var(line) {
            freed.insert(v);
        }
    }
    allocated
        .iter()
        .filter(|(v, _)| !freed.contains(v))
        .map(|(v, line)| Diagnostic {
            rule: "memory/leak",
            severity: Severity::Warning,
            message: format!("Allocated variable `{v}` may leak (no matching free in this function)"),
            file: file.to_path_buf(),
            line: *line,
            column: 1,
            span: None,
            suggestion: Some("Free/delete on all paths or use RAII wrappers".to_string()),
            fix: None,
            note: Some("Function-scoped path-aware dataflow (textual)".to_string()),
        })
        .collect()
}

fn use_after_free_in_range(
    file: &Path,
    lines: &[&str],
    start: usize,
    end: usize,
) -> Vec<Diagnostic> {
    let mut freed: HashSet<String> = HashSet::new();
    let mut out = Vec::new();
    for (idx, line) in lines.iter().enumerate().take(end + 1).skip(start) {
        let line = *line;
        if line.trim_start().starts_with("//") {
            continue;
        }
        if let Some(lhs) = simple_assignment_lhs(line) {
            freed.remove(&lhs);
        }
        if let Some(v) = free_var(line) {
            freed.insert(v);
            continue;
        }
        for v in freed.clone() {
            if line.contains(&v)
                && !line.contains("free(")
                && !line.contains("delete ")
                && !line.trim_start().starts_with("//")
            {
                if is_only_assignment_to(line, &v) {
                    continue;
                }
                out.push(Diagnostic {
                    rule: "memory/use-after-free",
                    severity: Severity::Error,
                    message: format!("Variable `{v}` appears used after free/delete"),
                    file: file.to_path_buf(),
                    line: line_one_based(idx),
                    column: (line.find(v.as_str()).unwrap_or(0) + 1) as u32,
                    span: Some((
                        (line.find(v.as_str()).unwrap_or(0) + 1) as u32,
                        (line.find(v.as_str()).unwrap_or(0) + 1 + v.len()) as u32,
                    )),
                    suggestion: Some("Do not access memory after release".to_string()),
                    fix: None,
                    note: Some("Function-scoped path-aware dataflow (textual)".to_string()),
                });
            }
        }
    }
    out
}

/// `x = rhs` where `x` is a simple identifier (reassigns `x`, clears freed state).
fn simple_assignment_lhs(line: &str) -> Option<String> {
    let t = line.trim_start();
    if t.starts_with("//") {
        return None;
    }
    let eq = t.find('=')?;
    if t[..eq].contains("==") || t[..eq].contains("!=") || t[..eq].contains("<=") || t[..eq].contains(">=")
    {
        return None;
    }
    let lhs = t[..eq].trim();
    let lhs = lhs.split_whitespace().last()?;
    let name = lhs.trim();
    if name.is_empty() {
        return None;
    }
    let name = name.trim_matches(|c| c == '*' || c == '&');
    if name.is_empty() {
        return None;
    }
    if !name
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
    {
        return None;
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }
    Some(name.to_string())
}

fn is_only_assignment_to(line: &str, v: &str) -> bool {
    let t = line.trim();
    if let Some(lhs) = simple_assignment_lhs(line) {
        return lhs == v;
    }
    if let Some(rest) = t.strip_prefix(v) {
        return rest.trim_start().starts_with('=');
    }
    false
}

fn allocation_var(line: &str) -> Option<String> {
    if !(line.contains("malloc(")
        || line.contains("calloc(")
        || line.contains("realloc(")
        || line.contains("new "))
    {
        return None;
    }
    line.split('=')
        .next()
        .and_then(|lhs| lhs.split_whitespace().last())
        .map(ToOwned::to_owned)
}

fn free_var(line: &str) -> Option<String> {
    if let Some(start) = line.find("free(") {
        let s = &line[start + 5..];
        return s.split(')').next().map(|v| v.trim().to_string());
    }
    if let Some(start) = line.find("delete ") {
        let s = &line[start + 7..];
        return s.split(';').next().map(|v| v.trim().to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn uaf_not_cross_function() {
        let src = r#"
void a() { int* p = (int*)malloc(4); free(p); }
void b() { use(p); }
"#;
        let diags = quick_dataflow_checks(Path::new("t.cpp"), src);
        assert!(!diags.iter().any(|d| d.rule == "memory/use-after-free"));
    }

    #[test]
    fn uaf_cleared_by_reassignment() {
        let src = r#"
void f() {
  int* p = (int*)malloc(4);
  free(p);
  p = (int*)malloc(8);
  *p = 1;
}
"#;
        let diags = quick_dataflow_checks(Path::new("t.cpp"), src);
        assert!(!diags.iter().any(|d| d.rule == "memory/use-after-free"));
    }

    #[test]
    fn leak_detected_in_function() {
        let src = r#"
void f() {
  int* p = (int*)malloc(4);
}
"#;
        let diags = quick_dataflow_checks(Path::new("t.cpp"), src);
        assert!(diags.iter().any(|d| d.rule == "memory/leak"));
    }
}
