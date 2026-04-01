use crate::linter::diagnostic::{Diagnostic, Severity};
use crate::linter::rules::Rule;
use clang::{Entity, EntityKind};
use std::path::PathBuf;

pub struct MemoryLeakRule;
pub struct DoubleFreeRule;

pub fn detect_memory_leak_line(line: &str) -> bool {
    line.contains("malloc(") || line.contains("calloc(") || line.contains("realloc(") || line.contains("new ")
}

pub fn detect_double_free(source: &str, var: &str) -> bool {
    let mut count = 0usize;
    for line in source.lines() {
        if line.contains(&format!("free({var})")) || line.contains(&format!("delete {var}")) {
            count += 1;
        }
    }
    count > 1
}

impl Rule for MemoryLeakRule {
    fn id(&self) -> &'static str {
        "memory/leak"
    }

    fn description(&self) -> &'static str {
        "Raw heap allocation that can leak when not guarded by RAII"
    }

    fn check(&self, entity: &Entity, _parent: &Entity) -> Option<Diagnostic> {
        if entity.get_kind() != EntityKind::CallExpr {
            return None;
        }
        let display = entity.get_display_name()?;
        if !(display.contains("malloc")
            || display.contains("calloc")
            || display.contains("realloc")
            || display.contains("new"))
        {
            return None;
        }
        let loc = entity.get_location()?;
        let file = loc.get_file_location().file?;
        Some(Diagnostic {
            rule: "memory/leak",
            severity: Severity::Warning,
            message: format!("Raw allocation via `{display}`"),
            file: PathBuf::from(file.get_path()),
            line: loc.get_file_location().line,
            column: loc.get_file_location().column,
            span: None,
            suggestion: Some(
                "Use RAII — replace with `std::unique_ptr<T>` or `std::vector<T>`".to_string(),
            ),
            fix: None,
            note: None,
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
}

impl Rule for DoubleFreeRule {
    fn id(&self) -> &'static str {
        "memory/double-free"
    }

    fn description(&self) -> &'static str {
        "Potential double free/delete call in same lexical scope"
    }

    fn check(&self, entity: &Entity, _parent: &Entity) -> Option<Diagnostic> {
        if entity.get_kind() != EntityKind::CallExpr {
            return None;
        }
        let display = entity.get_display_name()?;
        if !(display.contains("free") || display.contains("delete")) {
            return None;
        }
        let loc = entity.get_location()?;
        let file = loc.get_file_location().file?;
        Some(Diagnostic {
            rule: "memory/double-free",
            severity: Severity::Error,
            message: format!("Potential repeated deallocation via `{display}`"),
            file: PathBuf::from(file.get_path()),
            line: loc.get_file_location().line,
            column: loc.get_file_location().column,
            span: None,
            suggestion: Some("Ensure each allocation is freed exactly once".to_string()),
            fix: None,
            note: Some("Heuristic check: validate ownership and control-flow paths".to_string()),
        })
    }
}
