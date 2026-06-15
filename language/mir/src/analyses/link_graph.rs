use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::{Analysis, AnalysisId, CallGraph, ModuleAnalyses, ModuleAnalysis};
use crate::{
    Function, FunctionBehavior, Global, GlobalInitializer, Instruction, Linkage, MemoryEffect,
    Symbol, Tree,
};

/// How one symbol references another.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LinkEdgeKind {
    /// The source calls the target.
    Call,
    /// The source takes the target's address.
    Address,
}

/// One outgoing reference from a symbol to another symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LinkEdge {
    /// The referenced symbol.
    pub target: Symbol,
    /// How the target is referenced.
    pub kind: LinkEdgeKind,
}

/// One defined symbol in the link graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LinkNode {
    /// A defined function.
    Function {
        /// Visibility and definition location.
        linkage: Linkage,
        /// Memory effect.
        memory: MemoryEffect,
        /// Behavioral effects (unwind, determinism, allocation, and so on).
        behavior: FunctionBehavior,
        /// Inline cost approximation.
        inline_cost: u32,
        /// True when the function makes indirect or virtual calls.
        indirect: bool,
    },
    /// A defined global.
    Global {
        /// Visibility and definition location.
        linkage: Linkage,
    },
}

impl LinkNode {
    /// Return the symbol's linkage.
    pub fn linkage(&self) -> Linkage {
        match self {
            Self::Function { linkage, .. } | Self::Global { linkage } => *linkage,
        }
    }
}

/// Symbol-scoped reference graph for a module's linkable surface.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LinkGraph {
    /// Defined symbols, by identity.
    nodes: HashMap<Symbol, LinkNode>,
    /// Outgoing references, by source symbol.
    edges: HashMap<Symbol, Vec<LinkEdge>>,
}

impl LinkGraph {
    /// Create an empty link graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert one defined symbol's node.
    pub fn insert(&mut self, symbol: Symbol, node: LinkNode) {
        self.nodes.insert(symbol, node);
    }

    /// Record one outgoing reference from a source symbol.
    pub fn add_edge(&mut self, source: Symbol, edge: LinkEdge) {
        self.edges.entry(source).or_default().push(edge);
    }

    /// Look up one symbol's node.
    pub fn node(&self, symbol: Symbol) -> Option<&LinkNode> {
        self.nodes.get(&symbol)
    }

    /// Iterate over all defined symbol nodes with their identities.
    pub fn nodes(&self) -> impl Iterator<Item = (Symbol, &LinkNode)> {
        self.nodes.iter().map(|(symbol, node)| (*symbol, node))
    }

    /// Return the outgoing references from one source symbol.
    pub fn edges(&self, source: Symbol) -> &[LinkEdge] {
        self.edges.get(&source).map(Vec::as_slice).unwrap_or(&[])
    }

    /// Absorb another module's nodes and edges into this graph.
    ///
    /// Each symbol is defined in exactly one module, so per-module graphs have
    /// disjoint sources and merging is a plain union.
    pub fn merge(&mut self, other: &LinkGraph) {
        for (symbol, node) in &other.nodes {
            self.nodes.insert(*symbol, node.clone());
        }
        for (source, edges) in &other.edges {
            self.edges
                .entry(*source)
                .or_default()
                .extend(edges.iter().copied());
        }
    }
}

impl Analysis for LinkGraph {
    const ID: AnalysisId = AnalysisId("link-graph");
}

impl ModuleAnalysis for LinkGraph {
    /// Build the link graph for one module from its call graph and tree.
    fn compute(tree: &Tree, analyses: &ModuleAnalyses) -> Self {
        let call_graph = analyses.get::<CallGraph>(tree);
        let mut graph = LinkGraph::new();

        // record each defined function with its attributes and outgoing references
        for (function_id, function) in tree.iter_nodes::<Function>() {
            // skip declarations without a body
            if function.entry.is_none() {
                continue;
            }

            let symbol = function.symbol;
            let metadata = tree.metadata.functions.function(function_id);
            let memory = metadata.map(|m| m.memory.clone()).unwrap_or_default();
            let behavior = metadata.map(|m| m.behavior.clone()).unwrap_or_default();
            graph.insert(
                symbol,
                LinkNode::Function {
                    linkage: function.linkage,
                    memory,
                    behavior,
                    inline_cost: function_inline_cost(function, tree),
                    indirect: !call_graph.unknown_calls(function_id).is_empty(),
                },
            );

            // call edges, lowered from tree-local callees to persistent symbols
            for edge in call_graph.outgoing(function_id) {
                let target = tree.get(edge.callee).symbol;
                graph.add_edge(
                    symbol,
                    LinkEdge {
                        target,
                        kind: LinkEdgeKind::Call,
                    },
                );
            }

            // address-of edges from instruction operands
            for &block_id in &function.blocks {
                let block = tree.get(block_id);
                for &instruction_id in &block.instructions {
                    if let Some(target) = address_target(tree.get(instruction_id), tree) {
                        graph.add_edge(
                            symbol,
                            LinkEdge {
                                target,
                                kind: LinkEdgeKind::Address,
                            },
                        );
                    }
                }
            }
        }

        // record each global with its address-of references from its initializer
        for (_global_id, global) in tree.iter_nodes::<Global>() {
            let symbol = global.symbol;
            graph.insert(
                symbol,
                LinkNode::Global {
                    linkage: global.linkage,
                },
            );
            if let Some(initializer) = &global.initializer {
                collect_initializer_addresses(initializer, tree, symbol, &mut graph);
            }
        }

        graph
    }
}

/// Approximate a function's inline cost as its instruction count.
fn function_inline_cost(function: &Function, tree: &Tree) -> u32 {
    let mut count = 0usize;
    for &block_id in &function.blocks {
        count += tree.get(block_id).instructions.len();
    }
    count.min(u32::MAX as usize) as u32
}

/// Return the symbol whose address one instruction takes, if any.
fn address_target(instruction: &Instruction, tree: &Tree) -> Option<Symbol> {
    match instruction {
        Instruction::FunctionAddr { function, .. } | Instruction::ClosureBind { function, .. } => {
            function.function().map(|id| tree.get(id).symbol)
        }
        Instruction::GlobalAddr { global, .. } => global.global().map(|id| tree.get(id).symbol),
        _ => None,
    }
}

/// Record address-of edges from a global initializer, recursing into aggregates.
fn collect_initializer_addresses(
    initializer: &GlobalInitializer,
    tree: &Tree,
    source: Symbol,
    graph: &mut LinkGraph,
) {
    match initializer {
        GlobalInitializer::FunctionAddress(function) => {
            if let Some(id) = function.function() {
                graph.add_edge(
                    source,
                    LinkEdge {
                        target: tree.get(id).symbol,
                        kind: LinkEdgeKind::Address,
                    },
                );
            }
        }
        GlobalInitializer::Aggregate(elements) => {
            for element in elements {
                collect_initializer_addresses(element, tree, source, graph);
            }
        }
        GlobalInitializer::Zero | GlobalInitializer::Scalar(_) | GlobalInitializer::Bytes(_) => {}
    }
}
