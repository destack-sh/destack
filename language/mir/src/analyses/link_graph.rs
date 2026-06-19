use std::collections::HashMap;

use destack_core::{BitSet, DenseGraph};
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
}

/// Strongly connected components of the whole-program call graph, by dense symbol id.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CallComponentGraph {
    /// The component id of each symbol, by dense id.
    component: Vec<u32>,
    /// Whether each component contains a cycle, by component id.
    recursive: BitSet,
}

impl CallComponentGraph {
    /// Return the component id of a symbol by dense id.
    pub fn component(&self, symbol: usize) -> u32 {
        self.component[symbol]
    }

    /// Return whether a symbol's component contains a cycle, by dense id.
    pub fn is_recursive(&self, symbol: usize) -> bool {
        self.recursive.contains(self.component[symbol] as usize)
    }

    /// Return the number of components.
    pub fn len(&self) -> usize {
        self.recursive.len()
    }

    /// Return whether there are no components.
    pub fn is_empty(&self) -> bool {
        self.recursive.is_empty()
    }
}

/// Whole-program reference graph in compressed sparse row form over a dense symbol index.
#[derive(Debug, Default)]
pub struct LinkSupergraph {
    /// Every defined symbol, sorted so each position is its dense id.
    symbols: Vec<Symbol>,
    /// Per-source start offset into `edge_targets` (length is symbols + 1).
    edge_offsets: Vec<u32>,
    /// Edge heads as dense ids, grouped by source.
    edge_targets: Vec<u32>,
    /// Reference kind parallel to `edge_targets`.
    edge_kinds: Vec<LinkEdgeKind>,
}

