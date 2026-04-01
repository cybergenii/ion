use crate::linter::diagnostic::{Diagnostic, Severity};
use crate::linter::rules::{Rule, SemanticContext};
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

    fn check(&self, ctx: &SemanticContext, entity: &Entity, _parent: &Entity) -> Option<Diagnostic> {
        if entity.get_kind() != EntityKind::CallExpr {
            return None;
        }
        let display = entity.get_display_name()?;
        if !(display.contains("fopen") || display.contains("open") || display.contains("socket")) {
            return None;
        }
        // Skip obvious C++ std::fstream / path helpers (heuristic).
        if display.contains("std::") || display.contains("filesystem::") {
            return None;
        }
        let loc = entity.get_location()?;
        let file = loc.get_file_location().file?;
        let fl = loc.get_file_location();
        let scope = ctx
            .enclosing_function
            .as_ref()
            .map(|f| format!(" (function `{f}`)"))
            .unwrap_or_default();
        Some(Diagnostic {
            rule: "resource/leak",
            severity: Severity::Warning,
            message: format!("Resource acquisition via `{display}` may leak{scope}"),
            file: file.get_path(),
            line: fl.line,
            column: fl.column,
            span: None,
            suggestion: Some(
                "Wrap in RAII guard or ensure `fclose`/`close` on all paths".to_string(),
            ),
            fix: None,
            note: Some("Heuristic: verify pairing with fclose/close on all paths".to_string()),
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
