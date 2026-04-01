use crate::linter::diagnostic::{Diagnostic, Severity};
use crate::linter::rules::Rule;
use clang::{Entity, EntityKind};

pub struct ResourceLeakRule;

pub fn detect_resource_open(line: &str) -> bool {
    line.contains("fopen(") || line.contains("open(") || line.contains("socket(")
}

impl Rule for ResourceLeakRule {
    fn id(&self) -> &'static str {
        "resource/leak"
    }

    fn description(&self) -> &'static str {
        "Resource acquisition without guaranteed close/release"
    }

    fn check(&self, entity: &Entity, _parent: &Entity) -> Option<Diagnostic> {
        if entity.get_kind() != EntityKind::CallExpr {
            return None;
        }
        let display = entity.get_display_name()?;
        if !(display.contains("fopen") || display.contains("open") || display.contains("socket")) {
            return None;
        }
        let loc = entity.get_location()?;
        let file = loc.get_file_location().file?;
        Some(Diagnostic {
            rule: "resource/leak",
            severity: Severity::Warning,
            message: format!("Resource acquisition via `{display}` may leak"),
            file: file.get_path(),
            line: loc.get_file_location().line,
            column: loc.get_file_location().column,
            span: None,
            suggestion: Some(
                "Wrap in RAII guard or ensure `fclose`/`close` on all paths".to_string(),
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
    fn resource_leak_rule_detects_open_pattern() {
        assert!(detect_resource_open("FILE* f = fopen(\"a.txt\", \"r\");"));
    }
}
