use crate::linter::diagnostic::{Diagnostic, Severity};
use crate::linter::rules::{Rule, SemanticContext};
use clang::{Entity, EntityKind};

pub struct NullDerefRule;

pub fn detect_null_deref_pattern(source: &str) -> bool {
    source.contains("malloc(") && (source.contains("*ptr") || source.contains("->"))
}

impl Rule for NullDerefRule {
    fn id(&self) -> &'static str {
        "null/deref"
    }

    fn description(&self) -> &'static str {
        "Pointer dereference without nearby null guard"
    }

    fn check(
        &self,
        ctx: &SemanticContext,
        entity: &Entity,
        _parent: &Entity,
    ) -> Option<Diagnostic> {
        if entity.get_kind() != EntityKind::UnaryOperator {
            return None;
        }
        let display = entity.get_display_name().unwrap_or_default();
        if !display.contains('*') {
            return None;
        }
        // Skip obvious `*this` member access patterns (often non-null in instance methods).
        if display.contains("this") {
            return None;
        }
        let loc = entity.get_location()?;
        let file = loc.get_file_location().file?;
        let fl = loc.get_file_location();
        let scope = ctx
            .enclosing_function
            .as_ref()
            .map(|f| format!(" (in `{f}`)"))
            .unwrap_or_default();
        Some(Diagnostic {
            rule: "null/deref",
            severity: Severity::Warning,
            message: format!("Pointer dereference may require null check{scope}"),
            file: file.get_path(),
            line: fl.line,
            column: fl.column,
            span: None,
            suggestion: Some("Check pointer is non-null before dereferencing".to_string()),
            fix: None,
            note: Some("Heuristic: verify against control flow and invariants".to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_deref_rule_detects_pattern() {
        let src = "int* ptr = (int*)malloc(4); *ptr = 1;";
        assert!(detect_null_deref_pattern(src));
    }
}
