use destack_dir as dir;
use indexmap::IndexMap;
use smallvec::SmallVec;

use crate::check::Origin;
use crate::{CompilerError, CompilerResult};

/// One bound collected for an inference variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct TypeBound {
    /// The bound type.
    pub(in crate::check) ty: dir::GlobalTypeId,
    /// The source occurrence that produced the bound.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
}

impl TypeBound {
    /// Return one type bound from a source occurrence.
    pub(in crate::check) fn new(ty: dir::GlobalTypeId, source: dir::GlobalNodeIdAny) -> Self {
        Self { ty, source }
    }
}

/// One open inference variable.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct VariableState {
    /// The source that produced the variable.
    pub(in crate::check) origin: Origin,
    /// The literal widening policy applied when solving.
    pub(in crate::check) widening: Widening,
    /// Types that must be assignable to the variable.
    pub(in crate::check) lower: SmallVec<[TypeBound; 2]>,
    /// Types the variable must be assignable to.
    pub(in crate::check) upper: SmallVec<[TypeBound; 2]>,
    /// The solved type, when solving finished.
    pub(in crate::check) solution: Option<dir::GlobalTypeId>,
    /// The default solution applied when no bounds arrive.
    pub(in crate::check) default: Option<dir::GlobalTypeId>,
    /// The union-find representative, when aliased to another variable.
    pub(in crate::check) alias: Option<dir::TypeVariableId>,
}

/// Literal widening policy for one solved variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
