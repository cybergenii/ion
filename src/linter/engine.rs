use crate::linter::diagnostic::Diagnostic;
use crate::linter::rules::RuleSet;
use anyhow::Result;
use clang::{Clang, Entity, Index};
use std::path::Path;

pub struct LintEngine {
    enabled: bool,
    pub rules: RuleSet,
}

impl LintEngine {
    pub fn new() -> Self {
        let enabled = Clang::new().is_ok();
        Self {
            enabled,
            rules: RuleSet::default(),
        }
    }

    pub fn semantic_available(&self) -> bool {
        self.enabled
    }

    pub fn analyze_file(&self, file: &Path, filter_rule: Option<&str>) -> Result<Vec<Diagnostic>> {
        if !self.enabled {
            return Ok(Vec::new());
        }
        let clang = Clang::new()?;
        let index = Index::new(&clang, false, false);
        let tu = index
            .parser(file)
            .arguments(&["-x", "c++", "-std=c++20"])
            .parse()?;
        let root = tu.get_entity();
        let mut diagnostics = Vec::new();
        self.walk(&root, &root, &mut diagnostics, filter_rule);
        Ok(diagnostics)
    }

    fn walk(
        &self,
        entity: &Entity,
        parent: &Entity,
        diagnostics: &mut Vec<Diagnostic>,
        filter_rule: Option<&str>,
    ) {
        for rule in &self.rules.rules {
            if filter_rule.is_none() || filter_rule == Some(rule.id()) {
                if let Some(diag) = rule.check(entity, parent) {
                    diagnostics.push(diag);
                }
            }
        }
        for child in entity.get_children() {
            self.walk(&child, entity, diagnostics, filter_rule);
        }
    }
}
