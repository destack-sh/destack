use destack_dir as dir;
use indexmap::{IndexMap, IndexSet};

use super::VariableId;

/// Flow state while walking one module.
#[derive(Debug, Default)]
pub(in crate::check) struct FlowState {
    /// Function bodies currently being walked.
    pub(in crate::check) functions: Vec<FunctionFrame>,
    /// Local symbols definitely assigned at the current walk point.
    pub(in crate::check) assigned_symbols: IndexSet<dir::GlobalSymbolId>,
    /// Flow-refined type variables keyed by local symbol.
    pub(in crate::check) refined_symbol_types: IndexMap<dir::GlobalSymbolId, VariableId>,
}

/// A function body currently being walked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct FunctionFrame {
    /// The implicit this type variable.
    pub(in crate::check) this_type: Option<VariableId>,
    /// The inferred return type variable.
    pub(in crate::check) return_type: VariableId,
}

/// A scoped refinement saved for restoration after a branch walk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct RefinementFrame {
    /// The refined symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The refinement visible before the branch.
    pub(in crate::check) previous: Option<VariableId>,
}

impl FlowState {
    /// Enter one function body while walking.
    pub(in crate::check) fn push_function(&mut self, function: FunctionFrame) {
        self.functions.push(function);
    }

    /// Leave the current function body.
    pub(in crate::check) fn pop_function(&mut self) {
        self.functions.pop();
    }

    /// Return the current function body.
    pub(in crate::check) fn current_function(&self) -> Option<FunctionFrame> {
        self.functions.last().copied()
    }

    /// Mark one local symbol as definitely assigned.
    pub(in crate::check) fn mark_assigned(&mut self, symbol: dir::GlobalSymbolId) {
        self.assigned_symbols.insert(symbol);
    }

    /// Return whether one local symbol is definitely assigned.
    pub(in crate::check) fn is_assigned(&self, symbol: dir::GlobalSymbolId) -> bool {
        self.assigned_symbols.contains(&symbol)
    }

    /// Enter one scoped refinement.
    pub(in crate::check) fn push_refinement(
        &mut self,
        symbol: dir::GlobalSymbolId,
        ty: VariableId,
    ) -> RefinementFrame {
        let previous = self.refined_symbol_types.insert(symbol, ty);

        RefinementFrame { symbol, previous }
    }

    /// Leave one scoped refinement.
    pub(in crate::check) fn pop_refinement(&mut self, frame: RefinementFrame) {
        if let Some(previous) = frame.previous {
            self.refined_symbol_types.insert(frame.symbol, previous);
        } else {
            self.refined_symbol_types.shift_remove(&frame.symbol);
        }
    }

    /// Refine one symbol at the current walk point.
    pub(in crate::check) fn refine_symbol(&mut self, symbol: dir::GlobalSymbolId, ty: VariableId) {
        self.refined_symbol_types.insert(symbol, ty);
    }
}
