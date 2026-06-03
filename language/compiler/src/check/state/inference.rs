use destack_dir as dir;
use indexmap::IndexMap;
use smallvec::SmallVec;

use crate::check::{
    CallDecision, CheckState, Constraint, ConstructDecision, Dump, DumpContext, GenericApplication,
    GenericApplicationKey, GenericArgument, GenericInduction, GenericInductionRoot,
    GenericInductionSlot, GenericSlot, GenericSlotId, GenericTemplate, IdentityDecision,
    LayoutDecision, MemberDecision, Obligation, OperatorDecision, PatternDecision,
    ReceiverResolution, Solution, StaticOperand, Term, TermId, TermTable, TypeOperand, Variable,
    VariableId,
};
use crate::{CompilerError, CompilerResult};

/// Segmented inference graph for one checked component.
#[derive(Debug)]
pub(in crate::check) struct InferenceTable {
    /// Ordered inference segments.
    pub(in crate::check::state) segments: Vec<InferenceSegment>,
}

/// Live speculative inference segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct InferenceProbe {
    /// The probed segment depth.
    depth: usize,
}

/// One permanent or speculative inference segment.
#[derive(Debug)]
pub(in crate::check) struct InferenceSegment {
    /// Component term segment.
    terms: TermTable,
    /// Variables allocated in this segment.
    variables: Vec<Variable>,
    /// Variable constraints in collection order.
    constraints: Vec<Constraint>,
    /// Component post-solve obligations.
    obligations: Vec<Obligation>,

    /// Type operands that must be assignable to each variable.
    type_lower_bounds: IndexMap<VariableId, Vec<TypeOperand>>,
    /// Type operands that each variable must be assignable to.
    type_upper_bounds: IndexMap<VariableId, Vec<TypeOperand>>,
    /// Static operands that must be assignable to each variable.
    static_lower_bounds: IndexMap<VariableId, Vec<StaticOperand>>,
    /// Static operands that each variable must be assignable to.
    static_upper_bounds: IndexMap<VariableId, Vec<StaticOperand>>,
    /// Solved variable values.
    solutions: IndexMap<VariableId, Solution>,

    /// Generic templates keyed by owning symbol.
    generic_templates: IndexMap<dir::GlobalSymbolId, GenericTemplate>,
    /// Generic applications keyed by source node and owner.
    generic_applications: IndexMap<GenericApplicationKey, GenericApplication>,
    /// Generic slot ids keyed by parameter symbol.
    generic_slots_by_symbol: IndexMap<dir::GlobalSymbolId, GenericSlotId>,
    /// Generic slots keyed by slot id.
    generic_slots_by_id: IndexMap<GenericSlotId, GenericSlot>,
    /// Declaration operands that can induce owner generics.
    generic_induction_roots: Vec<GenericInductionRoot>,
    /// Variables that can induce owner generics.
    generic_inductions: IndexMap<VariableId, GenericInductionSlot>,

    /// Runtime calls resolved or rejected by solve.
    calls: IndexMap<dir::GlobalNodeIdAny, CallDecision>,
    /// Runtime construct expressions resolved or rejected by solve.
    constructs: IndexMap<dir::GlobalNodeIdAny, ConstructDecision>,
    /// Runtime operators resolved or rejected by solve.
    operators: IndexMap<dir::GlobalNodeIdAny, OperatorDecision>,
    /// Runtime identity checks resolved or rejected by solve.
    identities: IndexMap<dir::GlobalNodeIdAny, IdentityDecision>,
    /// Layout queries resolved or rejected by solve.
    layouts: IndexMap<dir::GlobalNodeIdAny, LayoutDecision>,
    /// Runtime members resolved or rejected by solve.
    members: IndexMap<dir::GlobalNodeIdAny, MemberDecision>,
    /// Patterns resolved or rejected by check.
    patterns: IndexMap<dir::GlobalNodeIdAny, PatternDecision>,
    /// Contextual receivers resolved by check.
    receivers: IndexMap<dir::GlobalNodeIdAny, ReceiverResolution>,
    /// Lexical names resolved by check.
    names: IndexMap<dir::GlobalNodeIdAny, dir::NameResolution>,
}

