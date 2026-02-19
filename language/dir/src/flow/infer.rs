use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{
    Expression, GlobalNodeIdAny, GlobalSymbolId, InferVarId, LocalNodeId, LocalResolutionId,
    LocalTypeId, StaticArgument, VarianceBound,
};

/// Represent a single inference variable with bounds and defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct InferScope {
    /// Owning symbol for this inference variable.
    pub owner: GlobalSymbolId,
    /// Function boundary for this inference variable.
    pub function_id: Option<GlobalNodeIdAny>,
}

/// Group id used to tie constraints for candidate selection.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Serialize, Deserialize)]
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
    pub type_by_var_id: Vec<Option<LocalTypeId>>,
    /// Stable cache key base for this table.
    cache_key_base: u64,
    /// Mutation generation for this table.
    cache_generation: u64,
    /// Associated comptime projection obligations collected during infer.
    #[serde(skip)]
    pub associated_comptime_projection_obligations: Vec<AssociatedComptimeProjectionObligation>,
    /// Instance-commit obligations collected during infer.
    #[serde(skip)]
    pub instance_commit_obligations: Vec<InstanceCommitObligation>,
    /// Instance-commit obligations attached to node ids.
    #[serde(skip)]
    pub instance_commit_obligation_by_node_id:
        IndexMap<GlobalNodeIdAny, InstanceCommitObligationId>,
    /// Instance-commit obligations attached to dynamic resolution candidate slots.
    #[serde(skip)]
    pub instance_commit_obligation_by_resolution_candidate:
        Vec<InstanceCommitResolutionCandidateAttachment>,
}

/// Associated comptime projection obligation collected during infer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssociatedComptimeProjectionObligation {
    /// The member expression that created this obligation.
    pub expression_id: LocalNodeId<Expression>,
    /// The projected member symbol.
    pub member_symbol: GlobalSymbolId,
    /// The projected member type id.
    pub member_type_id: LocalTypeId,
    /// The projection substitution environment captured at infer time.
    pub substitutions: HashMap<GlobalSymbolId, LocalTypeId>,
}

/// Infer-local identifier for one instance-commit obligation.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InstanceCommitObligationId(
    /// The instance-commit obligation id.
    pub u32,
);

impl InstanceCommitObligationId {
    /// Wrap an id as an instance-commit obligation id.
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

/// Instance-commit fact collected during infer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstanceCommitObligation {
    /// The target symbol for the instance.
    pub symbol_id: GlobalSymbolId,
    /// Canonical static arguments in declaration order.
    pub static_arguments: Vec<StaticArgument>,
    /// Canonical static parameter symbols aligned with arguments.
    pub static_parameter_symbols: Vec<GlobalSymbolId>,
    /// Number of inherited arguments at the front of `static_arguments`.
    pub inherited_static_argument_count: usize,
}

/// Instance-commit obligation attachment for one resolution candidate slot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstanceCommitResolutionCandidateAttachment {
    /// The resolution carrying the candidate list.
    pub resolution_id: LocalResolutionId,
    /// The dynamic candidate slot id within the resolution candidate list.
    pub candidate_slot: DynamicResolutionCandidateSlotId,
    /// The obligation that should attach to this candidate.
    pub obligation_id: InstanceCommitObligationId,
}

/// Infer-local dynamic candidate slot identifier.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DynamicResolutionCandidateSlotId(
    /// The dynamic candidate slot index.
    pub u32,
);

impl DynamicResolutionCandidateSlotId {
    /// Wrap one dynamic candidate slot index.
    pub fn new(index: u32) -> Self {
        Self(index)
    }
}

impl Default for InferTable {
    fn default() -> Self {
        Self {
            vars: Vec::new(),
            constraints: Vec::new(),
            var_by_node_id: IndexMap::new(),
            var_by_symbol_id: IndexMap::new(),
            var_by_type_parameter: IndexMap::new(),
            type_by_var_id: Vec::new(),
            cache_key_base: infer_table_cache_key_base_default(),
            cache_generation: 0,
            associated_comptime_projection_obligations: Vec::new(),
            instance_commit_obligations: Vec::new(),
            instance_commit_obligation_by_node_id: IndexMap::new(),
            instance_commit_obligation_by_resolution_candidate: Vec::new(),
        }
    }
}

impl InferTable {
    /// Allocate a new inference variable.
    pub fn new_var(&mut self, origin: InferOrigin, scope: InferScope) -> InferVarId {
        let id = InferVarId::new(self.vars.len() as u32);
        self.vars.push(InferVar::new(origin, scope));
        self.type_by_var_id.push(None);
        self.bump_cache_generation();
        id
    }

    /// Push a new constraint.
    #[inline]
    pub fn push_constraint(&mut self, constraint: Constraint) {
        self.constraints.push(constraint);
        self.bump_cache_generation();
    }

