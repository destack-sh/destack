use destack_dir as dir;
use indexmap::IndexMap;
use smallvec::SmallVec;

use crate::check::{Origin, Relation};
use crate::{CompilerError, CompilerResult};

/// When one bound may choose an inference variable's solution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) enum BoundMode {
    /// Bound may solve the variable during regular solver work.
    Strong,
    /// Bound may solve the variable only after regular work drains.
    Weak,
}

impl BoundMode {
    /// Return whether this solve pass may use weak bounds.
    pub(in crate::check) fn allows_weak(self) -> bool {
        matches!(self, Self::Weak)
    }
}

/// One bound collected for an inference variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct TypeBound {
    /// The bound type.
    pub(in crate::check) ty: dir::GlobalTypeId,
    /// The relation between the variable and this bound.
    pub(in crate::check) relation: Relation,
    /// The source occurrence that produced the bound.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// When this bound may choose the variable's solution.
    pub(in crate::check) mode: BoundMode,
}

impl TypeBound {
    /// Return one type bound from a source occurrence.
    pub(in crate::check) fn new(
        ty: dir::GlobalTypeId,
        relation: Relation,
        source: dir::GlobalNodeIdAny,
        mode: BoundMode,
    ) -> Self {
        Self {
            ty,
            relation,
            source,
            mode,
        }
    }
}

/// Special behavior attached to one inference variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum VariableRole {
    /// Ordinary inference variable.
    Inference,
    /// Variable that ranges over lifetime terms.
    Lifetime {
        /// The lifetime-kind constraint.
        constraint: Option<dir::GlobalTypeId>,
    },
}

impl VariableRole {
    /// Return whether this role is ordinary inference.
    pub(in crate::check) fn is_inference(self) -> bool {
        matches!(self, Self::Inference)
    }
}

/// One open inference variable.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct VariableState {
    /// The source that produced the variable.
    pub(in crate::check) origin: Origin,
    /// The literal widening policy applied when solving.
    pub(in crate::check) widening: Widening,
    /// Types that must relate to the variable.
    pub(in crate::check) lower: SmallVec<[TypeBound; 2]>,
    /// Types the variable must relate to.
    pub(in crate::check) upper: SmallVec<[TypeBound; 2]>,
    /// The solved type, when solving finished.
    pub(in crate::check) solution: Option<dir::GlobalTypeId>,
    /// The default solution applied when no bounds arrive.
    pub(in crate::check) default: Option<dir::GlobalTypeId>,
    /// The special behavior attached to this variable.
    pub(in crate::check) role: VariableRole,
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
                role: VariableRole::Inference,
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
