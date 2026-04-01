use clang::Entity;
use petgraph::graph::NodeIndex;
use petgraph::graph::DiGraph;
use std::collections::HashSet;

#[derive(Clone, Debug)]
pub struct BasicBlock {
    pub id: NodeIndex,
    pub stmts: Vec<String>,
    pub vars_defined: HashSet<String>,
    pub vars_used: HashSet<String>,
    pub vars_allocated: HashSet<String>,
    pub vars_freed: HashSet<String>,
}

pub struct ControlFlowGraph {
    pub graph: DiGraph<BasicBlock, EdgeKind>,
    pub entry: NodeIndex,
    pub exits: Vec<NodeIndex>,
}

pub enum EdgeKind {
    Unconditional,
    True,
    False,
}

impl ControlFlowGraph {
    pub fn from_function(_func: &Entity) -> Self {
        let mut graph = DiGraph::<BasicBlock, EdgeKind>::new();
        let entry = graph.add_node(BasicBlock {
            id: NodeIndex::new(0),
            stmts: Vec::new(),
            vars_defined: HashSet::new(),
            vars_used: HashSet::new(),
            vars_allocated: HashSet::new(),
            vars_freed: HashSet::new(),
        });
        let exit = graph.add_node(BasicBlock {
            id: NodeIndex::new(1),
            stmts: Vec::new(),
            vars_defined: HashSet::new(),
            vars_used: HashSet::new(),
            vars_allocated: HashSet::new(),
            vars_freed: HashSet::new(),
        });
        graph.add_edge(entry, exit, EdgeKind::Unconditional);
        Self {
            graph,
            entry,
            exits: vec![exit],
        }
    }
}
