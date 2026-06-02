use destack_dir as dir;
use indexmap::IndexMap;

use crate::check::{
    CallDecision, CheckState, Constraint, ConstructDecision, GenericApplication,
    GenericApplicationKey, GenericInduction, GenericInductionRoot, GenericInductionSlot,
    GenericSlot, GenericTemplate, IdentityDecision, LayoutDecision, MemberDecision, Obligation,
    OperatorDecision, PatternDecision, ReceiverResolution, Solution, StaticOperand, Term, TermId,
    TermTable, TypeOperand, Variable, VariableId,
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
    pub(in crate::check::state) generic_templates: IndexMap<dir::GlobalSymbolId, GenericTemplate>,
    /// Generic applications keyed by source node and owner.
    pub(in crate::check::state) generic_applications:
        IndexMap<GenericApplicationKey, GenericApplication>,
    /// Generic slot variables keyed by parameter symbol.
    pub(in crate::check::state) generic_slots_by_symbol: IndexMap<dir::GlobalSymbolId, VariableId>,
    /// Generic slots keyed by variable.
    pub(in crate::check::state) generic_slots_by_variable: IndexMap<VariableId, GenericSlot>,
    /// Declaration operands that can induce owner generics.
    pub(in crate::check::state) generic_induction_roots: Vec<GenericInductionRoot>,
    /// Variables that can induce owner generics.
    pub(in crate::check::state) generic_inductions: IndexMap<VariableId, GenericInductionSlot>,

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
        assert_eq!(
            probe.depth + 1,
            self.segments.len(),
            "inference probes must be committed in LIFO order"
        );

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
        assert_eq!(
            probe.depth + 1,
            self.segments.len(),
            "inference probes must be dropped in LIFO order"
        );

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

    /// Return one mutable term by id.
    pub(in crate::check) fn term_mut<T: Term>(&mut self, id: TermId<T>) -> &mut T {
        let mut base = 0;

        for segment in &mut self.segments {
            let terms = T::arena_mut(&mut segment.terms);
            let end = base + terms.len();
            if id.index() < end {
                return &mut terms[id.index() - base];
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

    /// Return all constraints in component order.
    pub(in crate::check) fn constraints_vec(&self) -> Vec<Constraint> {
        self.constraints().cloned().collect()
    }

    /// Push one obligation into the current segment.
    pub(in crate::check) fn push_obligation(&mut self, obligation: Obligation) {
        self.current_mut().obligations.push(obligation);
    }

    /// Return all obligations in component order.
    pub(in crate::check) fn obligations_vec(&self) -> Vec<Obligation> {
        self.segments
            .iter()
            .flat_map(|segment| segment.obligations.iter().cloned())
            .collect()
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

        assert!(
            previous.is_none(),
            "check variable {:?} already has generic induction",
            variable.variable
        );
    }

    /// Return the visible induced generic slot for one variable.
    pub(in crate::check) fn generic_induction_slot(
        &self,
        variable: VariableId,
    ) -> Option<GenericInductionSlot> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.generic_inductions.get(&variable).cloned())
    }

    /// Return visible lower type bounds for one variable.
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

    /// Return visible upper type bounds for one variable.
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

    /// Return visible lower static bounds for one variable.
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

    /// Return visible upper static bounds for one variable.
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

    /// Return the visible solution for one variable.
    pub(in crate::check) fn variable_solution(&self, variable: VariableId) -> Option<Solution> {
        for segment in self.segments.iter().rev() {
            if let Some(solution) = segment.solutions.get(&variable) {
                return Some(*solution);
            }
        }

        None
    }

    /// Insert one variable solution into the current segment.
    pub(in crate::check) fn insert_variable_solution(
        &mut self,
        variable: VariableId,
        solution: Solution,
    ) -> Option<Solution> {
        let previous = self.variable_solution(variable);
        if previous.is_none() {
            self.current_mut().solutions.insert(variable, solution);
        }

        previous
    }

    /// Return the total number of solved variables.
    pub(in crate::check) fn solution_count(&self) -> usize {
        self.segments
            .iter()
            .map(|segment| segment.solutions.len())
            .sum()
    }

    /// Return visible call decision.
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
    ) -> CompilerResult<()> {
        self.require_unselected(source, "call", |segment| &segment.calls)?;
        self.current_mut().calls.insert(source, decision);

        Ok(())
    }

    /// Return visible construct decision.
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
    ) -> CompilerResult<()> {
        self.require_unselected(source, "construct", |segment| &segment.constructs)?;
        self.current_mut().constructs.insert(source, decision);

        Ok(())
    }

    /// Select one operator decision in the current segment.
    pub(in crate::check) fn select_operator(
        &mut self,
        source: dir::GlobalNodeIdAny,
        decision: OperatorDecision,
    ) -> CompilerResult<()> {
        self.require_unselected(source, "operator", |segment| &segment.operators)?;
        self.current_mut().operators.insert(source, decision);

        Ok(())
    }

    /// Select one identity decision in the current segment.
    pub(in crate::check) fn select_identity(
        &mut self,
        source: dir::GlobalNodeIdAny,
        decision: IdentityDecision,
    ) -> CompilerResult<()> {
        self.require_unselected(source, "identity", |segment| &segment.identities)?;
        self.current_mut().identities.insert(source, decision);

        Ok(())
    }

    /// Select one layout decision in the current segment.
    pub(in crate::check) fn select_layout(
        &mut self,
        source: dir::GlobalNodeIdAny,
        decision: LayoutDecision,
    ) -> CompilerResult<()> {
        self.require_unselected(source, "layout", |segment| &segment.layouts)?;
        self.current_mut().layouts.insert(source, decision);

        Ok(())
    }

    /// Select one member decision in the current segment.
    pub(in crate::check) fn select_member(
        &mut self,
        source: dir::GlobalNodeIdAny,
        decision: MemberDecision,
    ) -> CompilerResult<()> {
        self.require_unselected(source, "member", |segment| &segment.members)?;
        self.current_mut().members.insert(source, decision);

        Ok(())
    }

    /// Select one pattern decision in the current segment.
    pub(in crate::check) fn select_pattern(
        &mut self,
        source: dir::GlobalNodeIdAny,
        decision: PatternDecision,
    ) -> CompilerResult<()> {
        self.require_unselected(source, "pattern", |segment| &segment.patterns)?;
        self.current_mut().patterns.insert(source, decision);

        Ok(())
    }

    /// Select one receiver resolution in the current segment.
    pub(in crate::check) fn select_receiver(
        &mut self,
        receiver: ReceiverResolution,
    ) -> CompilerResult<()> {
        self.require_unselected(receiver.source, "receiver", |segment| &segment.receivers)?;
        self.current_mut()
            .receivers
            .insert(receiver.source, receiver);

        Ok(())
    }

    /// Return visible name resolution.
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
    ) -> CompilerResult<()> {
        self.require_unselected(source, "name", |segment| &segment.names)?;
        self.current_mut().names.insert(source, resolution);

        Ok(())
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

    /// Return visible names in component order.
    pub(in crate::check) fn names_vec(&self) -> Vec<(dir::GlobalNodeIdAny, dir::NameResolution)> {
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

    /// Return visible receivers in component order.
    pub(in crate::check) fn receivers_vec(&self) -> Vec<ReceiverResolution> {
        self.segments
            .iter()
            .flat_map(|segment| segment.receivers.values().copied())
            .collect()
    }

    /// Return visible member decisions in component order.
    pub(in crate::check) fn members_vec(&self) -> Vec<MemberDecision> {
        self.segments
            .iter()
            .flat_map(|segment| segment.members.values().cloned())
            .collect()
    }

    /// Return visible pattern decisions in component order.
    pub(in crate::check) fn patterns_vec(&self) -> Vec<PatternDecision> {
        self.segments
            .iter()
            .flat_map(|segment| segment.patterns.values().cloned())
            .collect()
    }

    /// Return visible call decisions in component order.
    pub(in crate::check) fn calls_vec(&self) -> Vec<CallDecision> {
        self.segments
            .iter()
            .flat_map(|segment| segment.calls.values().cloned())
            .collect()
    }

    /// Return visible construct decisions in component order.
    pub(in crate::check) fn constructs_vec(&self) -> Vec<ConstructDecision> {
        self.segments
            .iter()
            .flat_map(|segment| segment.constructs.values().cloned())
            .collect()
    }

    /// Return visible operator decisions in component order.
    pub(in crate::check) fn operators_vec(&self) -> Vec<OperatorDecision> {
        self.segments
            .iter()
            .flat_map(|segment| segment.operators.values().cloned())
            .collect()
    }

    /// Return visible layout decisions in component order.
    pub(in crate::check) fn layouts_vec(&self) -> Vec<LayoutDecision> {
        self.segments
            .iter()
            .flat_map(|segment| segment.layouts.values().cloned())
            .collect()
    }

    /// Return whether one bound is already visible.
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

    /// Require one source to have no visible selection.
    fn require_unselected<T>(
        &self,
        source: dir::GlobalNodeIdAny,
        label: &str,
        select: impl Fn(&InferenceSegment) -> &IndexMap<dir::GlobalNodeIdAny, T>,
    ) -> CompilerResult<()> {
        let is_selected = self
            .segments
            .iter()
            .any(|segment| select(segment).contains_key(&source));

        if is_selected {
            return Err(CompilerError::Internal {
                message: format!("check {label} {source:?} was selected twice"),
            });
        }

        Ok(())
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
            generic_slots_by_variable: IndexMap::new(),
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
        self.generic_slots_by_variable
            .extend(child.generic_slots_by_variable);
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
    /// Select one lexical name resolution.
    pub(in crate::check) fn select_name(
        &mut self,
        source: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) {
        let resolution = dir::NameResolution::new(symbol);

        match self.inference.select_name(source, resolution) {
            Ok(()) => {}
            Err(CompilerError::Internal { message }) => self.record_internal_error(message),
            Err(error) => self.record_internal_error(format!("{error:?}")),
        }
    }

    /// Return the selected symbol for one resolved lexical name.
    pub(in crate::check) fn selected_name(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> Option<dir::GlobalSymbolId> {
        Some(self.inference.name(source)?.symbol())
    }

    /// Begin one speculative inference probe.
    pub(in crate::check) fn begin_inference_probe(&mut self) -> InferenceProbe {
        self.inference.begin_probe()
    }

    /// Commit one speculative inference probe.
    pub(in crate::check) fn commit_inference_probe(
        &mut self,
        probe: InferenceProbe,
    ) -> CompilerResult<()> {
        self.inference.commit_probe(probe)
    }

    /// Drop one speculative inference probe.
    pub(in crate::check) fn drop_inference_probe(&mut self, probe: InferenceProbe) {
        self.inference.drop_probe(probe);
    }

    /// Push one inference term into the current segment.
    pub(in crate::check) fn push_term<T: Term>(&mut self, term: T) -> TermId<T> {
        self.inference.push_term(term)
    }

    /// Return one inference term by id.
    pub(in crate::check) fn term<T: Term>(&self, id: TermId<T>) -> &T {
        self.inference.term(id)
    }

    /// Return one mutable inference term by id.
    pub(in crate::check) fn term_mut<T: Term>(&mut self, id: TermId<T>) -> &mut T {
        self.inference.term_mut(id)
    }
}
