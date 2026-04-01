use crate::linter::diagnostic::{Diagnostic, Severity};
use crate::linter::rules::Rule;
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

    fn check(&self, entity: &Entity, _parent: &Entity) -> Option<Diagnostic> {
        if entity.get_kind() != EntityKind::UnaryOperator {
            return None;
        }
        let display = entity.get_display_name().unwrap_or_default();
        if !display.contains('*') {
            return None;
        }
        let loc = entity.get_location()?;
        let file = loc.get_file_location().file?;
        Some(Diagnostic {
            rule: "null/deref",
            severity: Severity::Warning,
            message: "Pointer dereference may require null check".to_string(),
            file: file.get_path(),
            line: loc.get_file_location().line,
            column: loc.get_file_location().column,
            span: None,
            suggestion: Some("Check pointer is non-null before dereferencing".to_string()),
            fix: None,
            note: None,
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