    /// Bind a type to an inference variable id.
    pub fn bind_type(&mut self, id: InferVarId, ty_id: LocalTypeId) {
        let index = id.0 as usize;
        if self.type_by_var_id.len() <= index {
            self.type_by_var_id.resize(index + 1, None);
        }
        self.type_by_var_id[index] = Some(ty_id);
        self.bump_cache_generation();
    }

    /// Get the type that wraps an inference variable.
    pub fn type_for_var(&self, id: InferVarId) -> Option<LocalTypeId> {
        self.type_by_var_id.get(id.0 as usize).copied().flatten()
    }

    /// Return the cache key for this table.
    pub fn cache_key(&self) -> u64 {
        self.cache_key_base ^ self.cache_generation
    }

    /// Record a mutation that impacts cacheable inference state.
    fn bump_cache_generation(&mut self) {
        self.cache_generation = self.cache_generation.wrapping_add(1);
    }

    /// Record one associated comptime projection obligation.
    pub fn push_associated_comptime_projection_obligation(
        &mut self,
        obligation: AssociatedComptimeProjectionObligation,
    ) {
        self.associated_comptime_projection_obligations
            .push(obligation);
    }

    /// Take associated comptime projection obligations.
    pub fn take_associated_comptime_projection_obligations(
        &mut self,
    ) -> Vec<AssociatedComptimeProjectionObligation> {
        std::mem::take(&mut self.associated_comptime_projection_obligations)
    }

    /// Upsert one instance-commit obligation and return its infer-local id.
    pub fn upsert_instance_commit_obligation(
        &mut self,
        obligation: InstanceCommitObligation,
    ) -> InstanceCommitObligationId {
        for (index, existing) in self.instance_commit_obligations.iter().enumerate() {
            if existing == &obligation {
                return InstanceCommitObligationId::new(index as u32);
            }
        }

        let obligation_id =
            InstanceCommitObligationId::new(self.instance_commit_obligations.len() as u32);
        self.instance_commit_obligations.push(obligation);
        obligation_id
    }

    /// Attach one instance-commit obligation to a node id.
    pub fn set_instance_commit_obligation_for_node(
        &mut self,
        node_id: GlobalNodeIdAny,
        obligation_id: InstanceCommitObligationId,
    ) {
        self.instance_commit_obligation_by_node_id
            .insert(node_id, obligation_id);
    }

    /// Return the instance-commit obligation id for a node.
    pub fn instance_commit_obligation_id_for_node(
        &self,
        node_id: GlobalNodeIdAny,
    ) -> Option<InstanceCommitObligationId> {
        self.instance_commit_obligation_by_node_id
            .get(&node_id)
            .copied()
    }

    /// Return one instance-commit obligation by id.
    pub fn instance_commit_obligation(
        &self,
        obligation_id: InstanceCommitObligationId,
    ) -> Option<&InstanceCommitObligation> {
        self.instance_commit_obligations
            .get(obligation_id.0 as usize)
    }

    /// Iterate instance-commit obligations with infer-local ids.
    pub fn iter_instance_commit_obligations(
        &self,
    ) -> impl Iterator<Item = (InstanceCommitObligationId, &InstanceCommitObligation)> {
        self.instance_commit_obligations
            .iter()
            .enumerate()
            .map(|(index, obligation)| (InstanceCommitObligationId::new(index as u32), obligation))
    }

    /// Iterate node attachments for instance-commit obligations.
    pub fn iter_instance_commit_obligation_nodes(
        &self,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, InstanceCommitObligationId)> + '_ {
        self.instance_commit_obligation_by_node_id
            .iter()
            .map(|(node_id, obligation_id)| (*node_id, *obligation_id))
    }

    /// Attach one instance-commit obligation to one dynamic resolution candidate slot.
    pub fn push_instance_commit_obligation_for_resolution_candidate(
        &mut self,
        resolution_id: LocalResolutionId,
        candidate_slot: DynamicResolutionCandidateSlotId,
        obligation_id: InstanceCommitObligationId,
    ) {
        self.instance_commit_obligation_by_resolution_candidate
            .push(InstanceCommitResolutionCandidateAttachment {
                resolution_id,
                candidate_slot,
                obligation_id,
            });
    }

    /// Iterate instance-commit dynamic-resolution candidate attachments.
    pub fn iter_instance_commit_obligation_resolution_candidates(
        &self,
    ) -> impl Iterator<Item = InstanceCommitResolutionCandidateAttachment> + '_ {
        self.instance_commit_obligation_by_resolution_candidate
            .iter()
            .cloned()
    }
}

/// Return a new cache key base for inference tables.
fn infer_table_cache_key_base_default() -> u64 {
    INFER_TABLE_CACHE_KEY_COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// Global counter for inference table cache keys.
static INFER_TABLE_CACHE_KEY_COUNTER: AtomicU64 = AtomicU64::new(1);
