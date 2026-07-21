use destack_core::BitSet;
use destack_mir::{self as mir, CallComponentGraph, LinkGraph, LinkSupergraph, Symbol};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// Lowered MIR payload before optimization.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct MirLowered {
    /// The MIR tree.
    pub tree: mir::Tree,
    /// Target ABI layout.
    pub target: mir::TargetLayout,
    /// Canonical MIR type table.
    pub types: mir::TypeTable,
    /// Canonical MIR layout table.
    pub layouts: mir::LayoutTable,
    /// Canonical MIR dispatch table.
    pub dispatch: mir::DispatchTable,
    /// Canonical MIR drop table.
    pub drops: mir::DropTable,
    /// Explicit MIR memory access table.
    pub memory: mir::MemoryTable,
    /// Function and call effect table.
    pub effects: mir::EffectTable,
    /// Static profile counter table.
    pub profile: mir::ProfileTable,
    /// The module initializer storing runtime bindings, when one exists.
    pub initializer: Option<mir::FunctionId>,
}

impl MirLowered {
    /// Create a new lowered MIR payload.
    pub fn new() -> Self {
        Self {
            tree: mir::Tree::new(),
            target: mir::TargetLayout::default(),
            types: mir::TypeTable::default(),
            layouts: mir::LayoutTable::default(),
            dispatch: mir::DispatchTable::default(),
            drops: mir::DropTable::default(),
            memory: mir::MemoryTable::default(),
            effects: mir::EffectTable::default(),
            profile: mir::ProfileTable::default(),
            initializer: None,
        }
    }
}

impl Default for MirLowered {
    fn default() -> Self {
        Self::new()
    }
}

/// Successful MIR verification marker.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Reflect)]
pub struct MirVerified;

/// MIR after required elaboration.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct MirElaborated {
    /// The elaborated MIR tree.
    pub tree: mir::Tree,
    /// Target ABI layout.
    pub target: mir::TargetLayout,
    /// Canonical MIR type table.
    pub types: mir::TypeTable,
    /// Canonical MIR layout table.
    pub layouts: mir::LayoutTable,
    /// Canonical MIR dispatch table.
    pub dispatch: mir::DispatchTable,
    /// Canonical MIR drop table.
    pub drops: mir::DropTable,
    /// Explicit MIR memory access table.
    pub memory: mir::MemoryTable,
    /// Function and call effect table.
    pub effects: mir::EffectTable,
    /// Static profile counter table.
    pub profile: mir::ProfileTable,
}

/// Optimized MIR payload after pipeline transforms.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct MirOptimized {
    /// The optimized MIR tree.
    pub tree: mir::Tree,
    /// Target ABI layout.
    pub target: mir::TargetLayout,
    /// Canonical MIR type table.
    pub types: mir::TypeTable,
    /// Canonical MIR layout table.
    pub layouts: mir::LayoutTable,
    /// Canonical MIR dispatch table.
    pub dispatch: mir::DispatchTable,
    /// Canonical MIR drop table.
    pub drops: mir::DropTable,
    /// Explicit MIR memory access table.
    pub memory: mir::MemoryTable,
    /// Function and call effect table.
    pub effects: mir::EffectTable,
    /// Static profile counter table.
    pub profile: mir::ProfileTable,
}

impl MirOptimized {
    /// Create a new optimized MIR payload.
    pub fn new() -> Self {
        Self {
            tree: mir::Tree::new(),
            target: mir::TargetLayout::default(),
            types: mir::TypeTable::default(),
            layouts: mir::LayoutTable::default(),
            dispatch: mir::DispatchTable::default(),
            drops: mir::DropTable::default(),
            memory: mir::MemoryTable::default(),
            effects: mir::EffectTable::default(),
            profile: mir::ProfileTable::default(),
        }
    }

    /// Return the optimized MIR tree.
    pub fn tree(&self) -> &mir::Tree {
        &self.tree
    }
}

impl Default for MirOptimized {
    fn default() -> Self {
        Self::new()
    }
}

/// Per-module link summary produced by program analysis.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct MirAnalyzed {
    /// The module's symbol reference graph.
    pub links: LinkGraph,
}

impl MirAnalyzed {
    /// Create a per-module analysis payload from its link graph.
    pub fn new(links: LinkGraph) -> Self {
        Self { links }
    }
}

/// Whole-program analysis columns shared across the optimization of every module.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Reflect)]
pub struct ProgramAnalysis {
    /// Every defined symbol in the program.
    symbols: Vec<Symbol>,
    /// Whether each symbol is reachable from a program root.
    live: BitSet,
    /// Program-wide incoming reference count of each symbol.
    references: Vec<u32>,
    /// Whether each symbol's address is taken anywhere in the program.
    address_taken: BitSet,
    /// Whether each symbol is internal to the program (not an external root).
    internal: BitSet,
    /// Strongly connected components of the whole-program call graph.
    components: CallComponentGraph,
}

impl ProgramAnalysis {
    /// Create an empty whole-program analysis.
    pub fn new() -> Self {
        Self::default()
    }

    /// Derive the whole-program columns from the supergraph and the program's roots.
    pub fn analyze(supergraph: &LinkSupergraph, roots: &[Symbol]) -> Self {
        Self {
            symbols: supergraph.symbols().to_vec(),
            live: supergraph.reachable(roots),
            references: supergraph.reference_counts(),
            address_taken: supergraph.address_taken(),
            internal: supergraph.internal(roots),
            components: supergraph.call_components(),
        }
    }

    /// Return whether the analysis covers any symbols.
    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }

    /// Return whether the whole program can reach a symbol.
    pub fn is_live(&self, symbol: Symbol) -> bool {
        self.live.contains(self.index_of(symbol))
    }

    /// Return the program-wide reference count of a symbol.
    pub fn references(&self, symbol: Symbol) -> u32 {
        self.references[self.index_of(symbol)]
    }

    /// Return whether a symbol's address is taken anywhere in the program.
    pub fn is_address_taken(&self, symbol: Symbol) -> bool {
        self.address_taken.contains(self.index_of(symbol))
    }

    /// Return whether a symbol is internal to the program (not an external root).
    pub fn is_internal(&self, symbol: Symbol) -> bool {
        self.internal.contains(self.index_of(symbol))
    }

    /// Return whether a symbol belongs to a recursive call-graph component.
    pub fn is_recursive(&self, symbol: Symbol) -> bool {
        self.components.is_recursive(self.index_of(symbol))
    }

    /// Return the dense id of a symbol the program defines.
    fn index_of(&self, symbol: Symbol) -> usize {
        self.symbols
            .binary_search(&symbol)
            .expect("queried a symbol the program analysis does not define")
    }
}
