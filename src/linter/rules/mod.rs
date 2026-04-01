use crate::linter::diagnostic::Diagnostic;
use clang::Entity;
use std::path::Path;

pub mod memory;
pub mod modern;
pub mod null;
pub mod resource;

/// Per-translation-unit context passed to semantic rules (libclang AST walk).
#[derive(Clone, Debug)]
pub struct SemanticContext<'a> {
    pub file: &'a Path,
    pub source: &'a str,
    /// Innermost enclosing function/method/ctor/dtor name, if known.
    pub enclosing_function: Option<String>,
}

pub trait Rule: Send + Sync {
    fn id(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn check(&self, ctx: &SemanticContext, entity: &Entity, parent: &Entity) -> Option<Diagnostic>;
}

pub struct RuleSet {
    pub rules: Vec<Box<dyn Rule>>,
}

/// All rule ids that `ion check --rule` may filter on (semantic, dataflow, and modern checks).
pub const KNOWN_RULE_IDS: &[&str] = &[
    "memory/double-free",
    "memory/leak",
    "memory/move-after-use",
    "memory/raw-from-smart",
    "memory/shared-cycle-hint",
    "memory/smart-get",
    "memory/use-after-free",
    "modern/c-cast",
    "modern/emplace-back",
    "modern/nullptr",
    "modern/printf",
    "modern/range-for",
    "null/deref",
    "resource/leak",
];

pub fn is_known_rule_id(id: &str) -> bool {
    KNOWN_RULE_IDS.binary_search(&id).is_ok()
}

pub fn describe_rule(id: &str) -> &'static str {
    match id {
        "memory/leak" => "Raw heap allocation that may not be released on all paths",
        "memory/double-free" => "Repeated release call on the same allocation",
        "memory/use-after-free" => "Variable appears used after memory was released",
        "memory/smart-get" => "Storing or exposing `.get()` from a smart pointer can outlive the owner",
        "memory/raw-from-smart" => "Raw pointer taken from a smart pointer; ensure lifetime is bounded",
        "memory/move-after-use" => "Variable used after `std::move` may be in a moved-from state",
        "memory/shared-cycle-hint" => "Multiple `shared_ptr` members may form reference cycles; consider `weak_ptr`",
        "null/deref" => "Pointer dereference without a preceding null check",
        "resource/leak" => "Resource opened but not clearly closed",
        "modern/nullptr" => "Prefer `nullptr` over legacy `NULL`",
        "modern/c-cast" => "Prefer C++ casts over C-style casts",
        "modern/printf" => "Prefer modern C++ formatting APIs",
        "modern/range-for" => "Prefer range-based for loops where possible",
        "modern/emplace-back" => "Prefer `emplace_back` for in-place construction",
        _ => "Ion lint rule",
    }
}

impl Default for RuleSet {
    fn default() -> Self {
        Self {
            rules: vec![
                Box::new(memory::MemoryLeakRule),
                Box::new(memory::DoubleFreeRule),
                Box::new(null::NullDerefRule),
                Box::new(resource::ResourceLeakRule),
            ],
        }
    }
}