impl InferenceTable {
    /// Create an empty inference graph.
    pub(in crate::check) fn new() -> Self {
        Self {
            segments: vec![InferenceSegment::new()],
        }
    }

    /// Return the current mutable inference segment.
    pub(in crate::check) fn current_mut(&mut self) -> &mut InferenceSegment {
        self.segments
            .last_mut()
            .unwrap_or_else(|| panic!("inference table has no current segment"))
    }

    /// Begin one speculative inference segment.
    pub(in crate::check) fn begin_probe(&mut self) -> InferenceProbe {
        let depth = self.segments.len();
        self.segments.push(InferenceSegment::new());

        InferenceProbe { depth }
    }

    /// Merge the current speculative segment into its parent.
    pub(in crate::check) fn commit_probe(&mut self, probe: InferenceProbe) -> CompilerResult<()> {
        if probe.depth + 1 != self.segments.len() {
            panic!("inference probes must be committed in LIFO order");
        }

        let segment = self
            .segments
            .pop()
            .unwrap_or_else(|| panic!("inference probe segment is missing"));
        let parent = self
            .segments
            .last_mut()
            .unwrap_or_else(|| panic!("inference probe has no parent segment"));

        parent.merge(segment)?;

        Ok(())
    }

    /// Drop the current speculative segment.
    pub(in crate::check) fn drop_probe(&mut self, probe: InferenceProbe) {
        if probe.depth + 1 != self.segments.len() {
            panic!("inference probes must be dropped in LIFO order");
        }

        self.segments.pop();
    }

    /// Push one term into the current inference segment.
    pub(in crate::check) fn push_term<T: Term>(&mut self, term: T) -> TermId<T> {
        let id = self.term_count::<T>() as u32;
        let id = TermId::new(id);

        self.current_mut().terms.push(term);

        id
    }

    /// Return one term by id.
    pub(in crate::check) fn term<T: Term>(&self, id: TermId<T>) -> &T {
        let mut base = 0;

        for segment in &self.segments {
            let terms = T::arena(&segment.terms);
            let end = base + terms.len();
            if id.index() < end {
                return &terms[id.index() - base];
            }

            base = end;
        }

        panic!("check term {id:?} is not allocated")
    }

    /// Return the total number of stored terms.
    pub(in crate::check) fn term_count_total(&self) -> usize {
        self.segments
            .iter()
            .map(|segment| segment.terms.len())
            .sum()
    }

    /// Return the total number of stored terms for one term kind.
    fn term_count<T: Term>(&self) -> usize {
        self.segments
            .iter()
            .map(|segment| T::arena(&segment.terms).len())
            .sum()
    }

    /// Push one variable into the current inference segment.
    pub(in crate::check) fn push_variable(&mut self, variable: Variable) {
        self.current_mut().variables.push(variable);
    }

    /// Return the number of allocated variables.
    pub(in crate::check) fn variable_count(&self) -> usize {
        self.segments
            .iter()
            .map(|segment| segment.variables.len())
            .sum()
    }

    /// Return one variable by allocation index.
    pub(in crate::check) fn variable_at(&self, index: usize) -> &Variable {
        let mut base = 0;

        for segment in &self.segments {
            let end = base + segment.variables.len();
            if index < end {
                return &segment.variables[index - base];
            }

            base = end;
        }

        panic!("check variable {index} is not allocated")
    }

    /// Push one constraint into the current segment.
    pub(in crate::check) fn push_constraint(&mut self, constraint: Constraint) {
        self.current_mut().constraints.push(constraint);
    }

    /// Return the total number of constraints.
    pub(in crate::check) fn constraint_count(&self) -> usize {
        self.segments
            .iter()
            .map(|segment| segment.constraints.len())
            .sum()
    }

    /// Return one constraint by component order.
    pub(in crate::check) fn constraint(&self, index: usize) -> &Constraint {
        let mut base = 0;

        for segment in &self.segments {
            let end = base + segment.constraints.len();
            if index < end {
                return &segment.constraints[index - base];
            }

            base = end;
        }

        panic!("check constraint {index} is not allocated")
    }

    /// Iterate constraints in segment order.
    pub(in crate::check) fn constraints(&self) -> impl Iterator<Item = &Constraint> {
        self.segments
            .iter()
            .flat_map(|segment| segment.constraints.iter())
    }

