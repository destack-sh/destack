use std::sync::Arc;

use destack_core::BitSet;
use destack_mir::{self as mir, CallComponentTable, LinkSupergraph, LinkTable, Symbol};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// The declarations one module lowers ahead of its bodies: its types and its headers.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Reflect)]
pub struct MirDeclared {
    /// The MIR tree holding the declarations.
    pub tree: Arc<mir::Tree>,
    /// Target ABI layout.
    pub target: mir::TargetLayout,
}

/// MIR produced by lowering.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct MirLowered {
    /// The MIR tree.
    pub tree: Arc<mir::Tree>,
    /// Target ABI layout.
    pub target: mir::TargetLayout,
    /// The module initializer storing runtime bindings, when one exists.
    pub initializer: Option<mir::FunctionId>,

    /// Canonical MIR layout table.
    pub layouts: mir::LayoutTable,
    /// Canonical MIR dispatch table.
    pub dispatch: mir::DispatchTable,
    /// Canonical MIR drop table.
    pub drops: mir::DropTable,
    /// The witnesses that satisfy each closed type's constraints.
    pub witnesses: mir::WitnessTable,
    /// Function and call effect table.
    pub effects: mir::EffectTable,
    /// Static profile counter table.
    pub profile: mir::ProfileTable,
}

impl MirLowered {
    /// Create a new lowered MIR payload.
    pub fn new() -> Self {
        Self {
            tree: Arc::new(mir::Tree::new()),
            target: mir::TargetLayout::default(),
            initializer: None,
            layouts: mir::LayoutTable::default(),
            dispatch: mir::DispatchTable::default(),
            drops: mir::DropTable::default(),
            witnesses: mir::WitnessTable::default(),
            effects: mir::EffectTable::default(),
            profile: mir::ProfileTable::default(),
        }
    }
}

impl Default for MirLowered {
    fn default() -> Self {
        Self::new()
    }
}

/// Ownership retention produced by MIR verification.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct MirVerified {
    /// Ownership retention required by drop elaboration.
    pub retention: mir::RetentionTable,
}

/// MIR with the requested instance bodies filled.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct MirInstantiated {
    /// The MIR tree.
    pub tree: Arc<mir::Tree>,
    /// Target ABI layout.
    pub target: mir::TargetLayout,
    /// The module initializer, when one exists.
    pub initializer: Option<mir::FunctionId>,
    /// Type layouts.
    pub layouts: mir::LayoutTable,
    /// Dispatch tables.
    pub dispatch: mir::DispatchTable,
    /// Drop hooks by type.
    pub drops: mir::DropTable,
    /// Function and call effects.
    pub effects: mir::EffectTable,
    /// Static profile counters.
    pub profile: mir::ProfileTable,
    /// Required ownership retention in this tree.
    pub retention: mir::RetentionTable,
}

/// MIR with explicit ownership and concrete representations.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct MirElaborated {
    /// The elaborated MIR tree.
    pub tree: mir::Tree,
    /// Target ABI layout.
    pub target: mir::TargetLayout,
    /// The module initializer, when one exists.
    pub initializer: Option<mir::FunctionId>,
    /// Canonical MIR layout table.
    pub layouts: mir::LayoutTable,
    /// Canonical MIR dispatch table.
    pub dispatch: mir::DispatchTable,
    /// Canonical MIR drop table.
    pub drops: mir::DropTable,
    /// Function and call effect table.
    pub effects: mir::EffectTable,
    /// Static profile counter table.
    pub profile: mir::ProfileTable,
}

/// MIR prepared for emission.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct MirOptimized {
    /// The optimized MIR tree.
    pub tree: mir::Tree,
    /// Target ABI layout.
    pub target: mir::TargetLayout,
    /// The module initializer, when one exists.
    pub initializer: Option<mir::FunctionId>,
    /// Canonical MIR layout table.
    pub layouts: mir::LayoutTable,
    /// Canonical MIR dispatch table.
    pub dispatch: mir::DispatchTable,
    /// Canonical MIR drop table.
    pub drops: mir::DropTable,
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
            initializer: None,
            layouts: mir::LayoutTable::default(),
            dispatch: mir::DispatchTable::default(),
            drops: mir::DropTable::default(),
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

/// The symbol links one module contributes to whole-program analysis.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct MirAnalyzed {
    /// The module's symbol links.
    pub links: LinkTable,
    /// The module initializer called by the runtime, when one exists.
    pub initializer: Option<Symbol>,
    /// Extracted function effects and pointer flows.
    pub functions: Vec<(Symbol, Arc<mir::FunctionEffectBody>)>,
}

/// Whole-program analysis columns shared across the optimization of every module.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Reflect)]
pub struct ProgramAnalysis {
    /// Every defined symbol in the program.
    pub symbols: Vec<Symbol>,
    /// Whether each symbol is reachable from a program root.
    pub live: BitSet,
    /// Program-wide incoming reference count of each symbol.
    pub references: Vec<u32>,
    /// Whether each symbol's address is taken anywhere in the program.
    pub address_taken: BitSet,
    /// Whether each symbol is internal to the program.
    pub internal: BitSet,
    /// Strongly connected components of the whole-program call graph.
    pub components: CallComponentTable,

    /// Function effects and escape paths with reusable recursive components.
    pub effects: mir::ProgramEffectTable,
}

impl ProgramAnalysis {
    /// Create an empty whole-program analysis.
    pub fn new() -> Self {
        Self::default()
    }

    /// Derive the whole-program columns from the supergraph and the program's roots.
    pub fn analyze(
        supergraph: &LinkSupergraph,
        roots: &[Symbol],
        effects: mir::ProgramEffectTable,
    ) -> Self {
        Self {
            symbols: supergraph.symbols().to_vec(),
            live: supergraph.reachable(roots),
            references: supergraph.reference_counts(),
            address_taken: supergraph.address_taken(),
            internal: supergraph.internal(roots),
            components: supergraph.call_components(),
            effects,
        }
    }

    /// Return whether the analysis covers any symbols.
    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }

    /// Return whether a symbol stays live in the whole program.
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

    /// Return whether a symbol is internal to the program.
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
