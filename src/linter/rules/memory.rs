use crate::linter::diagnostic::{Diagnostic, Severity};
use crate::linter::rules::{Rule, SemanticContext};
use clang::{Entity, EntityKind};

pub struct MemoryLeakRule;
pub struct DoubleFreeRule;

pub fn detect_memory_leak_line(line: &str) -> bool {
    line.contains("malloc(") || line.contains("calloc(") || line.contains("realloc(") || line.contains("new ")
}

pub fn detect_double_free(source: &str, var: &str) -> bool {
    let mut count = 0usize;
    let free_pat = format!("free({var})");
    let del_pat = format!("delete {var}");
    for line in source.lines() {
        count += line.matches(&free_pat).count();
        count += line.matches(&del_pat).count();
    }
    count > 1
}

/// Extract identifier passed to `free(...)` from clang display name when possible.
pub fn variable_from_free_display(display: &str) -> Option<String> {
    let d = display.trim();
    if let Some(start) = d.find("free(") {
        let s = &d[start + 5..];
        return s.split(')').next().map(|v| v.trim().to_string());
    }
    if let Some(start) = d.find("delete ") {
        let s = &d[start + 7..];
        return s.split(';').next().map(|v| v.trim().to_string());
    }
    None
}

impl Rule for MemoryLeakRule {
    fn id(&self) -> &'static str {
        "memory/leak"
    }

    fn description(&self) -> &'static str {
        "Raw heap allocation that can leak when not guarded by RAII"
    }

    fn check(&self, ctx: &SemanticContext, entity: &Entity, _parent: &Entity) -> Option<Diagnostic> {
        if entity.get_kind() != EntityKind::CallExpr {
            return None;
        }
        let display = entity.get_display_name()?;
        if display.contains("make_unique")
            || display.contains("make_shared")
            || display.contains("allocate")
        {
            return None;
        }
        if !(display.contains("malloc")
            || display.contains("calloc")
            || display.contains("realloc")
            || display.contains("new"))
        {
            return None;
        }
        let loc = entity.get_location()?;
        let file = loc.get_file_location().file?;
        let fl = loc.get_file_location();
        let scope_note = ctx
            .enclosing_function
            .as_ref()
            .map(|f| format!("In function `{f}`: "))
            .unwrap_or_default();
        Some(Diagnostic {
            rule: "memory/leak",
            severity: Severity::Warning,
            message: format!("{scope_note}Raw allocation via `{display}`"),
            file: file.get_path(),
            line: fl.line,
            column: fl.column,
            span: None,
            suggestion: Some(
                "Use RAII — replace with `std::unique_ptr<T>` or `std::vector<T>`".to_string(),
            ),
            fix: None,
            note: Some("Heuristic: verify all paths release or transfer ownership".to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_leak_rule_detects_malloc_pattern() {
        assert!(detect_memory_leak_line("auto p = malloc(42);"));
    }

    #[test]
    fn double_free_rule_detects_repeat_free() {
        let src = "int* p = (int*)malloc(4); free(p); free(p);";
        assert!(detect_double_free(src, "p"));
    }

    #[test]
    fn variable_from_free_display_parses() {
        assert_eq!(
            variable_from_free_display("free(p)").as_deref(),
            Some("p")
        );
    }
}

impl Rule for DoubleFreeRule {
    fn id(&self) -> &'static str {
        "memory/double-free"
    }

    fn description(&self) -> &'static str {
        "Repeated free/delete on the same variable in this translation unit"
    }

    fn check(&self, ctx: &SemanticContext, entity: &Entity, _parent: &Entity) -> Option<Diagnostic> {
        if entity.get_kind() != EntityKind::CallExpr {
            return None;
        }
        let display = entity.get_display_name()?;
        if !(display.contains("free") || display.contains("delete")) {
            return None;
        }
        let var = variable_from_free_display(&display)?;
        if !detect_double_free(ctx.source, &var) {
            return None;
        }
        let loc = entity.get_location()?;
        let file = loc.get_file_location().file?;
        let fl = loc.get_file_location();
        Some(Diagnostic {
            rule: "memory/double-free",
            severity: Severity::Error,
            message: format!("Repeated deallocation of `{var}` via `{display}`"),
            file: file.get_path(),
            line: fl.line,
            column: fl.column,
            span: None,
            suggestion: Some("Ensure each allocation is freed exactly once".to_string()),
            fix: None,
            note: Some("Matched multiple free/delete sites for the same variable in this file".to_string()),
        })
    }
}
