use destack_serde::Reflect;

use destack_core::FxIndexMap;
use serde::{Deserialize, Serialize};

use crate::{
    Analysis, CallTable, DropTable, EffectTable, Function, FunctionBehavior, FunctionId, Global,
    GlobalInitializer, Instruction, Linkage, MemoryEffect, Mutation, PlaceOrigin, Storage, Symbol,
    Terminator, Tree, TypeId,
};

/// Symbol references for one module.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Reflect)]
pub struct LinkTable {
    /// Defined symbols and nodes sorted by identity.
    nodes: Vec<(Symbol, LinkNode)>,
    /// First outgoing edge offset for each node and the final edge count.
    offsets: Vec<u32>,
    /// Outgoing references grouped by source node.
    edges: Vec<LinkEdge>,
}

/// How one symbol references another.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub enum LinkEdgeKind {
    /// The source calls the target.
    Call,
    /// The source takes the target's address.
    Address,
    /// The source constructs a continuation for the target.
    Continuation,
}

/// One outgoing reference from a symbol to another symbol.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub struct LinkEdge {
    /// The referenced symbol.
    pub target: Symbol,
    /// How the target is referenced.
    pub kind: LinkEdgeKind,
}

/// One defined symbol in the link graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum LinkNode {
    /// A defined function.
    Function {
        /// Visibility and definition location.
        linkage: Linkage,
        /// Memory effect.
        memory: MemoryEffect,
        /// Behavioral effects (unwind, determinism, allocation, and so on).
        behavior: FunctionBehavior,
        /// Estimated inline cost.
        inline_cost: u32,
        /// Whether the function contains calls without a closed target.
        has_open_calls: bool,
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

impl LinkTable {
    /// Look up one symbol's node.
    pub fn node(&self, symbol: Symbol) -> Option<&LinkNode> {
        let index = self
            .nodes
            .binary_search_by_key(&symbol, |(symbol, _)| *symbol)
            .ok()?;

        Some(&self.nodes[index].1)
    }

    /// Iterate over all defined symbol nodes with their identities.
    pub fn nodes(&self) -> impl Iterator<Item = (Symbol, &LinkNode)> {
        self.nodes.iter().map(|(symbol, node)| (*symbol, node))
    }

    /// Return the outgoing references from one source symbol.
    pub fn edges(&self, source: Symbol) -> &[LinkEdge] {
        let Ok(index) = self
            .nodes
            .binary_search_by_key(&source, |(symbol, _)| *symbol)
        else {
            return &[];
        };
        let start = self.offsets[index] as usize;
        let end = self.offsets[index + 1] as usize;

        &self.edges[start..end]
    }

    /// Build a compact link table from symbol nodes and outgoing references.
    fn build(mut nodes: Vec<(Symbol, LinkNode)>, mut references: Vec<(Symbol, LinkEdge)>) -> Self {
        nodes.sort_unstable_by_key(|(symbol, _)| *symbol);
        if nodes.windows(2).any(|nodes| nodes[0].0 == nodes[1].0) {
            unreachable!("duplicate symbol in link table");
        }

        let indices = nodes
            .iter()
            .enumerate()
            .map(|(index, (symbol, _))| (*symbol, index))
            .collect::<FxIndexMap<_, _>>();
        references.sort_unstable_by_key(|(source, edge)| (*source, *edge));

        let mut offsets = vec![0u32; nodes.len() + 1];
        for (source, _) in &references {
            let index = indices
                .get(source)
                .copied()
                .unwrap_or_else(|| unreachable!("link edge has undefined source {source:?}"));
            offsets[index + 1] += 1;
        }
        for index in 0..nodes.len() {
            offsets[index + 1] += offsets[index];
        }

        let edges = references.into_iter().map(|(_, edge)| edge).collect();

        Self {
            nodes,
            offsets,
            edges,
        }
    }

    /// Record address references from a global initializer.
    fn add_initializer_edges(
        edges: &mut Vec<(Symbol, LinkEdge)>,
        source: Symbol,
        initializer: &GlobalInitializer,
        tree: &Tree,
    ) {
        match initializer {
            GlobalInitializer::FunctionAddress(function) => {
                edges.push((
                    source,
                    LinkEdge {
                        target: tree.get(*function).symbol,
                        kind: LinkEdgeKind::Address,
                    },
                ));
            }
            GlobalInitializer::GlobalAddress(global) => {
                edges.push((
                    source,
                    LinkEdge {
                        target: tree.get(*global).symbol,
                        kind: LinkEdgeKind::Address,
                    },
                ));
            }
            GlobalInitializer::Aggregate(elements) => {
                for element in elements {
                    Self::add_initializer_edges(edges, source, element, tree);
                }
            }
            GlobalInitializer::Zero
            | GlobalInitializer::Scalar(_)
            | GlobalInitializer::Bytes(_)
            | GlobalInitializer::String(_)
            | GlobalInitializer::BigInt(_) => {}
        }
    }

    /// Approximate a function's inline cost as its instruction count.
    fn function_inline_cost(function: &Function, tree: &Tree) -> u32 {
        let mut count = 0usize;
        for &block_id in function.blocks() {
            count += tree.get(block_id).instructions.len();
        }

        count.min(u32::MAX as usize) as u32
    }

    /// Return the symbol reference made by one instruction.
    fn instruction_edge(
        instruction: &Instruction,
        function: FunctionId,
        tree: &Tree,
        drops: &DropTable,
    ) -> Option<LinkEdge> {
        // retain global references made directly by memory operands
        if let Some(place) = instruction.place()
            && let PlaceOrigin::Global(global) = place.origin
        {
            return Some(LinkEdge {
                target: tree.get(global).symbol,
                kind: LinkEdgeKind::Address,
            });
        }

        match instruction {
            Instruction::FunctionAddr { function, .. }
            | Instruction::FunctionBind { function, .. } => Some(LinkEdge {
                target: tree.get(*function).symbol,
                kind: LinkEdgeKind::Address,
            }),
            Instruction::Drop { value } => {
                let ty = tree
                    .get(function)
                    .value_type(*value)
                    .unwrap_or_else(|| unreachable!("drop value has no type"));

                Self::destructor_edge(ty, Storage::Frame, tree, drops)
            }
            Instruction::NewZeroed {
                storage_type,
                space,
                ..
            }
            | Instruction::NewUninit {
                storage_type,
                space,
                ..
            } => Self::destructor_edge(*storage_type, Storage::heap(*space), tree, drops),
            Instruction::NewSliceZeroed { element, space, .. }
            | Instruction::NewSliceUninit { element, space, .. } => {
                Self::destructor_edge(*element, Storage::heap(*space), tree, drops)
            }
            _ => None,
        }
    }

    /// Return the destructor reference made by one fallible allocation.
    fn terminator_edge(
        terminator: &Terminator,
        tree: &Tree,
        drops: &DropTable,
    ) -> Option<LinkEdge> {
        let (ty, space) = match terminator {
            Terminator::NewZeroedTry {
                storage_type,
                space,
                ..
            }
            | Terminator::NewUninitTry {
                storage_type,
                space,
                ..
            } => (*storage_type, *space),
            Terminator::NewSliceZeroedTry { element, space, .. }
            | Terminator::NewSliceUninitTry { element, space, .. } => (*element, *space),
            _ => return None,
        };

        Self::destructor_edge(ty, Storage::heap(space), tree, drops)
    }

    /// Return one direct reference to a generated destructor.
    fn destructor_edge(
        ty: TypeId,
        storage: Storage,
        tree: &Tree,
        drops: &DropTable,
    ) -> Option<LinkEdge> {
        let destructor = drops.destructor(ty, storage)?;

        Some(LinkEdge {
            target: tree.get(destructor).symbol,
            kind: LinkEdgeKind::Call,
        })
    }
}

impl Analysis for LinkTable {
    const INVALIDATED_BY: Mutation = CallTable::INVALIDATED_BY
        .union(Mutation::EFFECT)
        .union(Mutation::DROP);
}

impl LinkTable {
    /// Build the link graph for one module from its call graph and tree.
    pub fn analyse(
        call_table: &CallTable,
        effects: &EffectTable,
        drops: &DropTable,
        tree: &Tree,
    ) -> Self {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        // record each defined function with its attributes and outgoing references
        let functions = tree
            .iter_nodes::<Function>()
            .map(|(id, _)| id)
            .collect::<Vec<_>>();
        for function_id in functions {
            let function = tree.get(function_id);
            // skip declarations without a body
            if function.entry().is_none() {
                continue;
            }

            let symbol = function.symbol;
            let tables = effects.function(function_id);
            let memory = tables.map(|m| m.memory.clone()).unwrap_or_default();
            let behavior = tables.map(|m| m.behavior.clone()).unwrap_or_default();
            nodes.push((
                symbol,
                LinkNode::Function {
                    linkage: function.linkage,
                    memory,
                    behavior,
                    inline_cost: Self::function_inline_cost(function, tree),
                    has_open_calls: !call_table.open_callsites(function_id).is_empty(),
                },
            ));

            // lower call edges from tree-local callees to persistent symbols
            for edge in call_table.outgoing(function_id) {
                let target = tree.get(edge.callee).symbol;
                edges.push((
                    symbol,
                    LinkEdge {
                        target,
                        kind: LinkEdgeKind::Call,
                    },
                ));
            }

            // record symbol references from instruction operands
            for index in 0..tree.get(function_id).blocks().len() {
                let block_id = tree.get(function_id).blocks()[index];
                for index in 0..tree.get(block_id).instructions.len() {
                    let instruction_id = tree.get(block_id).instructions[index];
                    if let Some(edge) = LinkTable::instruction_edge(
                        &tree.get(instruction_id).clone(),
                        function_id,
                        tree,
                        drops,
                    ) {
                        edges.push((symbol, edge));
                    }
                }

                let terminator = tree.get(block_id).terminator;
                if let Some(edge) =
                    LinkTable::terminator_edge(&tree.get(terminator).clone(), tree, drops)
                {
                    edges.push((symbol, edge));
                }
            }
        }

        // record each global with its address-of references from its initializer
        for (_global_id, global) in tree.iter_nodes::<Global>() {
            let symbol = global.symbol;
            nodes.push((
                symbol,
                LinkNode::Global {
                    linkage: global.linkage,
                },
            ));
            if let Some(initializer) = &global.initializer {
                Self::add_initializer_edges(&mut edges, symbol, initializer, tree);
            }
        }

        Self::build(nodes, edges)
    }
}
