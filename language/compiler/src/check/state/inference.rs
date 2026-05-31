use destack_dir as dir;
use indexmap::IndexMap;

use crate::check::{
    CallDecision, CheckState, Constraint, ConstructDecision, IdentityDecision, Induction,
    LayoutDecision, MemberDecision, Obligation, OperatorDecision, PatternDecision,
    ReceiverSelection, Solution, StaticOperand, Term, TermId, TermTable, TypeOperand, Variable,
    VariableId, VariableTable,
};

/// Segmented inference graph for one checked component.
#[derive(Debug)]
pub(in crate::check) struct InferenceTable {
    /// Ordered inference segments.
    segments: Vec<InferenceSegment>,
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
    variables: VariableTable,

    /// Variable constraints in collection order.
    constraints: Vec<Constraint>,
    /// Component post-solve obligations.
    obligations: Vec<Obligation>,
    /// Declaration terms that can induce owner generics.
    inductions: Vec<Induction>,

    /// Type operands that must be assignable to each variable.
    type_lower_bounds: IndexMap<VariableId, Vec<TypeOperand>>,
    /// Type operands that each variable must be assignable to.
    type_upper_bounds: IndexMap<VariableId, Vec<TypeOperand>>,
    /// Static operands that must be assignable to each variable.
    static_lower_bounds: IndexMap<VariableId, Vec<StaticOperand>>,
    /// Static operands that each variable must be assignable to.
    static_upper_bounds: IndexMap<VariableId, Vec<StaticOperand>>,

    /// Solved variable values.
    variable_solutions: IndexMap<VariableId, Solution>,
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
    receivers: IndexMap<dir::GlobalNodeIdAny, ReceiverSelection>,
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
    pub(in crate::check) fn commit_probe(&mut self, probe: InferenceProbe) {
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

        parent.merge(segment);
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
        self.current_mut().variables.variables.push(variable);
    }

    /// Return the number of allocated variables.
    pub(in crate::check) fn variable_count(&self) -> usize {
        self.segments
            .iter()
            .map(|segment| segment.variables.variables.len())
            .sum()
    }

    /// Return one variable by allocation index.
    pub(in crate::check) fn variable_at(&self, index: usize) -> &Variable {
        let mut base = 0;

        for segment in &self.segments {
            let end = base + segment.variables.variables.len();
            if index < end {
                return &segment.variables.variables[index - base];
            }

            base = end;
        }

        panic!("check variable {index} is not allocated")
    }

    /// Return the visible materialized type variable for one type id.
    pub(in crate::check) fn type_variable_by_id(
        &self,
        id: dir::GlobalTypeId,
    ) -> Option<VariableId> {
        for segment in self.segments.iter().rev() {
            if let Some(variable) = segment.variables.type_by_id.get(&id) {
                return Some(*variable);
            }
        }

        None
    }

    /// Insert one materialized type variable into the current segment.
    pub(in crate::check) fn insert_type_variable_id(
        &mut self,
        id: dir::GlobalTypeId,
        variable: VariableId,
    ) {
        self.current_mut().variables.type_by_id.insert(id, variable);
    }

    /// Return whether one type id has been materialized.
    pub(in crate::check) fn contains_type_variable_id(&self, id: dir::GlobalTypeId) -> bool {
        self.type_variable_by_id(id).is_some()
    }

    /// Return the visible materialized static variable for one static id.
    pub(in crate::check) fn static_variable_by_id(
        &self,
        id: dir::GlobalStaticId,
    ) -> Option<VariableId> {
        for segment in self.segments.iter().rev() {
            if let Some(variable) = segment.variables.static_by_id.get(&id) {
                return Some(*variable);
            }
        }

        None
    }