impl LinkSupergraph {
    /// Stitch the dense cross-module graph from per-module link graphs.
    pub fn build<'a>(members: impl IntoIterator<Item = &'a LinkGraph>) -> Self {
        let members: Vec<&LinkGraph> = members.into_iter().collect();

        // collect every defined symbol, sorted to assign dense ids
        let mut symbols: Vec<Symbol> = members
            .iter()
            .flat_map(|graph| graph.nodes().map(|(symbol, _)| symbol))
            .collect();
        symbols.sort_unstable();

        // index each symbol to its dense id for edge translation
        let index: HashMap<Symbol, u32> = symbols
            .iter()
            .enumerate()
            .map(|(dense, symbol)| (*symbol, dense as u32))
            .collect();

        // count each source's edges that land on a defined symbol
        let mut edge_offsets = vec![0u32; symbols.len() + 1];
        for graph in &members {
            for (symbol, _) in graph.nodes() {
                let source = index[&symbol] as usize;
                for edge in graph.edges(symbol) {
                    if index.contains_key(&edge.target) {
                        edge_offsets[source + 1] += 1;
                    }
                }
            }
        }

        // prefix sum the counts into compressed sparse row start offsets
        for source in 0..symbols.len() {
            edge_offsets[source + 1] += edge_offsets[source];
        }

        // fill each source's edges at its advancing cursor
        let edge_count = edge_offsets[symbols.len()] as usize;
        let mut edge_targets = vec![0u32; edge_count];
        let mut edge_kinds = vec![LinkEdgeKind::Call; edge_count];
        let mut cursor: Vec<u32> = edge_offsets[..symbols.len()].to_vec();
        for graph in &members {
            for (symbol, _) in graph.nodes() {
                let source = index[&symbol] as usize;
                for edge in graph.edges(symbol) {
                    if let Some(&target) = index.get(&edge.target) {
                        let at = cursor[source] as usize;
                        edge_targets[at] = target;
                        edge_kinds[at] = edge.kind;
                        cursor[source] += 1;
                    }
                }
            }
        }

        Self {
            symbols,
            edge_offsets,
            edge_targets,
            edge_kinds,
        }
    }

    /// Return the program's defined symbols in dense-id order.
    pub fn symbols(&self) -> &[Symbol] {
        &self.symbols
    }

    /// Return the dense id of a symbol, if the program defines it.
    pub fn index_of(&self, symbol: Symbol) -> Option<usize> {
        self.symbols.binary_search(&symbol).ok()
    }

    /// Mark every symbol reachable from the roots over call and address edges.
    pub fn reachable(&self, roots: &[Symbol]) -> BitSet {
        let mut live = BitSet::new(self.symbols.len());
        let mut worklist: Vec<usize> = Vec::new();

        // seed the worklist with the defined roots
        for &root in roots {
            if let Some(dense) = self.index_of(root)
                && live.insert(dense)
            {
                worklist.push(dense);
            }
        }

        // walk outgoing edges until no new symbol is discovered
        while let Some(source) = worklist.pop() {
            for &target in self.successors(source) {
                if live.insert(target as usize) {
                    worklist.push(target as usize);
                }
            }
        }

        live
    }

    /// Count program-wide incoming references to each symbol, by dense id.
    pub fn reference_counts(&self) -> Vec<u32> {
        let mut counts = vec![0u32; self.symbols.len()];
        for &target in &self.edge_targets {
            counts[target as usize] += 1;
        }

        counts
    }

    /// Mark every symbol whose address is taken anywhere in the program.
    pub fn address_taken(&self) -> BitSet {
        let mut taken = BitSet::new(self.symbols.len());
        for (position, kind) in self.edge_kinds.iter().enumerate() {
            if *kind == LinkEdgeKind::Address {
                taken.insert(self.edge_targets[position] as usize);
            }
        }

        taken
    }

    /// Mark every defined symbol internal to the program (not an external root).
    pub fn internal(&self, roots: &[Symbol]) -> BitSet {
        // collect the dense ids of the program's external roots
        let mut is_root = BitSet::new(self.symbols.len());
        for &root in roots {
            if let Some(dense) = self.index_of(root) {
                is_root.insert(dense);
            }
        }

        // every symbol the external surface does not name is internal
        let mut internal = BitSet::new(self.symbols.len());
        for dense in 0..self.symbols.len() {
            if !is_root.contains(dense) {
                internal.insert(dense);
            }
        }

        internal
    }

    /// Condense the call graph into strongly connected components in bottom-up order.
    pub fn call_components(&self) -> CallComponentGraph {
        let count = self.symbols.len();

        // build the call-only graph used for recursion analysis
        let (call_offsets, call_targets) = self.call_edges();
        let graph = DenseGraph::new(&call_offsets, &call_targets);
        let partition = graph.strongly_connected_components();

        // count component sizes to distinguish cycles from singletons
        let component_count = partition.component_count() as usize;
        let mut component_sizes = vec![0u32; component_count];
        for &component in partition.components() {
            component_sizes[component as usize] += 1;
        }

        // mark components that contain a cycle or a direct self-call
        let mut recursive_bits = BitSet::new(component_count);
        for node in 0..count {
            let component = partition.component(node);
            if component_sizes[component as usize] > 1 || self.has_call_self_loop(node as u32) {
                recursive_bits.insert(component as usize);
            }
        }

        CallComponentGraph {
            component: partition.components().to_vec(),
            recursive: recursive_bits,
        }
    }

    /// Return the call-only edge graph in dense CSR form.
    fn call_edges(&self) -> (Vec<u32>, Vec<u32>) {
        let mut edge_offsets = vec![0u32; self.symbols.len() + 1];

        // count direct call edges per symbol
        for source in 0..self.symbols.len() {
            let start = self.edge_offsets[source] as usize;
            let end = self.edge_offsets[source + 1] as usize;
            let count = self.edge_kinds[start..end]
                .iter()
                .filter(|kind| **kind == LinkEdgeKind::Call)
                .count();
            edge_offsets[source + 1] = count as u32;
        }

        // prefix sum the counts into CSR offsets
        for source in 0..self.symbols.len() {
            edge_offsets[source + 1] += edge_offsets[source];
        }

        // copy direct call targets into the call-only graph
        let mut edge_targets = Vec::with_capacity(edge_offsets[self.symbols.len()] as usize);
        for source in 0..self.symbols.len() {
            let start = self.edge_offsets[source] as usize;
            let end = self.edge_offsets[source + 1] as usize;
            for position in start..end {
                if self.edge_kinds[position] == LinkEdgeKind::Call {
                    edge_targets.push(self.edge_targets[position]);
                }
            }
        }

        (edge_offsets, edge_targets)
    }

    /// Return whether a symbol calls itself through a direct call edge.
    fn has_call_self_loop(&self, node: u32) -> bool {
        let start = self.edge_offsets[node as usize] as usize;
        let end = self.edge_offsets[node as usize + 1] as usize;

        (start..end).any(|position| {
            self.edge_kinds[position] == LinkEdgeKind::Call && self.edge_targets[position] == node
        })
    }

    /// Return the dense ids reachable in one hop from a source.
    fn successors(&self, source: usize) -> &[u32] {
        let start = self.edge_offsets[source] as usize;
        let end = self.edge_offsets[source + 1] as usize;

        &self.edge_targets[start..end]
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
        Instruction::FunctionAddr { function, .. } | Instruction::FunctionBind { function, .. } => {
            Some(tree.get(*function).symbol)
        }
        Instruction::GlobalAddr { global, .. } => Some(tree.get(*global).symbol),
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
            graph.add_edge(
                source,
                LinkEdge {
                    target: tree.get(*function).symbol,
                    kind: LinkEdgeKind::Address,
                },
            );
        }
        GlobalInitializer::Aggregate(elements) => {
            for element in elements {
                collect_initializer_addresses(element, tree, source, graph);
            }
        }
        GlobalInitializer::Zero | GlobalInitializer::Scalar(_) | GlobalInitializer::Bytes(_) => {}
    }
}
