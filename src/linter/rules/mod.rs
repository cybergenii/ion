use crate::linter::diagnostic::Diagnostic;
use clang::Entity;

pub mod memory;
pub mod modern;
pub mod null;
pub mod resource;

pub trait Rule: Send + Sync {
    fn id(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn check(&self, entity: &Entity, parent: &Entity) -> Option<Diagnostic>;
}

pub struct RuleSet {
    pub rules: Vec<Box<dyn Rule>>,
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
