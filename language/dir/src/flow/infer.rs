use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use indexmap::{IndexMap, IndexSet};

use crate::{
    Addressability, Declarator, Expression, GlobalNodeIdAny, GlobalSymbolId, InferVarId,
    LocalInstanceId, LocalNodeId, LocalTypeId, Resolution, StaticArgument, StaticKey,
    VarianceBound,
};

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
        generic_arguments: Vec<LocalTypeId>,
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
#[derive(Debug)]
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
    pub cache_key_base: u64,
    /// Mutation generation for this table.
    pub cache_generation: u64,

    /// Associated comptime projection obligations collected during infer.
    associated_comptime_projection_obligations: Vec<AssociatedComptimeProjectionObligation>,
    /// Type relation obligations collected during infer.
    type_relation_obligations: IndexSet<TypeRelationObligation>,
    /// Missing-member obligations collected during infer.
    missing_member_obligations: IndexSet<MissingMemberObligation>,
    /// Direct-binding commit intents collected during infer.
    direct_binding_value_commit_intent_by_symbol:
        IndexMap<GlobalSymbolId, DirectBindingValueCommitEntry>,
    /// Provisional resolutions recorded during infer by node id.
    pub provisional_resolution_by_node_id: IndexMap<GlobalNodeIdAny, Resolution>,
    /// Provisional instance attachments recorded during infer by node id.
    pub provisional_instance_by_node_id: IndexMap<GlobalNodeIdAny, LocalInstanceId>,
    /// Infer-owned expression type cache by node id.
    pub inferred_type_by_node_id: IndexMap<GlobalNodeIdAny, LocalTypeId>,
    /// Infer-owned expression addressability cache by node id.
    pub addressability_by_node_id: IndexMap<GlobalNodeIdAny, Addressability>,

    /// Instance-commit obligations collected during infer.
    instance_commit_obligations: Vec<InstanceCommitObligation>,
    /// Instance-commit obligation ids indexed by symbol.
    instance_commit_obligation_ids_by_symbol:
        IndexMap<GlobalSymbolId, Vec<InstanceCommitObligationId>>,
    /// Instance-commit obligations attached to node ids.
    instance_commit_obligation_by_node_id: IndexMap<GlobalNodeIdAny, InstanceCommitObligationId>,
    /// Instance-commit obligations attached to dynamic resolution candidate slots.
    instance_commit_obligation_by_resolution_candidate:
        Vec<InstanceCommitResolutionCandidateAttachment>,
}

/// Associated comptime projection obligation collected during infer.
#[derive(Debug, Clone)]
pub struct AssociatedComptimeProjectionObligation {
    /// The member expression that created this obligation.
    pub expression_id: LocalNodeId<Expression>,
    /// The projected member symbol, when infer resolved it eagerly.
    pub member_symbol: Option<GlobalSymbolId>,
    /// The projected member type id.
    pub member_type_id: LocalTypeId,
    /// Receiver static arguments captured at the projection site.
    pub receiver_arguments: Vec<StaticArgument>,
    /// The projection substitution environment captured at infer time.
    pub substitutions: HashMap<GlobalSymbolId, LocalTypeId>,
}

/// Diagnostic to emit when a type relation obligation fails after infer convergence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TypeRelationObligationDiagnostic {
    /// Emit an unassignable type diagnostic.
    UnassignableType,
    /// Emit an unsatisfied type diagnostic.
    UnsatisfiedType,
}

/// Operand source for a deferred type relation obligation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeRelationObligationOperands {
    /// Use captured type ids recorded during infer.
    CapturedTypes {
        /// The target type id for assignability.
        target_type_id: LocalTypeId,
        /// The source type id for assignability.
        source_type_id: LocalTypeId,
    },
    /// Read operand inferred types from expression nodes after solve convergence.
    ExpressionOperands {
        /// The target expression id.
        target_expression_id: GlobalNodeIdAny,
        /// The source expression id.
        source_expression_id: GlobalNodeIdAny,
    },
    /// Use a captured target type and a source expression resolved after solve convergence.
    CapturedTargetTypeAndSourceExpression {
        /// The target type id for assignability.
        target_type_id: LocalTypeId,
        /// The source expression id.
        source_expression_id: GlobalNodeIdAny,
    },
}

/// Type relation obligation collected during infer and checked after solve convergence.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeRelationObligation {
    /// The source node that owns this relation check.
    pub source_node_id: GlobalNodeIdAny,
    /// The deferred operand source for this relation.
    pub operands: TypeRelationObligationOperands,
    /// The diagnostic to emit when the relation fails.
    pub diagnostic: TypeRelationObligationDiagnostic,
}