    /// Push one obligation into the current segment.
    pub(in crate::check) fn push_obligation(&mut self, obligation: Obligation) {
        self.current_mut().obligations.push(obligation);
    }

    /// Iterate obligations in segment order.
    pub(in crate::check) fn obligations(&self) -> impl Iterator<Item = &Obligation> {
        self.segments
            .iter()
            .flat_map(|segment| segment.obligations.iter())
    }

    /// Return the total number of obligations.
    pub(in crate::check) fn obligation_count(&self) -> usize {
        self.segments
            .iter()
            .map(|segment| segment.obligations.len())
            .sum()
    }

    /// Push one induction root into the current segment.
    pub(in crate::check) fn push_generic_induction_root(&mut self, root: GenericInductionRoot) {
        self.current_mut().generic_induction_roots.push(root);
    }

    /// Iterate generic induction roots in component order.
    pub(in crate::check) fn generic_induction_roots(
        &self,
    ) -> impl Iterator<Item = &GenericInductionRoot> {
        self.segments
            .iter()
            .flat_map(|segment| segment.generic_induction_roots.iter())
    }

    /// Mark one variable as able to induce an owner generic.
    pub(in crate::check) fn insert_generic_induction(&mut self, variable: GenericInduction) {
        let previous = self
            .current_mut()
            .generic_inductions
            .insert(variable.variable, variable.slot);

        if previous.is_some() {
            panic!(
                "check variable {:?} already has generic induction",
                variable.variable
            );
        }
    }

