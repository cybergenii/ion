use crate::linter::diagnostic::Diagnostic;
use crate::linter::rules::{RuleSet, SemanticContext};
use anyhow::Result;
use clang::{Clang, Entity, EntityKind, Index, Unsaved};
use serde::Deserialize;
use std::fs;
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

    pub fn analyze_file(
        &self,
        file: &Path,
        filter_rules: Option<&[String]>,
    ) -> Result<Vec<Diagnostic>> {
        let source = fs::read_to_string(file)?;
        self.analyze_file_with_source(file, &source, filter_rules)
    }

    /// Parse `file` with optional in-memory buffer (for LSP unsaved documents).
    pub fn analyze_file_with_source(
        &self,
        file: &Path,
        source: &str,
        filter_rules: Option<&[String]>,
    ) -> Result<Vec<Diagnostic>> {
        if !self.enabled {
            return Ok(Vec::new());
        }
        let clang = Clang::new().map_err(|e| anyhow::anyhow!(e))?;
        let index = Index::new(&clang, false, false);
        let mut parser = index.parser(file);
        let args = compile_args_for(file);
        if args.is_empty() {
            parser.arguments(&["-x", "c++", "-std=c++20"]);
        } else {
            let refs = args.iter().map(String::as_str).collect::<Vec<_>>();
            parser.arguments(&refs);
        }
        parser.unsaved(&[Unsaved::new(file, source)]);
        let tu = parser.parse()?;
        let root = tu.get_entity();
        let mut diagnostics = Vec::new();
        let ctx = SemanticContext {
            file,
            source,
            enclosing_function: None,
        };
        self.walk(&root, &root, &mut diagnostics, filter_rules, &ctx);
        Ok(diagnostics)
    }

    fn walk(
        &self,
        entity: &Entity,
        parent: &Entity,
        diagnostics: &mut Vec<Diagnostic>,
        filter_rules: Option<&[String]>,
        ctx: &SemanticContext,
    ) {
        let ctx_here = if is_function_like(entity.get_kind()) {
            SemanticContext {
                file: ctx.file,
                source: ctx.source,
                enclosing_function: entity.get_name().or_else(|| entity.get_display_name()),
            }
        } else {
            SemanticContext {
                file: ctx.file,
                source: ctx.source,
                enclosing_function: ctx.enclosing_function.clone(),
            }
        };

        for rule in &self.rules.rules {
            let run = match filter_rules {
                None => true,
                Some(ids) => ids.iter().any(|r| r == rule.id()),
            };
            if run {
                if let Some(diag) = rule.check(&ctx_here, entity, parent) {
                    diagnostics.push(diag);
                }
            }
        }
        for child in entity.get_children() {
            self.walk(&child, entity, diagnostics, filter_rules, &ctx_here);
        }
    }
}

fn is_function_like(kind: EntityKind) -> bool {
    matches!(
        kind,
        EntityKind::FunctionDecl
            | EntityKind::Method
            | EntityKind::Constructor
            | EntityKind::Destructor
            | EntityKind::ConversionFunction
    )
}

#[derive(Debug, Deserialize)]
struct CompileCommand {
    file: String,
    arguments: Option<Vec<String>>,
    command: Option<String>,
}

pub(crate) fn compile_args_for(file: &Path) -> Vec<String> {
    let cc_path = Path::new("compile_commands.json");
    if !cc_path.exists() {
        return Vec::new();
    }
    let content = match fs::read_to_string(cc_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let entries: Vec<CompileCommand> = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let file_s = file.to_string_lossy();
    let cmd = match entries.iter().find(|e| e.file.ends_with(file_s.as_ref())) {
        Some(c) => c,
        None => return Vec::new(),
    };
    if let Some(args) = &cmd.arguments {
        return normalize_args(args);
    }
    if let Some(command) = &cmd.command {
        return normalize_args(
            &command
                .split_whitespace()
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>(),
        );
    }
    Vec::new()
}

fn normalize_args(args: &[String]) -> Vec<String> {
    args.iter()
        .filter(|a| {
            !a.starts_with("clang")
                && !a.starts_with("g++")
                && !a.starts_with("c++")
                && !a.ends_with(".cpp")
                && !a.ends_with(".cc")
                && !a.ends_with(".cxx")
        })
        .cloned()
        .collect()
}
