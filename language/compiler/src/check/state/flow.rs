use destack_dir as dir;
use indexmap::{IndexMap, IndexSet};
use smallvec::SmallVec;

use super::{Capture, VariableId};

/// Flow state while walking one module.
#[derive(Debug, Default)]
pub(in crate::check) struct FlowState {
    /// Function bodies currently being walked.
    pub(in crate::check) functions: Vec<FunctionFrame>,
    /// Local symbols definitely assigned at the current walk point.
    pub(in crate::check) assigned_symbols: IndexSet<dir::GlobalSymbolId>,
    /// Flow-refined type variables keyed by access path.
    pub(in crate::check) type_refinements: IndexMap<FlowPath, VariableId>,
}

/// One value path tracked by flow analysis.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(in crate::check) struct FlowPath {
    /// The root binding symbol.
    pub(in crate::check) root: dir::GlobalSymbolId,
    /// The selected path segments below the root.
    pub(in crate::check) segments: SmallVec<[dir::StaticKey; 2]>,
}

impl FlowPath {
    /// Create a path rooted at one binding symbol.
    pub(in crate::check) fn symbol(symbol: dir::GlobalSymbolId) -> Self {
        Self {
            root: symbol,
            segments: SmallVec::new(),
        }
    }
}

/// A function body currently being walked.
#[derive(Debug)]
pub(in crate::check) struct FunctionFrame {
    /// The function symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The implicit this type variable.
    pub(in crate::check) this_type: Option<VariableId>,
    /// The inferred return type variable.
    pub(in crate::check) return_type: VariableId,
    /// Outer symbols read by this function.
    pub(in crate::check) captured_symbols: IndexSet<dir::GlobalSymbolId>,
    /// Whether this function reads an outer `this`.
    pub(in crate::check) captures_this: bool,
}

/// A scoped type refinement saved for restoration after a branch walk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) struct TypeRefinementFrame {
    /// The refined path.
    pub(in crate::check) path: FlowPath,
    /// The refinement visible before the branch.
    pub(in crate::check) previous: Option<VariableId>,
}

impl FlowState {
    /// Enter one function body while walking.
    pub(in crate::check) fn push_function(&mut self, function: FunctionFrame) {
        self.functions.push(function);
    }

    /// Leave the current function body.
    pub(in crate::check) fn pop_function(&mut self) -> Option<Capture> {
        let function = self.functions.pop()?;
        let symbols = function.captured_symbols.iter().copied().collect();

        Some(Capture {
            symbol: function.symbol,
            symbols,
            captures_this: function.captures_this,
        })
    }

    /// Return the current function body.
    pub(in crate::check) fn current_function(&self) -> Option<&FunctionFrame> {
        self.functions.last()
    }

    /// Record one symbol captured by the current function body.
    pub(in crate::check) fn capture_symbol(&mut self, symbol: dir::GlobalSymbolId) {
        if let Some(function) = self.functions.last_mut() {
            function.captured_symbols.insert(symbol);
        }
    }

    /// Record `this` captured by the current function body.
    pub(in crate::check) fn capture_this(&mut self) {
        if let Some(function) = self.functions.last_mut() {
            function.captures_this = true;
        }
    }

    /// Mark one local symbol as definitely assigned.
    pub(in crate::check) fn mark_assigned(&mut self, symbol: dir::GlobalSymbolId) {
        self.assigned_symbols.insert(symbol);
    }

    /// Return whether one local symbol is definitely assigned.
    pub(in crate::check) fn is_assigned(&self, symbol: dir::GlobalSymbolId) -> bool {
        self.assigned_symbols.contains(&symbol)
    }

    /// Enter one scoped type refinement.
    pub(in crate::check) fn push_type_refinement(
        &mut self,
        path: FlowPath,
        ty: VariableId,
    ) -> TypeRefinementFrame {
        let previous = self.type_refinements.insert(path.clone(), ty);

        TypeRefinementFrame { path, previous }
    }

    /// Leave one scoped type refinement.
    pub(in crate::check) fn pop_type_refinement(&mut self, frame: TypeRefinementFrame) {
        if let Some(previous) = frame.previous {
            self.type_refinements.insert(frame.path, previous);
        } else {
            self.type_refinements.shift_remove(&frame.path);
        }
    }

    /// Refine one path at the current walk point.
    pub(in crate::check) fn refine_type(&mut self, path: FlowPath, ty: VariableId) {
        self.type_refinements.insert(path, ty);
    }

    /// Return the current type refinement for one symbol.
    pub(in crate::check) fn symbol_type_refinement(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<VariableId> {
        let path = FlowPath::symbol(symbol);

        self.type_refinements.get(&path).copied()
    }
}