    /// Return the active induced generic slot for one variable.
    pub(in crate::check) fn generic_induction_slot(
        &self,
        variable: VariableId,
    ) -> Option<GenericInductionSlot> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.generic_inductions.get(&variable).cloned())
    }

    /// Return generic slots in template order.
    pub(in crate::check) fn generic_slots(
        &self,
    ) -> impl Iterator<Item = (GenericSlotId, &GenericSlot)> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.generic_templates.values())
            .flat_map(|template| template.slots.iter())
            .filter_map(|slot_id| self.generic_slot_entry(*slot_id))
    }

    /// Return generic slots owned by one symbol.
    pub(in crate::check) fn generic_slots_for_owner(
        &self,
        owner: dir::GlobalSymbolId,
    ) -> impl Iterator<Item = (GenericSlotId, &GenericSlot)> + '_ {
        self.segments
            .iter()
            .flat_map(move |segment| segment.generic_templates.get(&owner))
            .flat_map(|template| template.slots.iter().copied())
            .filter_map(|slot_id| self.generic_slot_entry(slot_id))
    }

    /// Return the active generic template for one owner.
    pub(in crate::check) fn generic_template_for_owner(
        &self,
        owner: dir::GlobalSymbolId,
    ) -> Option<&GenericTemplate> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.generic_templates.get(&owner))
    }

    /// Return one active generic slot by id.
    pub(in crate::check) fn generic_slot_by_id(
        &self,
        slot_id: GenericSlotId,
    ) -> Option<&GenericSlot> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.generic_slots_by_id.get(&slot_id))
    }

    /// Return one required generic slot by id.
    pub(in crate::check) fn generic_slot(&self, slot_id: GenericSlotId) -> &GenericSlot {
        self.generic_slot_by_id(slot_id)
            .unwrap_or_else(|| panic!("generic slot {slot_id:?} does not exist"))
    }

    /// Return one active generic application.
    pub(in crate::check) fn generic_application(
        &self,
        key: GenericApplicationKey,
    ) -> Option<&GenericApplication> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.generic_applications.get(&key))
    }

    /// Return one active generic application argument.
    pub(in crate::check) fn generic_application_argument(
        &self,
        source: dir::GlobalNodeIdAny,
        owner: dir::GlobalSymbolId,
        index: dir::GenericSlotIndex,
    ) -> Option<GenericArgument> {
        let key = GenericApplicationKey { source, owner };

        self.generic_application(key)
            .and_then(|application| application.arguments.get(index.0 as usize))
            .cloned()
    }

    /// Insert one generic application into the current segment.
    pub(in crate::check) fn insert_generic_application(
        &mut self,
        key: GenericApplicationKey,
        arguments: SmallVec<[GenericArgument; 2]>,
    ) -> GenericApplication {
        if let Some(application) = self.generic_application(key) {
            if application.arguments != arguments {
                panic!("check generic application {key:?} already has different arguments");
            }

            return application.clone();
        }

        let application = GenericApplication {
            owner: key.owner,
            arguments,
        };

        self.current_mut()
            .generic_applications
            .insert(key, application.clone());

        application
    }

    /// Insert one generic slot into the current segment.
    pub(in crate::check) fn insert_generic_slot(&mut self, generic: GenericSlot) {
        let owner = generic.slot().owner;
        let key = generic.slot().key;
        let slot_id = generic.slot().id();
        if self.generic_slot_by_id(slot_id).is_some() {
            panic!("generic slot {slot_id:?} is already inserted");
        }

        let segment = self.current_mut();
        segment.generic_slots_by_id.insert(slot_id, generic);
        segment
            .generic_templates
            .entry(owner)
            .or_insert_with(|| GenericTemplate::new(owner))
            .slots
            .push(slot_id);
        if let dir::GenericSlotKey::Symbol(symbol) = key {
            segment.generic_slots_by_symbol.insert(symbol, slot_id);
        }
    }

    /// Return the active generic slot id declared by one symbol.
    pub(in crate::check) fn generic_slot_id_for_symbol(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<GenericSlotId> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.generic_slots_by_symbol.get(&symbol).copied())
    }

    /// Return one active generic slot mutably.
    pub(in crate::check) fn generic_slot_by_id_mut(
        &mut self,
        slot_id: GenericSlotId,
    ) -> &mut GenericSlot {
        self.segments
            .iter_mut()
            .rev()
            .find_map(|segment| segment.generic_slots_by_id.get_mut(&slot_id))
            .unwrap_or_else(|| panic!("generic slot {slot_id:?} does not exist"))
    }

    /// Return the next generic slot index for one owner.
    pub(in crate::check) fn next_generic_slot_index(
        &self,
        owner: dir::GlobalSymbolId,
    ) -> dir::GenericSlotIndex {
        let index = self
            .segments
            .iter()
            .filter_map(|segment| segment.generic_templates.get(&owner))
            .map(|template| template.slots.len())
            .sum::<usize>();

        dir::GenericSlotIndex::new(index as u32)
    }

    /// Return active lower type bounds for one variable.
    pub(in crate::check) fn lower_type_bounds(&self, variable: VariableId) -> Vec<TypeOperand> {
        self.segments
            .iter()
            .flat_map(|segment| {
                segment
                    .type_lower_bounds
                    .get(&variable)
                    .into_iter()
                    .flatten()
                    .copied()
            })
            .collect()
    }

    /// Return whether one variable has active lower type bounds.
    pub(in crate::check) fn has_lower_type_bounds(&self, variable: VariableId) -> bool {
        self.segments
            .iter()
            .any(|segment| segment.type_lower_bounds.contains_key(&variable))
    }

    /// Return active upper type bounds for one variable.
    pub(in crate::check) fn upper_type_bounds(&self, variable: VariableId) -> Vec<TypeOperand> {
        self.segments
            .iter()
            .flat_map(|segment| {
                segment
                    .type_upper_bounds
                    .get(&variable)
                    .into_iter()
                    .flatten()
                    .copied()
            })
            .collect()
    }

    /// Return active lower static bounds for one variable.
    pub(in crate::check) fn lower_static_bounds(&self, variable: VariableId) -> Vec<StaticOperand> {
        self.segments
            .iter()
            .flat_map(|segment| {
                segment
                    .static_lower_bounds
                    .get(&variable)
                    .into_iter()
                    .flatten()
                    .copied()
            })
            .collect()
    }

    /// Return whether one variable has active lower static bounds.
    pub(in crate::check) fn has_lower_static_bounds(&self, variable: VariableId) -> bool {
        self.segments
            .iter()
            .any(|segment| segment.static_lower_bounds.contains_key(&variable))
    }

    /// Return active upper static bounds for one variable.
    pub(in crate::check) fn upper_static_bounds(&self, variable: VariableId) -> Vec<StaticOperand> {
        self.segments
            .iter()
            .flat_map(|segment| {
                segment
                    .static_upper_bounds
                    .get(&variable)
                    .into_iter()
                    .flatten()
                    .copied()
            })
            .collect()
    }

    /// Return the total number of lower type bounds.
    pub(in crate::check) fn lower_type_bound_count(&self) -> usize {
        self.segments
            .iter()
            .flat_map(|segment| segment.type_lower_bounds.values())
            .map(Vec::len)
            .sum()
    }

    /// Return the total number of upper type bounds.
    pub(in crate::check) fn upper_type_bound_count(&self) -> usize {
        self.segments
            .iter()
            .flat_map(|segment| segment.type_upper_bounds.values())
            .map(Vec::len)
            .sum()
    }

    /// Return the total number of lower static bounds.
    pub(in crate::check) fn lower_static_bound_count(&self) -> usize {
        self.segments
            .iter()
            .flat_map(|segment| segment.static_lower_bounds.values())
            .map(Vec::len)
            .sum()
    }

    /// Return the total number of upper static bounds.
    pub(in crate::check) fn upper_static_bound_count(&self) -> usize {
        self.segments
            .iter()
            .flat_map(|segment| segment.static_upper_bounds.values())
            .map(Vec::len)
            .sum()
    }

    /// Insert one lower type bound into the current segment.
    pub(in crate::check) fn insert_lower_type_bound(
        &mut self,
        variable: VariableId,
        bound: TypeOperand,
    ) -> bool {
        if self.contains_bound(variable, bound, |segment| &segment.type_lower_bounds) {
            return false;
        }

        self.current_mut()
            .type_lower_bounds
            .entry(variable)
            .or_default()
            .push(bound);

        true
    }

    /// Insert one upper type bound into the current segment.
    pub(in crate::check) fn insert_upper_type_bound(
        &mut self,
        variable: VariableId,
        bound: TypeOperand,
    ) -> bool {
        if self.contains_bound(variable, bound, |segment| &segment.type_upper_bounds) {
            return false;
        }

        self.current_mut()
            .type_upper_bounds
            .entry(variable)
            .or_default()
            .push(bound);

        true
    }

    /// Insert one lower static bound into the current segment.
    pub(in crate::check) fn insert_lower_static_bound(
        &mut self,
        variable: VariableId,
        bound: StaticOperand,
    ) -> bool {
        if self.contains_bound(variable, bound, |segment| &segment.static_lower_bounds) {
            return false;
        }

        self.current_mut()
            .static_lower_bounds
            .entry(variable)
            .or_default()
            .push(bound);

        true
    }

    /// Insert one upper static bound into the current segment.
    pub(in crate::check) fn insert_upper_static_bound(
        &mut self,
        variable: VariableId,
        bound: StaticOperand,
    ) -> bool {
        if self.contains_bound(variable, bound, |segment| &segment.static_upper_bounds) {
            return false;
        }

        self.current_mut()
            .static_upper_bounds
            .entry(variable)
            .or_default()
            .push(bound);

        true
    }

    /// Return the active solution for one variable.
    pub(in crate::check) fn variable_solution(&self, variable: VariableId) -> Option<Solution> {
        for segment in self.segments.iter().rev() {
            if let Some(solution) = segment.solutions.get(&variable) {
                return Some(*solution);
            }
        }

        None
    }

    /// Set one variable solution in the current segment.
    pub(in crate::check) fn set_variable_solution(
        &mut self,
        variable: VariableId,
        solution: Solution,
    ) -> CompilerResult<()> {
        if let Some(existing) = self.variable_solution(variable) {
            return Err(CompilerError::Internal {
                message: format!(
                    "check variable {variable:?} already has solution {existing:?}, got {solution:?}"
                ),
            });
        }

        self.current_mut().solutions.insert(variable, solution);

        Ok(())
    }

    /// Return the total number of solved variables.
    pub(in crate::check) fn solution_count(&self) -> usize {
        self.segments
            .iter()
            .map(|segment| segment.solutions.len())
            .sum()
    }

    /// Return active call decision.
    pub(in crate::check) fn call(&self, source: dir::GlobalNodeIdAny) -> Option<CallDecision> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.calls.get(&source).cloned())
    }

    /// Select one call decision in the current segment.
    pub(in crate::check) fn select_call(
        &mut self,
        source: dir::GlobalNodeIdAny,
        decision: CallDecision,
    ) {
        if !self.is_new_selection(source, "call", &decision, |segment| &segment.calls) {
            return;
        }

        self.current_mut().calls.insert(source, decision);
    }

    /// Return active construct decision.
    pub(in crate::check) fn construct(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> Option<ConstructDecision> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.constructs.get(&source).cloned())
    }

    /// Select one construct decision in the current segment.
    pub(in crate::check) fn select_construct(
        &mut self,
        source: dir::GlobalNodeIdAny,
        decision: ConstructDecision,
    ) {
        if !self.is_new_selection(source, "construct", &decision, |segment| {
            &segment.constructs
        }) {
            return;
        }

        self.current_mut().constructs.insert(source, decision);
    }

    /// Return active operator decision.
    pub(in crate::check) fn operator(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> Option<OperatorDecision> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.operators.get(&source).cloned())
    }

    /// Select one operator decision in the current segment.
    pub(in crate::check) fn select_operator(
        &mut self,
        source: dir::GlobalNodeIdAny,
        decision: OperatorDecision,
    ) {
        if !self.is_new_selection(source, "operator", &decision, |segment| &segment.operators) {
            return;
        }

        self.current_mut().operators.insert(source, decision);
    }

    /// Return active identity decision.
    pub(in crate::check) fn identity(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> Option<IdentityDecision> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.identities.get(&source).cloned())
    }

    /// Select one identity decision in the current segment.
    pub(in crate::check) fn select_identity(
        &mut self,
        source: dir::GlobalNodeIdAny,
        decision: IdentityDecision,
    ) {
        if !self.is_new_selection(source, "identity", &decision, |segment| &segment.identities) {
            return;
        }

        self.current_mut().identities.insert(source, decision);
    }

    /// Return active layout decision.
    pub(in crate::check) fn layout(&self, source: dir::GlobalNodeIdAny) -> Option<LayoutDecision> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.layouts.get(&source).cloned())
    }

    /// Select one layout decision in the current segment.
    pub(in crate::check) fn select_layout(
        &mut self,
        source: dir::GlobalNodeIdAny,
        decision: LayoutDecision,
    ) {
        if !self.is_new_selection(source, "layout", &decision, |segment| &segment.layouts) {
            return;
        }

        self.current_mut().layouts.insert(source, decision);
    }

    /// Return active member decision.
    pub(in crate::check) fn member(&self, source: dir::GlobalNodeIdAny) -> Option<MemberDecision> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.members.get(&source).cloned())
    }

    /// Select one member decision in the current segment.
    pub(in crate::check) fn select_member(
        &mut self,
        source: dir::GlobalNodeIdAny,
        decision: MemberDecision,
    ) {
        if !self.is_new_selection(source, "member", &decision, |segment| &segment.members) {
            return;
        }

        self.current_mut().members.insert(source, decision);
    }

    /// Select one pattern decision in the current segment.
    pub(in crate::check) fn select_pattern(
        &mut self,
        source: dir::GlobalNodeIdAny,
        decision: PatternDecision,
    ) {
        if !self.is_new_selection(source, "pattern", &decision, |segment| &segment.patterns) {
            return;
        }

        self.current_mut().patterns.insert(source, decision);
    }

    /// Select one receiver resolution in the current segment.
    pub(in crate::check) fn select_receiver(&mut self, receiver: ReceiverResolution) {
        if !self.is_new_selection(receiver.source, "receiver", &receiver, |segment| {
            &segment.receivers
        }) {
            return;
        }

        self.current_mut()
            .receivers
            .insert(receiver.source, receiver);
    }

    /// Return active name resolution.
    pub(in crate::check) fn name(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> Option<dir::NameResolution> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.names.get(&source).cloned())
    }

    /// Select one name resolution in the current segment.
    pub(in crate::check) fn select_name(
        &mut self,
        source: dir::GlobalNodeIdAny,
        resolution: dir::NameResolution,
    ) {
        if !self.is_new_selection(source, "name", &resolution, |segment| &segment.names) {
            return;
        }

        self.current_mut().names.insert(source, resolution);
    }

    /// Return the total number of decisions.
    pub(in crate::check) fn decision_count(&self) -> usize {
        self.segments
            .iter()
            .map(|segment| {
                segment.calls.len()
                    + segment.constructs.len()
                    + segment.operators.len()
                    + segment.identities.len()
                    + segment.layouts.len()
                    + segment.members.len()
                    + segment.patterns.len()
                    + segment.receivers.len()
                    + segment.names.len()
            })
            .sum()
    }

    /// Return active names in component order.
    pub(in crate::check) fn names(&self) -> Vec<(dir::GlobalNodeIdAny, dir::NameResolution)> {
        self.segments
            .iter()
            .flat_map(|segment| {
                segment
                    .names
                    .iter()
                    .map(|(source, resolution)| (*source, resolution.clone()))
            })
            .collect()
    }

    /// Return active receivers in component order.
    pub(in crate::check) fn receivers(&self) -> Vec<ReceiverResolution> {
        self.segments
            .iter()
            .flat_map(|segment| segment.receivers.values().copied())
            .collect()
    }

    /// Return active member decisions in component order.
    pub(in crate::check) fn members(&self) -> Vec<MemberDecision> {
        self.segments
            .iter()
            .flat_map(|segment| segment.members.values().cloned())
            .collect()
    }

    /// Return active pattern decisions in component order.
    pub(in crate::check) fn patterns(&self) -> Vec<PatternDecision> {
        self.segments
            .iter()
            .flat_map(|segment| segment.patterns.values().cloned())
            .collect()
    }

    /// Return active call decisions in component order.
    pub(in crate::check) fn calls(&self) -> Vec<CallDecision> {
        self.segments
            .iter()
            .flat_map(|segment| segment.calls.values().cloned())
            .collect()
    }

    /// Return active construct decisions in component order.
    pub(in crate::check) fn constructs(&self) -> Vec<ConstructDecision> {
        self.segments
            .iter()
            .flat_map(|segment| segment.constructs.values().cloned())
            .collect()
    }

    /// Return active operator decisions in component order.
    pub(in crate::check) fn operators(&self) -> Vec<OperatorDecision> {
        self.segments
            .iter()
            .flat_map(|segment| segment.operators.values().cloned())
            .collect()
    }

    /// Return active layout decisions in component order.
    pub(in crate::check) fn layouts(&self) -> Vec<LayoutDecision> {
        self.segments
            .iter()
            .flat_map(|segment| segment.layouts.values().cloned())
            .collect()
    }

    /// Return whether one bound is already active.
    fn contains_bound<T: Eq>(
        &self,
        variable: VariableId,
        bound: T,
        select: impl Fn(&InferenceSegment) -> &IndexMap<VariableId, Vec<T>>,
    ) -> bool {
        self.segments.iter().any(|segment| {
            select(segment)
                .get(&variable)
                .is_some_and(|bounds| bounds.contains(&bound))
        })
    }

    /// Return whether one source has not selected this value yet.
    fn is_new_selection<T: PartialEq>(
        &self,
        source: dir::GlobalNodeIdAny,
        label: &str,
        value: &T,
        select: impl Fn(&InferenceSegment) -> &IndexMap<dir::GlobalNodeIdAny, T>,
    ) -> bool {
        for segment in &self.segments {
            let Some(existing) = select(segment).get(&source) else {
                continue;
            };
            if existing == value {
                return false;
            }

            panic!("check {label} {source:?} received two different selections");
        }

        true
    }

    /// Return one generic slot entry with its id.
    fn generic_slot_entry(&self, slot_id: GenericSlotId) -> Option<(GenericSlotId, &GenericSlot)> {
        let generic = self.generic_slot_by_id(slot_id)?;

        Some((slot_id, generic))
    }
}