    /// Insert one materialized static variable into the current segment.
    pub(in crate::check) fn insert_static_variable_id(
        &mut self,
        id: dir::GlobalStaticId,
        variable: VariableId,
    ) {
        self.current_mut()
            .variables
            .static_by_id
            .insert(id, variable);
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
    pub(in crate::check) fn push_induction(&mut self, induction: Induction) {
        self.current_mut().inductions.push(induction);
    }

    /// Return all induction roots in component order.
    pub(in crate::check) fn inductions_vec(&self) -> Vec<Induction> {
        self.segments
            .iter()
            .flat_map(|segment| segment.inductions.iter().cloned())
            .collect()
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
            if let Some(solution) = segment.variable_solutions.get(&variable) {
                return Some(solution.clone());
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
            self.current_mut()
                .variable_solutions
                .insert(variable, solution);
        }

        previous
    }

    /// Return the total number of solved variables.
    pub(in crate::check) fn solution_count(&self) -> usize {
        self.segments
            .iter()
            .map(|segment| segment.variable_solutions.len())
            .sum()
    }

    /// Return visible call decision.
    pub(in crate::check) fn call(&self, source: dir::GlobalNodeIdAny) -> Option<CallDecision> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.calls.get(&source).cloned())
    }

    /// Insert one call decision into the current segment.
    pub(in crate::check) fn insert_call(
        &mut self,
        source: dir::GlobalNodeIdAny,
        decision: CallDecision,
    ) {
        self.current_mut().calls.insert(source, decision);
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

    /// Insert one construct decision into the current segment.
    pub(in crate::check) fn insert_construct(
        &mut self,
        source: dir::GlobalNodeIdAny,
        decision: ConstructDecision,
    ) {
        self.current_mut().constructs.insert(source, decision);
    }

    /// Return visible operator decision.
    pub(in crate::check) fn operator(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> Option<OperatorDecision> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.operators.get(&source).cloned())
    }

    /// Insert one operator decision into the current segment.
    pub(in crate::check) fn insert_operator(
        &mut self,
        source: dir::GlobalNodeIdAny,
        decision: OperatorDecision,
    ) {
        self.current_mut().operators.insert(source, decision);
    }

    /// Return visible identity decision.
    pub(in crate::check) fn identity(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> Option<IdentityDecision> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.identities.get(&source).cloned())
    }

    /// Insert one identity decision into the current segment.
    pub(in crate::check) fn insert_identity(
        &mut self,
        source: dir::GlobalNodeIdAny,
        decision: IdentityDecision,
    ) {
        self.current_mut().identities.insert(source, decision);
    }

    /// Return visible layout decision.
    pub(in crate::check) fn layout(&self, source: dir::GlobalNodeIdAny) -> Option<LayoutDecision> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.layouts.get(&source).cloned())
    }

    /// Insert one layout decision into the current segment.
    pub(in crate::check) fn insert_layout(
        &mut self,
        source: dir::GlobalNodeIdAny,
        decision: LayoutDecision,
    ) {
        self.current_mut().layouts.insert(source, decision);
    }

    /// Return visible member decision.
    pub(in crate::check) fn member(&self, source: dir::GlobalNodeIdAny) -> Option<MemberDecision> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.members.get(&source).cloned())
    }

    /// Insert one member decision into the current segment.
    pub(in crate::check) fn insert_member(
        &mut self,
        source: dir::GlobalNodeIdAny,
        decision: MemberDecision,
    ) {
        self.current_mut().members.insert(source, decision);
    }

    /// Return visible pattern decision.
    pub(in crate::check) fn pattern(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> Option<PatternDecision> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.patterns.get(&source).cloned())
    }

    /// Insert one pattern decision into the current segment.
    pub(in crate::check) fn insert_pattern(
        &mut self,
        source: dir::GlobalNodeIdAny,
        decision: PatternDecision,
    ) {
        self.current_mut().patterns.insert(source, decision);
    }

    /// Return visible receiver resolution.
    pub(in crate::check) fn receiver(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> Option<ReceiverSelection> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.receivers.get(&source).cloned())
    }

    /// Insert one receiver resolution into the current segment.
    pub(in crate::check) fn insert_receiver(&mut self, receiver: ReceiverSelection) {
        self.current_mut()
            .receivers
            .insert(receiver.source, receiver);
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

    /// Insert one name resolution into the current segment.
    pub(in crate::check) fn insert_name(
        &mut self,
        source: dir::GlobalNodeIdAny,
        resolution: dir::NameResolution,
    ) {
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
    pub(in crate::check) fn receivers_vec(&self) -> Vec<ReceiverSelection> {
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
}

impl InferenceSegment {
    /// Create an empty inference segment.
    fn new() -> Self {
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
            patterns: IndexMap::new(),
            receivers: IndexMap::new(),
            names: IndexMap::new(),
        }
    }

    /// Merge one child segment into this segment.
    fn merge(&mut self, mut child: InferenceSegment) {
        self.terms.append(child.terms);
        self.variables
            .variables
            .append(&mut child.variables.variables);
        self.variables.type_by_id.extend(child.variables.type_by_id);
        self.variables
            .static_by_id
            .extend(child.variables.static_by_id);
        self.constraints.append(&mut child.constraints);
        self.obligations.append(&mut child.obligations);
        self.inductions.append(&mut child.inductions);

        merge_bounds(&mut self.type_lower_bounds, child.type_lower_bounds);
        merge_bounds(&mut self.type_upper_bounds, child.type_upper_bounds);
        merge_bounds(&mut self.static_lower_bounds, child.static_lower_bounds);
        merge_bounds(&mut self.static_upper_bounds, child.static_upper_bounds);

        self.variable_solutions.extend(child.variable_solutions);
        self.calls.extend(child.calls);
        self.constructs.extend(child.constructs);
        self.operators.extend(child.operators);
        self.identities.extend(child.identities);
        self.layouts.extend(child.layouts);
        self.members.extend(child.members);
        self.patterns.extend(child.patterns);
        self.receivers.extend(child.receivers);
        self.names.extend(child.names);
    }
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
        if let Some(previous) = self.inference.name(source) {
            assert_eq!(
                previous, resolution,
                "check name {source:?} already has a different decision"
            );

            return;
        }

        self.inference.insert_name(source, resolution);
    }

    /// Return the selected symbol for one resolved lexical name.
    pub(in crate::check) fn selected_name(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> Option<dir::GlobalSymbolId> {
        self.inference.name(source)?.symbol()
    }

    /// Begin one speculative inference probe.
    pub(in crate::check) fn begin_inference_probe(&mut self) -> InferenceProbe {
        self.inference.begin_probe()
    }

    /// Commit one speculative inference probe.
    pub(in crate::check) fn commit_inference_probe(&mut self, probe: InferenceProbe) {
        self.inference.commit_probe(probe);
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
