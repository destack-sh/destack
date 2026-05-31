use destack_dir as dir;
use indexmap::IndexMap;

use crate::check::{
    CallDecision, Constraint, ConstructDecision, IdentityDecision, Induction, LayoutDecision,
    MemberDecision, Obligation, OperatorDecision, ReceiverResolution, Solution, StaticOperand,
    TermTable, TypeOperand, VariableId, VariableTable,
};

/// The inference graph for one checked component.
#[derive(Debug)]
pub(in crate::check) struct InferenceTable {
    /// Component-wide term table.
    pub(in crate::check) terms: TermTable,
    /// Component-wide variable graph.
    pub(in crate::check) variables: VariableTable,
    /// Variable constraints in collection order.
    pub(in crate::check) constraints: Vec<Constraint>,
    /// Component-wide post-solve obligations.
    pub(in crate::check) obligations: Vec<Obligation>,
    /// Declaration terms that can induce owner generics.
    pub(in crate::check) inductions: Vec<Induction>,

    /// Type operands that must be assignable to each variable.
    pub(in crate::check) type_lower_bounds: IndexMap<VariableId, Vec<TypeOperand>>,
    /// Type operands that each variable must be assignable to.
    pub(in crate::check) type_upper_bounds: IndexMap<VariableId, Vec<TypeOperand>>,
    /// Static operands that must be assignable to each variable.
    pub(in crate::check) static_lower_bounds: IndexMap<VariableId, Vec<StaticOperand>>,
    /// Static operands that each variable must be assignable to.
    pub(in crate::check) static_upper_bounds: IndexMap<VariableId, Vec<StaticOperand>>,

    /// Solved variable values.
    pub(in crate::check) variable_solutions: IndexMap<VariableId, Solution>,
    /// Runtime calls resolved or rejected by solve.
    pub(in crate::check) calls: IndexMap<dir::GlobalNodeIdAny, CallDecision>,
    /// Runtime construct expressions resolved or rejected by solve.
    pub(in crate::check) constructs: IndexMap<dir::GlobalNodeIdAny, ConstructDecision>,
    /// Runtime operators resolved or rejected by solve.
    pub(in crate::check) operators: IndexMap<dir::GlobalNodeIdAny, OperatorDecision>,
    /// Runtime identity checks resolved or rejected by solve.
    pub(in crate::check) identities: IndexMap<dir::GlobalNodeIdAny, IdentityDecision>,
    /// Layout queries resolved or rejected by solve.
    pub(in crate::check) layouts: IndexMap<dir::GlobalNodeIdAny, LayoutDecision>,
    /// Runtime members resolved or rejected by solve.
    pub(in crate::check) members: IndexMap<dir::GlobalNodeIdAny, MemberDecision>,
    /// Contextual receivers resolved by check.
    pub(in crate::check) receivers: IndexMap<dir::GlobalNodeIdAny, ReceiverResolution>,
    /// Lexical names resolved by check.
    pub(in crate::check) names: IndexMap<dir::GlobalNodeIdAny, dir::NameResolution>,
}

impl InferenceTable {
    /// Create an empty inference graph.
    pub(in crate::check) fn new() -> Self {
        Self {
            terms: TermTable::new(),
            variables: VariableTable::new(),
            constraints: Vec::new(),
            obligations: Vec::new(),
            inductions: Vec::new(),
            type_lower_bounds: IndexMap::new(),
            type_upper_bounds: IndexMap::new(),
            static_lower_bounds: IndexMap::new(),
            static_upper_bounds: IndexMap::new(),
            variable_solutions: IndexMap::new(),
            calls: IndexMap::new(),
            constructs: IndexMap::new(),
            operators: IndexMap::new(),
            identities: IndexMap::new(),
            layouts: IndexMap::new(),
            members: IndexMap::new(),
            receivers: IndexMap::new(),
            names: IndexMap::new(),
        }
    }
}