impl InferenceSegment {
    /// Create an empty inference segment.
    fn new() -> Self {
        Self {
            terms: TermTable::new(),
            variables: Vec::new(),
            constraints: Vec::new(),
            obligations: Vec::new(),
            type_lower_bounds: IndexMap::new(),
            type_upper_bounds: IndexMap::new(),
            static_lower_bounds: IndexMap::new(),
            static_upper_bounds: IndexMap::new(),
            solutions: IndexMap::new(),
            generic_templates: IndexMap::new(),
            generic_applications: IndexMap::new(),
            generic_slots_by_symbol: IndexMap::new(),
            generic_slots_by_id: IndexMap::new(),
            generic_induction_roots: Vec::new(),
            generic_inductions: IndexMap::new(),
            calls: IndexMap::new(),
            constructs: IndexMap::new(),
            operators: IndexMap::new(),
            identities: IndexMap::new(),
            layouts: IndexMap::new(),
            members: IndexMap::new(),
            patterns: IndexMap::new(),
            receivers: IndexMap::new(),
            names: IndexMap::new(),
        }
    }

    /// Merge one child segment into this segment.
    fn merge(&mut self, mut child: InferenceSegment) -> CompilerResult<()> {
        self.terms.append(child.terms);
        self.variables.append(&mut child.variables);
        self.constraints.append(&mut child.constraints);
        self.obligations.append(&mut child.obligations);

        merge_bounds(&mut self.type_lower_bounds, child.type_lower_bounds);
        merge_bounds(&mut self.type_upper_bounds, child.type_upper_bounds);
        merge_bounds(&mut self.static_lower_bounds, child.static_lower_bounds);
        merge_bounds(&mut self.static_upper_bounds, child.static_upper_bounds);
        self.solutions.extend(child.solutions);

        for (owner, mut template) in child.generic_templates {
            self.generic_templates
                .entry(owner)
                .or_insert_with(|| GenericTemplate::new(owner))
                .slots
                .append(&mut template.slots);
        }
        self.generic_applications.extend(child.generic_applications);
        self.generic_slots_by_symbol
            .extend(child.generic_slots_by_symbol);
        self.generic_slots_by_id.extend(child.generic_slots_by_id);
        self.generic_induction_roots
            .append(&mut child.generic_induction_roots);
        self.generic_inductions.extend(child.generic_inductions);

        merge_selections(&mut self.calls, child.calls, "call")?;
        merge_selections(&mut self.constructs, child.constructs, "construct")?;
        merge_selections(&mut self.operators, child.operators, "operator")?;
        merge_selections(&mut self.identities, child.identities, "identity")?;
        merge_selections(&mut self.layouts, child.layouts, "layout")?;
        merge_selections(&mut self.members, child.members, "member")?;
        merge_selections(&mut self.patterns, child.patterns, "pattern")?;
        merge_selections(&mut self.receivers, child.receivers, "receiver")?;
        merge_selections(&mut self.names, child.names, "name")?;

        Ok(())
    }
}