/// Missing-member obligation collected during infer and checked after solve convergence.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MissingMemberObligation {
    /// The member expression that created this obligation.
    pub expression_id: GlobalNodeIdAny,
    /// The receiver expression used to re-resolve the lookup.
    pub receiver_expression_id: GlobalNodeIdAny,
    /// The receiver type observed when infer deferred lookup.
    pub receiver_type_id: LocalTypeId,
    /// The member key to validate after convergence.
    pub member_key: StaticKey,
}

/// Direct-binding commit entry collected during infer.
#[derive(Debug, Clone, PartialEq)]
struct DirectBindingValueCommitEntry {
    /// The direct binding symbol to commit.
    pub symbol_id: GlobalSymbolId,
    /// The declarator that owns the binding.
    pub declarator_id: LocalNodeId<Declarator>,
    /// The initializer expression for the binding.
    pub value_id: LocalNodeId<Expression>,
}

/// Infer-local identifier for one instance-commit obligation.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

/// Instance-commit record collected during infer.
#[derive(Debug, Clone, PartialEq)]
pub struct InstanceCommitObligation {
    /// The target symbol for the instance.
    pub symbol_id: GlobalSymbolId,
    /// Canonical static arguments in declaration order.
    pub generic_arguments: Vec<StaticArgument>,
    /// Canonical generic parameter symbols aligned with arguments.
    pub generic_parameter_symbols: Vec<GlobalSymbolId>,
    /// Number of inherited arguments at the front of `generic_arguments`.
    pub inherited_static_argument_count: usize,
}

/// Instance-commit obligation attachment for one resolution candidate slot.
#[derive(Debug, Clone, PartialEq)]
pub struct InstanceCommitResolutionCandidateAttachment {
    /// The node carrying the resolution candidate list.
    pub node_id: GlobalNodeIdAny,
    /// The dynamic candidate slot id within the resolution candidate list.
    pub candidate_slot: DynamicResolutionCandidateSlotId,
    /// The obligation that should attach to this candidate.
    pub obligation_id: InstanceCommitObligationId,
}

