use indexmap::IndexMap;

use destack_dir::{GlobalNodeIdAny, GlobalSymbolId, InferVarId, LocalTypeId, VarianceBound};

/// Represent a single inference variable with bounds and defaults.
#[derive(Debug, Clone)]
pub struct InferVar {
    /// Lower bounds for this variable.
    pub lower_bounds: Vec<LocalTypeId>,
    /// Upper bounds for this variable.
    pub upper_bounds: Vec<LocalTypeId>,
    /// Default type if the variable remains unconstrained.
    pub default: Option<LocalTypeId>,
    /// Origin of this variable for diagnostics.
    pub origin: InferOrigin,
    /// Scope used to limit variable usage and reporting.
    pub scope: InferScope,
}

impl InferVar {
    /// Create a new inference variable.
    pub fn new(origin: InferOrigin, scope: InferScope) -> Self {
        Self {
            lower_bounds: Vec::new(),
            upper_bounds: Vec::new(),
            default: None,
            origin,
            scope,
        }
    }
}

/// Describe where an inference variable was created.
#[derive(Debug, Clone, Copy)]
pub enum InferOrigin {
    /// Variable introduced by an expression.
    Expression(
        /// Identify the expression node.
        GlobalNodeIdAny,
    ),
    /// Variable introduced by a parameter.
    Parameter(
        /// Identify the parameter node.
        GlobalNodeIdAny,
    ),
    /// Variable introduced by a return position.
    Return(
        /// Identify the return node.
        GlobalNodeIdAny,
    ),
    /// Variable introduced by a type parameter.
    TypeParameter(
        /// Identify the type parameter symbol.
        GlobalSymbolId,
    ),
    /// Variable introduced by a constraint group.
    ConstraintGroup(
        /// Identify the constraint group.
        ConstraintGroupId,
    ),
}

/// Describe the scope of an inference variable.
#[derive(Debug, Clone, Copy)]
pub struct InferScope {
    /// Owning symbol for this inference variable.
    pub owner: GlobalSymbolId,
    /// Function boundary for this inference variable.
    pub function_id: Option<GlobalNodeIdAny>,
}

/// Group id used to tie constraints for candidate selection.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ConstraintGroupId(
    /// Identify the constraint group.
    pub u32,
);

impl ConstraintGroupId {
    /// Wrap an id as a ConstraintGroupId.
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

/// Represent a constraint over types and inference variables.
#[derive(Debug, Clone)]
pub enum Constraint {
    /// Require two types to be equal.
    Equal {
        /// Identify the left type.
        left: LocalTypeId,
        /// Identify the right type.
        right: LocalTypeId,
    },
    /// Require one type to be a subtype of another.
    Subtype {
        /// Identify the subtype.
        sub_type: LocalTypeId,
        /// Identify the supertype.
        super_type: LocalTypeId,
        /// Store any variance bounds for the constraint.
        variance: Option<VarianceBound>,
    },
    /// Require a variable to join multiple source types.
    Join {
        /// Identify the inference variable.
        target: InferVarId,
        /// Store the types to join.
        sources: Vec<LocalTypeId>,
    },
    /// Require instantiation of a generic type.
    Instantiate {
        /// Identify the inference variable.
        target: InferVarId,
        /// Identify the generic type.
        generic_type: LocalTypeId,
        /// Store static arguments for instantiation.
        static_arguments: Vec<LocalTypeId>,
    },
    /// Require a type based on a guard condition.
    Conditional {
        /// Identify the guard type.
        guard: LocalTypeId,
        /// Identify the type for the true branch.
        when_true: LocalTypeId,
        /// Identify the type for the false branch.
        when_false: LocalTypeId,
    },
    /// Require one of a set of candidate constraint groups to hold.
    CandidateGroup {
        /// Identify the constraint group.
        id: ConstraintGroupId,
        /// Store constraint options for candidate selection.
        options: Vec<Vec<Constraint>>,
    },
}

/// Store inference variables and constraints for a module.
#[derive(Debug, Default)]
pub struct InferTable {
    /// All inference variables allocated in this module.
    pub vars: Vec<InferVar>,
    /// All constraints collected during inference.
    pub constraints: Vec<Constraint>,
    /// Inference variables associated with nodes.
    pub var_by_node_id: IndexMap<GlobalNodeIdAny, InferVarId>,
    /// Inference variables associated with symbols.
    pub var_by_symbol_id: IndexMap<GlobalSymbolId, InferVarId>,
    /// Inference variables associated with type parameters.
    pub var_by_type_parameter: IndexMap<GlobalSymbolId, InferVarId>,
    /// Types that wrap inference variables by id.
    pub type_by_var_id: Vec<LocalTypeId>,
}

impl InferTable {
    /// Allocate a new inference variable.
    pub fn new_var(&mut self, origin: InferOrigin, scope: InferScope) -> InferVarId {
        let id = InferVarId::new(self.vars.len() as u32);
        self.vars.push(InferVar::new(origin, scope));
        self.type_by_var_id.push(LocalTypeId::new(u32::MAX));
        id
    }

    /// Push a new constraint.
    #[inline]
    pub fn push_constraint(&mut self, constraint: Constraint) {
        self.constraints.push(constraint);
    }

    /// Bind a type to an inference variable id.
    pub fn bind_type(&mut self, id: InferVarId, ty_id: LocalTypeId) {
        let index = id.0 as usize;
        if self.type_by_var_id.len() <= index {
            self.type_by_var_id
                .resize(index + 1, LocalTypeId::new(u32::MAX));
        }
        self.type_by_var_id[index] = ty_id;
    }

    /// Get the type that wraps an inference variable.
    pub fn type_for_var(&self, id: InferVarId) -> Option<LocalTypeId> {
        self.type_by_var_id.get(id.0 as usize).copied()
    }
}