/// Merge selected entries without overwriting existing selections.
fn merge_selections<T>(
    parent: &mut IndexMap<dir::GlobalNodeIdAny, T>,
    child: IndexMap<dir::GlobalNodeIdAny, T>,
    label: &str,
) -> CompilerResult<()> {
    for (source, value) in child {
        if parent.contains_key(&source) {
            return Err(CompilerError::Internal {
                message: format!("check {label} {source:?} was selected twice"),
            });
        }

        parent.insert(source, value);
    }

    Ok(())
}

/// Merge one child bound map into its parent map.
fn merge_bounds<T>(parent: &mut IndexMap<VariableId, Vec<T>>, child: IndexMap<VariableId, Vec<T>>) {
    for (variable, mut bounds) in child {
        parent.entry(variable).or_default().append(&mut bounds);
    }
}

impl CheckState<'_> {
    /// Return an internal error for one conflicting checked decision.
    pub(in crate::check) fn selection_conflict_error<T: Dump>(
        &self,
        label: &str,
        source: dir::GlobalNodeIdAny,
        existing: &T,
        incoming: &T,
    ) -> CompilerError {
        let context = DumpContext::new(self).with_module(source.module_id);
        let source_label = context.node_label(source);
        let source_location = context.node_source_label(source);
        let existing = existing.dump(&context);
        let incoming = incoming.dump(&context);

        CompilerError::Internal {
            message: format!(
                "check selection conflict kind={label} source={source_label} at={source_location} existing={existing} incoming={incoming}"
            ),
        }
    }

    /// Select one lexical name resolution.
    pub(in crate::check) fn select_name(
        &mut self,
        source: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) {
        let resolution = dir::NameResolution::new(symbol);

        self.inference.select_name(source, resolution);
    }

    /// Return the selected symbol for one resolved lexical name.
    pub(in crate::check) fn selected_name(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> Option<dir::GlobalSymbolId> {
        Some(self.inference.name(source)?.symbol())
    }
}
