use destack_dir as dir;
use indexmap::IndexMap;
use smallvec::SmallVec;

use crate::check::{Origin, Task};
use crate::{CompilerError, CompilerResult};

/// One open inference variable.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct VariableState {
    /// The source that produced the variable.
    pub(in crate::check) origin: Origin,
    /// The literal widening policy applied when solving.
    pub(in crate::check) widening: Widening,
    /// Types that must be assignable to the variable.
    pub(in crate::check) lower: SmallVec<[dir::GlobalTypeId; 2]>,
    /// Types the variable must be assignable to.
    pub(in crate::check) upper: SmallVec<[dir::GlobalTypeId; 2]>,
    /// The solved type, when solving finished.
    pub(in crate::check) solution: Option<dir::GlobalTypeId>,
    /// The default solution applied when no bounds arrive.
    pub(in crate::check) default: Option<dir::GlobalTypeId>,
    /// The union-find representative, when aliased to another variable.
    pub(in crate::check) alias: Option<dir::TypeVariableId>,
    /// Tasks waiting for this variable to solve.
    pub(in crate::check) waiters: SmallVec<[Task; 2]>,
}

/// Literal widening policy for one solved variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum Widening {
    /// Keep literal solutions exact.
    Preserve,
    /// Widen literal solutions to their base type.
    Widen,
}

/// Variable states owned by one solver.
#[derive(Debug)]
pub(in crate::check) struct VariableTable {
    /// Variables keyed by absolute variable id.
    variables: IndexMap<dir::TypeVariableId, VariableState>,
}

impl VariableTable {
    /// Create an empty variable table.
    pub(in crate::check) fn new() -> Self {
        Self {
            variables: IndexMap::new(),
        }
    }

    /// Allocate one variable state.
    pub(in crate::check) fn allocate(
        &mut self,
        variable: dir::TypeVariableId,
        origin: Origin,
        widening: Widening,
    ) {
        self.insert(
            variable,
            VariableState {
                origin,
                widening,
                lower: SmallVec::new(),
                upper: SmallVec::new(),
                solution: None,
                default: None,
                alias: None,
                waiters: SmallVec::new(),
            },
        );
    }

    /// Insert one variable state.
    pub(in crate::check) fn insert(&mut self, variable: dir::TypeVariableId, state: VariableState) {
        self.variables.insert(variable, state);
    }

    /// Remove one variable state.
    pub(in crate::check) fn remove(
        &mut self,
        variable: dir::TypeVariableId,
    ) -> Option<VariableState> {
        self.variables.swap_remove(&variable)
    }

    /// Return one variable state.
    pub(in crate::check) fn get(&self, id: dir::TypeVariableId) -> CompilerResult<&VariableState> {
        self.variables
            .get(&id)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check variable {id:?} is not allocated"),
            })
    }

    /// Return one mutable variable state.
    pub(in crate::check) fn get_mut(
        &mut self,
        id: dir::TypeVariableId,
    ) -> CompilerResult<&mut VariableState> {
        self.variables
            .get_mut(&id)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check variable {id:?} is not allocated"),
            })
    }

    /// Return the total number of allocated variables.
    pub(in crate::check) fn count(&self) -> usize {
        self.variables.len()
    }

    /// Iterate all variables with their ids.
    pub(in crate::check) fn iter(
        &self,
    ) -> impl Iterator<Item = (dir::TypeVariableId, &VariableState)> + '_ {
        self.variables
            .iter()
            .map(|(variable, state)| (*variable, state))
    }
}
