use crate::linter::diagnostic::{Diagnostic, Severity};
use std::collections::HashSet;
use std::path::Path;

pub fn quick_dataflow_checks(file: &Path, source: &str) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    out.extend(must_free_analysis(file, source));
    out.extend(use_after_free_analysis(file, source));
    out
}

fn must_free_analysis(file: &Path, source: &str) -> Vec<Diagnostic> {
    let mut allocated: HashSet<(String, u32)> = HashSet::new();
    let mut freed: HashSet<String> = HashSet::new();
    for (idx, line) in source.lines().enumerate() {
        if let Some(v) = allocation_var(line) {
            allocated.insert((v, (idx + 1) as u32));
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
            message: format!("Allocated variable `{v}` may leak across exit paths"),
            file: file.to_path_buf(),
            line: *line,
            column: 1,
            span: None,
            suggestion: Some("Free/delete on all paths or use RAII wrappers".to_string()),
            fix: None,
            note: Some("Dataflow approximation over textual statements".to_string()),
        })
        .collect()
}

fn use_after_free_analysis(file: &Path, source: &str) -> Vec<Diagnostic> {
    let mut freed: HashSet<String> = HashSet::new();
    let mut out = Vec::new();
    for (idx, line) in source.lines().enumerate() {
        if let Some(v) = free_var(line) {
            freed.insert(v);
            continue;
        }
        for v in &freed {
            if line.contains(v)
                && !line.contains("free(")
                && !line.contains("delete ")
                && !line.trim_start().starts_with("//")
            {
                out.push(Diagnostic {
                    rule: "memory/use-after-free",
                    severity: Severity::Error,
                    message: format!("Variable `{v}` appears used after free/delete"),
                    file: file.to_path_buf(),
                    line: (idx + 1) as u32,
                    column: (line.find(v).unwrap_or(0) + 1) as u32,
                    span: Some((
                        (line.find(v).unwrap_or(0) + 1) as u32,
                        (line.find(v).unwrap_or(0) + 1 + v.len()) as u32,
                    )),
                    suggestion: Some("Do not access memory after release".to_string()),
                    fix: None,
                    note: None,
                });
            }
        }
    }
    out
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