/// Infer-local dynamic candidate slot identifier.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
            type_relation_obligations: IndexSet::new(),
            missing_member_obligations: IndexSet::new(),
            direct_binding_value_commit_intent_by_symbol: IndexMap::new(),
            provisional_resolution_by_node_id: IndexMap::new(),
            provisional_instance_by_node_id: IndexMap::new(),
            inferred_type_by_node_id: IndexMap::new(),
            addressability_by_node_id: IndexMap::new(),
            instance_commit_obligations: Vec::new(),
            instance_commit_obligation_ids_by_symbol: IndexMap::new(),
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

    /// Record one type relation obligation.
    pub fn push_type_relation_obligation(&mut self, obligation: TypeRelationObligation) {
        self.type_relation_obligations.insert(obligation);
    }

    /// Take type relation obligations.
    pub fn take_type_relation_obligations(&mut self) -> Vec<TypeRelationObligation> {
        std::mem::take(&mut self.type_relation_obligations)
            .into_iter()
            .collect()
    }

    /// Record one missing-member obligation.
    pub fn push_missing_member_obligation(&mut self, obligation: MissingMemberObligation) {
        self.missing_member_obligations.insert(obligation);
    }

    /// Take missing-member obligations.
    pub fn take_missing_member_obligations(&mut self) -> Vec<MissingMemberObligation> {
        std::mem::take(&mut self.missing_member_obligations)
            .into_iter()
            .collect()
    }

    /// Return true when a missing-member obligation exists for one expression node.
    pub fn has_missing_member_obligation_for_expression(
        &self,
        expression_id: GlobalNodeIdAny,
    ) -> bool {
        self.missing_member_obligations
            .iter()
            .any(|obligation| obligation.expression_id == expression_id)
    }

    /// Upsert one direct-binding commit intent by symbol.
    pub fn upsert_direct_binding_value_commit_intent(
        &mut self,
        symbol_id: GlobalSymbolId,
        declarator_id: LocalNodeId<Declarator>,
        value_id: LocalNodeId<Expression>,
    ) {
        self.direct_binding_value_commit_intent_by_symbol.insert(
            symbol_id,
            DirectBindingValueCommitEntry {
                symbol_id,
                declarator_id,
                value_id,
            },
        );
    }

    /// Iterate direct-binding commit intents.
    pub fn iter_direct_binding_value_commit_intents(
        &self,
    ) -> impl Iterator<
        Item = (
            GlobalSymbolId,
            LocalNodeId<Declarator>,
            LocalNodeId<Expression>,
        ),
    > + '_ {
        self.direct_binding_value_commit_intent_by_symbol
            .values()
            .map(|intent| (intent.symbol_id, intent.declarator_id, intent.value_id))
    }

    /// Record one provisional resolution for one node.
    pub fn set_provisional_resolution_for_node(
        &mut self,
        node_id: GlobalNodeIdAny,
        resolution: Resolution,
    ) {
        self.provisional_resolution_by_node_id
            .insert(node_id, resolution);
    }

    /// Return one provisional resolution for one node.
    pub fn provisional_resolution_for_node(&self, node_id: GlobalNodeIdAny) -> Option<&Resolution> {
        self.provisional_resolution_by_node_id.get(&node_id)
    }

    /// Iterate provisional resolutions by node.
    pub fn iter_provisional_resolution_nodes(
        &self,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, &Resolution)> {
        self.provisional_resolution_by_node_id
            .iter()
            .map(|(node_id, resolution)| (*node_id, resolution))
    }

    /// Record one provisional instance attachment for one node.
    pub fn set_provisional_instance_for_node(
        &mut self,
        node_id: GlobalNodeIdAny,
        instance_id: LocalInstanceId,
    ) {
        self.provisional_instance_by_node_id
            .insert(node_id, instance_id);
    }

    /// Return one provisional instance attachment for one node.
    pub fn provisional_instance_for_node(
        &self,
        node_id: GlobalNodeIdAny,
    ) -> Option<LocalInstanceId> {
        self.provisional_instance_by_node_id.get(&node_id).copied()
    }

    /// Iterate provisional instance attachments by node.
    pub fn iter_provisional_instance_nodes(
        &self,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, LocalInstanceId)> + '_ {
        self.provisional_instance_by_node_id
            .iter()
            .map(|(node_id, instance_id)| (*node_id, *instance_id))
    }

    /// Record one infer-owned expression type for one node.
    pub fn set_inferred_type_for_node(&mut self, node_id: GlobalNodeIdAny, type_id: LocalTypeId) {
        self.inferred_type_by_node_id.insert(node_id, type_id);
    }

    /// Return one infer-owned expression type for one node.
    pub fn inferred_type_for_node(&self, node_id: GlobalNodeIdAny) -> Option<LocalTypeId> {
        self.inferred_type_by_node_id.get(&node_id).copied()
    }

    /// Iterate infer-owned expression types by node.
    pub fn iter_inferred_type_nodes(
        &self,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, LocalTypeId)> + '_ {
        self.inferred_type_by_node_id
            .iter()
            .map(|(node_id, type_id)| (*node_id, *type_id))
    }

    /// Record one infer-owned expression addressability for one node.
    pub fn set_addressability_for_node(
        &mut self,
        node_id: GlobalNodeIdAny,
        addressability: Addressability,
    ) {
        self.addressability_by_node_id
            .insert(node_id, addressability);
    }

    /// Return one infer-owned expression addressability for one node.
    pub fn addressability_for_node(&self, node_id: GlobalNodeIdAny) -> Option<Addressability> {
        self.addressability_by_node_id.get(&node_id).copied()
    }

    /// Iterate infer-owned expression addressability by node.
    pub fn iter_addressability_nodes(
        &self,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, Addressability)> + '_ {
        self.addressability_by_node_id
            .iter()
            .map(|(node_id, addressability)| (*node_id, *addressability))
    }

    /// Upsert one instance-commit obligation and return its infer-local id.
    pub fn upsert_instance_commit_obligation(
        &mut self,
        obligation: InstanceCommitObligation,
    ) -> InstanceCommitObligationId {
        let symbol_id = obligation.symbol_id;
        let symbol_obligation_ids = self
            .instance_commit_obligation_ids_by_symbol
            .get(&symbol_id);
        if let Some(symbol_obligation_ids) = symbol_obligation_ids {
            for obligation_id in symbol_obligation_ids {
                if let Some(existing) = self
                    .instance_commit_obligations
                    .get(obligation_id.0 as usize)
                    && existing == &obligation
                {
                    return *obligation_id;
                }
            }
        }

        let obligation_id =
            InstanceCommitObligationId::new(self.instance_commit_obligations.len() as u32);
        self.instance_commit_obligations.push(obligation);
        self.instance_commit_obligation_ids_by_symbol
            .entry(symbol_id)
            .or_default()
            .push(obligation_id);
        obligation_id
    }

    /// Return the number of instance-commit obligations.
    pub fn instance_commit_obligation_count(&self) -> usize {
        self.instance_commit_obligations.len()
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
        node_id: GlobalNodeIdAny,
        candidate_slot: DynamicResolutionCandidateSlotId,
        obligation_id: InstanceCommitObligationId,
    ) {
        self.instance_commit_obligation_by_resolution_candidate
            .push(InstanceCommitResolutionCandidateAttachment {
                node_id,
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
