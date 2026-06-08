use destack_dir as dir;
use indexmap::{IndexMap, IndexSet};
use smallvec::SmallVec;

use crate::check::{
    CallDecision, CheckEvent, CheckState, Constraint, ConstraintId, ConstructDecision,
    GenericArgument, GenericInduction, GenericInductionParameter, GenericInductionSource,
    GenericInstance, GenericInstanceKey, GenericParameterBinding, GenericParameterId,
    GenericTemplate, GenericTemplateId, IdentityDecision, LayoutDecision, MemberDecision,
    Obligation, OperatorDecision, PatternDecision, ReceiverResolution, Solution, SolveTask,
    StaticOperand, Term, TermCursor, TermId, TermTable, TypeOperand, TypeTerm, Variable,
    VariableId,
};
use crate::{CompilerError, CompilerResult};

/// Segmented inference graph for one checked component.
#[derive(Debug)]
pub(in crate::check) struct InferenceTable {
    /// Component term table.
    terms: TermTable,
    /// Ordered inference segments.
    pub(in crate::check::state) segments: Vec<InferenceSegment>,
}

/// Live speculative inference segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct InferenceProbe {
    /// The probed segment depth.
    depth: usize,
    /// Term table cursor before the probe.
    term_cursor: TermCursor,
}

/// One permanent or speculative inference segment.
#[derive(Debug)]
pub(in crate::check) struct InferenceSegment {
    /// Variables allocated in this segment.
    variables: Vec<Variable>,
    /// Solved variable values.
    solutions: IndexMap<VariableId, Solution>,
    /// Type operands that must be assignable to each variable.
    type_lower_bounds: IndexMap<VariableId, Vec<TypeOperand>>,
    /// Type operands that each variable must be assignable to.
    type_upper_bounds: IndexMap<VariableId, Vec<TypeOperand>>,
    /// Static operands that must be assignable to each variable.
    static_lower_bounds: IndexMap<VariableId, Vec<StaticOperand>>,
    /// Static operands that each variable must be assignable to.
    static_upper_bounds: IndexMap<VariableId, Vec<StaticOperand>>,

    /// Variable constraints in collection order.
    constraints: Vec<Constraint>,
    /// Constraints depending on each variable.
    constraint_dependents_by_variable: IndexMap<VariableId, Vec<ConstraintId>>,
    /// Bound variables depending on each variable.
    bound_dependents_by_variable: IndexMap<VariableId, Vec<VariableId>>,
    /// Component post-solve obligations.
    obligations: Vec<Obligation>,

    /// Generic templates keyed by template id.
    generic_templates: IndexMap<GenericTemplateId, GenericTemplate>,
    /// Generic template ids keyed by source node.
    generic_templates_by_source: IndexMap<dir::GlobalNodeIdAny, GenericTemplateId>,
    /// Generic template ids keyed by declaration symbol.
    generic_templates_by_symbol: IndexMap<dir::GlobalSymbolId, GenericTemplateId>,
    /// Generic instances keyed by source node and owner.
    generic_instances: IndexMap<GenericInstanceKey, GenericInstance>,
    /// Generic parameter ids keyed by parameter symbol.
    generic_parameters_by_symbol: IndexMap<dir::GlobalSymbolId, GenericParameterId>,
    /// Generic parameters keyed by parameter id.
    generic_parameters_by_id: IndexMap<GenericParameterId, GenericParameterBinding>,
    /// Declaration operands that can induce template generics.
    generic_induction_sources: Vec<GenericInductionSource>,
    /// Variables that can induce template generics.
    generic_inductions: IndexMap<VariableId, GenericInductionParameter>,

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

    /// Scheduled solver tasks in insertion order.
    worklist: IndexSet<SolveTask>,
    /// Trace events emitted by this segment.
    events: Vec<CheckEvent>,
}

impl InferenceTable {
    /// Create an empty inference graph.
    pub(in crate::check) fn new() -> Self {
        Self {
            terms: TermTable::new(),
            segments: vec![InferenceSegment::new()],
        }
    }

    /// Return the current mutable inference segment.
    pub(in crate::check) fn current_mut(&mut self) -> &mut InferenceSegment {
        self.segments
            .last_mut()
            .unwrap_or_else(|| unreachable!("inference table has no current segment"))
    }

    /// Begin one speculative inference segment.
    pub(in crate::check) fn begin_probe(&mut self) -> InferenceProbe {
        let depth = self.segments.len();
        let term_cursor = self.terms.cursor();
        self.segments.push(InferenceSegment::new());

        InferenceProbe { depth, term_cursor }
    }

    /// Merge the current speculative segment into its parent.
    pub(in crate::check) fn commit_probe(&mut self, probe: InferenceProbe) -> CompilerResult<()> {
        if probe.depth + 1 != self.segments.len() {
            unreachable!("inference probes must be committed in LIFO order");
        }

        let segment = self
            .segments
            .pop()
            .unwrap_or_else(|| unreachable!("inference probe segment is missing"));
        let parent = self
            .segments
            .last_mut()
            .unwrap_or_else(|| unreachable!("inference probe has no parent segment"));

        parent.merge(segment)?;

        Ok(())
    }

    /// Drop the current speculative segment.
    pub(in crate::check) fn drop_probe(&mut self, probe: InferenceProbe) {
        if probe.depth + 1 != self.segments.len() {
            unreachable!("inference probes must be dropped in LIFO order");
        }

        self.segments.pop();
        self.terms.truncate(probe.term_cursor);
    }

    /// Return whether inference is currently in a speculative probe.
    pub(in crate::check) fn is_probing(&self) -> bool {
        self.segments.len() > 1
    }

    /// Push one trace event into the current inference segment.
    pub(in crate::check) fn push_event(&mut self, event: CheckEvent) {
        self.current_mut().events.push(event);
    }

    /// Iterate retained trace events in segment order.
    pub(in crate::check) fn events(&self) -> impl Iterator<Item = &CheckEvent> {
        self.segments
            .iter()
            .flat_map(|segment| segment.events.iter())
    }

    /// Push one term into the current inference segment.
    pub(in crate::check) fn push_term<T: Term>(&mut self, term: T) -> TermId<T> {
        self.terms.push(term)
    }

    /// Return one term by id.
    pub(in crate::check) fn term<T: Term>(&self, id: TermId<T>) -> &T {
        self.terms.get(id)
    }

    /// Return the total number of stored terms.
    pub(in crate::check) fn term_count_total(&self) -> usize {
        self.terms.len()
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

        unreachable!("check variable {index} is not allocated")
    }

    /// Push one constraint into the current segment.
    pub(in crate::check) fn push_constraint(
        &mut self,
        constraint: Constraint,
        variables: impl IntoIterator<Item = VariableId>,
    ) -> ConstraintId {
        let id = ConstraintId::new(self.constraint_count() as u32);

        self.current_mut().constraints.push(constraint);
        for variable in variables {
            self.current_mut()
                .constraint_dependents_by_variable
                .entry(variable)
                .or_default()
                .push(id);
        }
        self.push_solve_task(SolveTask::Constraint(id));

        id
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

        unreachable!("check constraint {index} is not allocated")
    }

    /// Return one constraint by id.
    pub(in crate::check) fn constraint_by_id(&self, id: ConstraintId) -> &Constraint {
        self.constraint(id.index())
    }

    /// Schedule one solver task.
    pub(in crate::check) fn push_solve_task(&mut self, task: SolveTask) {
        if !self
            .segments
            .iter()
            .any(|segment| segment.worklist.contains(&task))
        {
            self.current_mut().worklist.insert(task);
        }
    }

    /// Return the next scheduled solver task.
    pub(in crate::check) fn pop_solve_task(&mut self) -> Option<SolveTask> {
        for segment in &mut self.segments {
            let Some(task) = segment.worklist.iter().next().copied() else {
                continue;
            };

            segment.worklist.shift_remove(&task);

            return Some(task);
        }

        None
    }

    /// Return the number of scheduled solver tasks.
    pub(in crate::check) fn solve_task_count(&self) -> usize {
        self.segments
            .iter()
            .map(|segment| segment.worklist.len())
            .sum()
    }

    /// Record that one variable's bounds read another variable.
    pub(in crate::check) fn insert_bound_dependency(
        &mut self,
        source: VariableId,
        dependent: VariableId,
    ) {
        let dependents = self
            .current_mut()
            .bound_dependents_by_variable
            .entry(source)
            .or_default();
        if dependents.contains(&dependent) {
            return;
        }

        dependents.push(dependent);
    }

    /// Schedule all solver tasks depending on one solved variable.
    pub(in crate::check) fn wake_variable(&mut self, variable: VariableId) {
        let constraints = self
            .segments
            .iter()
            .flat_map(|segment| {
                segment
                    .constraint_dependents_by_variable
                    .get(&variable)
                    .into_iter()
                    .flatten()
                    .copied()
            })
            .collect::<Vec<_>>();
        let bound_dependents = self
            .segments
            .iter()
            .flat_map(|segment| {
                segment
                    .bound_dependents_by_variable
                    .get(&variable)
                    .into_iter()
                    .flatten()
                    .copied()
            })
            .collect::<Vec<_>>();

        for constraint in constraints {
            self.push_solve_task(SolveTask::Constraint(constraint));
        }

        for variable in bound_dependents {
            self.push_solve_task(SolveTask::Variable(variable));
        }
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

    /// Add one induction source into the current segment.
    pub(in crate::check) fn add_generic_induction_source(
        &mut self,
        source: GenericInductionSource,
    ) {
        self.current_mut().generic_induction_sources.push(source);
    }

    /// Iterate generic induction sources in component order.
    pub(in crate::check) fn generic_induction_sources(
        &self,
    ) -> impl Iterator<Item = &GenericInductionSource> {
        self.segments
            .iter()
            .flat_map(|segment| segment.generic_induction_sources.iter())
    }

    /// Mark one variable as able to induce a template generic.
    pub(in crate::check) fn insert_generic_induction(&mut self, variable: GenericInduction) {
        let previous = self
            .current_mut()
            .generic_inductions
            .insert(variable.variable, variable.parameter);

        if previous.is_some() {
            unreachable!(
                "variable {:?} already has generic induction",
                variable.variable
            );
        }
    }

    /// Return the active induced generic parameter for one variable.
    pub(in crate::check) fn generic_induction_parameter(
        &self,
        variable: VariableId,
    ) -> Option<GenericInductionParameter> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.generic_inductions.get(&variable).cloned())
    }

    /// Return generic parameters in template order.
    pub(in crate::check) fn generic_parameters(
        &self,
    ) -> impl Iterator<Item = (GenericParameterId, &GenericParameterBinding)> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.generic_templates.values())
            .flat_map(|template| template.parameters.iter())
            .filter_map(|parameter_id| self.generic_parameter_entry(*parameter_id))
    }

    /// Return generic parameters declared by one template.
    pub(in crate::check) fn generic_template_parameters(
        &self,
        template: GenericTemplateId,
    ) -> impl Iterator<Item = (GenericParameterId, &GenericParameterBinding)> + '_ {
        self.segments
            .iter()
            .flat_map(move |segment| segment.generic_templates.get(&template))
            .flat_map(|template| template.parameters.iter().copied())
            .filter_map(|parameter_id| self.generic_parameter_entry(parameter_id))
    }

    /// Return one active generic template.
    pub(in crate::check) fn generic_template(
        &self,
        template: GenericTemplateId,
    ) -> Option<&GenericTemplate> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.generic_templates.get(&template))
    }

    /// Iterate generic templates in allocation order.
    pub(in crate::check) fn generic_templates(
        &self,
    ) -> impl Iterator<Item = (GenericTemplateId, &GenericTemplate)> + '_ {
        self.segments
            .iter()
            .flat_map(|segment| segment.generic_templates.iter())
            .map(|(template, value)| (*template, value))
    }

    /// Return one active generic template id by source node.
    pub(in crate::check) fn generic_template_by_source(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> Option<GenericTemplateId> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.generic_templates_by_source.get(&source).copied())
    }

    /// Return one active generic template id by declaration symbol.
    pub(in crate::check) fn symbol_generic_template(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<GenericTemplateId> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.generic_templates_by_symbol.get(&symbol).copied())
    }

    /// Upsert one declaration symbol to generic template mapping.
    pub(in crate::check) fn upsert_symbol_generic_template(
        &mut self,
        template: GenericTemplateId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        if self.generic_template(template).is_none() {
            return Err(CompilerError::Internal {
                message: format!("generic template {template:?} is not allocated"),
            });
        }
        if let Some(existing) = self.symbol_generic_template(symbol) {
            if existing != template {
                return Err(CompilerError::Internal {
                    message: format!("generic template symbol {symbol:?} points at two templates"),
                });
            }

            return Ok(());
        }

        self.current_mut()
            .generic_templates_by_symbol
            .insert(symbol, template);

        Ok(())
    }

    /// Return one active generic parameter.
    pub(in crate::check) fn generic_parameter(
        &self,
        parameter_id: GenericParameterId,
    ) -> Option<&GenericParameterBinding> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.generic_parameters_by_id.get(&parameter_id))
    }

    /// Return one required generic parameter.
    pub(in crate::check) fn require_generic_parameter(
        &self,
        parameter_id: GenericParameterId,
    ) -> &GenericParameterBinding {
        self.generic_parameter(parameter_id)
            .unwrap_or_else(|| unreachable!("generic parameter {parameter_id:?} does not exist"))
    }

    /// Return one active generic instance.
    pub(in crate::check) fn generic_instance(
        &self,
        key: GenericInstanceKey,
    ) -> Option<&GenericInstance> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.generic_instances.get(&key))
    }

    /// Upsert one generic instance into the current segment.
    pub(in crate::check) fn upsert_generic_instance(
        &mut self,
        key: GenericInstanceKey,
        arguments: SmallVec<[GenericArgument; 2]>,
    ) -> GenericInstance {
        if let Some(instance) = self.generic_instance(key) {
            if instance.arguments != arguments {
                unreachable!("generic instance {key:?} already has different arguments");
            }

            return instance.clone();
        }

        let instance = GenericInstance::new(key.template, arguments);

        self.current_mut()
            .generic_instances
            .insert(key, instance.clone());

        instance
    }

    /// Insert one generic parameter into the current segment.
    pub(in crate::check) fn insert_generic_parameter(&mut self, generic: GenericParameterBinding) {
        let template = generic.parameter().template;
        let key = generic.parameter().key;
        let parameter_id = generic.parameter().id();
        if self.generic_parameter(parameter_id).is_some() {
            unreachable!("generic parameter {parameter_id:?} is already inserted");
        }

        let segment = self.current_mut();
        segment
            .generic_parameters_by_id
            .insert(parameter_id, generic);
        segment
            .generic_templates
            .get_mut(&template)
            .unwrap_or_else(|| unreachable!("generic template {template:?} is not allocated"))
            .parameters
            .push(parameter_id);
        if let dir::GenericParameterKey::Symbol(symbol) = key {
            segment
                .generic_parameters_by_symbol
                .insert(symbol, parameter_id);
        }
    }

    /// Insert one generic template into the current segment.
    pub(in crate::check) fn insert_generic_template(
        &mut self,
        template: GenericTemplate,
        symbol: Option<dir::GlobalSymbolId>,
    ) {
        if self.generic_template(template.id).is_some() {
            unreachable!("generic template {:?} is already inserted", template.id);
        }
        if self.generic_template_by_source(template.source).is_some() {
            unreachable!(
                "generic template source {:?} is already inserted",
                template.source
            );
        }

        let segment = self.current_mut();
        if let Some(symbol) = symbol {
            let previous = segment
                .generic_templates_by_symbol
                .insert(symbol, template.id);

            if previous.is_some() {
                unreachable!("generic template symbol {symbol:?} is already inserted");
            }
        }
        segment
            .generic_templates_by_source
            .insert(template.source, template.id);
        segment.generic_templates.insert(template.id, template);
    }

    /// Return one symbol's active generic parameter id.
    pub(in crate::check) fn symbol_generic_parameter(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<GenericParameterId> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.generic_parameters_by_symbol.get(&symbol).copied())
    }

    /// Return one active generic parameter mutably.
    pub(in crate::check) fn generic_parameter_mut(
        &mut self,
        parameter_id: GenericParameterId,
    ) -> &mut GenericParameterBinding {
        self.segments
            .iter_mut()
            .rev()
            .find_map(|segment| segment.generic_parameters_by_id.get_mut(&parameter_id))
            .unwrap_or_else(|| unreachable!("generic parameter {parameter_id:?} does not exist"))
    }

    /// Add one type constraint to an existing type generic parameter.
    pub(in crate::check) fn constrain_generic_type_parameter(
        &mut self,
        parameter_id: GenericParameterId,
        constraint: TypeOperand,
    ) {
        let current = self
            .require_generic_parameter(parameter_id)
            .type_constraint();
        let constraint = match current {
            Some(current) => TypeOperand::Term(self.push_term(TypeTerm::Intersection {
                elements: vec![current, constraint],
            })),
            None => constraint,
        };
        let generic = self.generic_parameter_mut(parameter_id);

        match generic {
            GenericParameterBinding::Type {
                constraint: current,
                ..
            }
            | GenericParameterBinding::VariadicType {
                constraint: current,
                ..
            } => *current = Some(constraint),
            GenericParameterBinding::Static { .. }
            | GenericParameterBinding::VariadicStatic { .. } => {
                unreachable!("generic parameter {parameter_id:?} is not a type parameter")
            }
        }
    }

    /// Return the next generic parameter number for one template.
    pub(in crate::check) fn next_generic_parameter_number(
        &self,
        template: GenericTemplateId,
    ) -> u32 {
        self.segments
            .iter()
            .filter_map(|segment| segment.generic_templates.get(&template))
            .map(|template| template.parameters.len())
            .sum::<usize>() as u32
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
    ) -> CompilerResult<()> {
        self.select_once(
            source,
            "call",
            decision,
            |segment| &segment.calls,
            |segment| &mut segment.calls,
        )
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
    ) -> CompilerResult<()> {
        self.select_once(
            source,
            "construct",
            decision,
            |segment| &segment.constructs,
            |segment| &mut segment.constructs,
        )
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
    ) -> CompilerResult<()> {
        self.select_once(
            source,
            "operator",
            decision,
            |segment| &segment.operators,
            |segment| &mut segment.operators,
        )
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
    ) -> CompilerResult<()> {
        self.select_once(
            source,
            "identity",
            decision,
            |segment| &segment.identities,
            |segment| &mut segment.identities,
        )
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
    ) -> CompilerResult<()> {
        self.select_once(
            source,
            "layout",
            decision,
            |segment| &segment.layouts,
            |segment| &mut segment.layouts,
        )
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
    ) -> CompilerResult<()> {
        self.select_once(
            source,
            "member",
            decision,
            |segment| &segment.members,
            |segment| &mut segment.members,
        )
    }

    /// Select one pattern decision in the current segment.
    pub(in crate::check) fn select_pattern(
        &mut self,
        source: dir::GlobalNodeIdAny,
        decision: PatternDecision,
    ) -> CompilerResult<()> {
        self.select_once(
            source,
            "pattern",
            decision,
            |segment| &segment.patterns,
            |segment| &mut segment.patterns,
        )
    }

    /// Select one receiver resolution in the current segment.
    pub(in crate::check) fn select_receiver(
        &mut self,
        receiver: ReceiverResolution,
    ) -> CompilerResult<()> {
        self.select_once(
            receiver.source,
            "receiver",
            receiver,
            |segment| &segment.receivers,
            |segment| &mut segment.receivers,
        )
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
    ) -> CompilerResult<()> {
        self.select_once(
            source,
            "name",
            resolution,
            |segment| &segment.names,
            |segment| &mut segment.names,
        )
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
    pub(in crate::check) fn members(&self) -> Vec<(dir::GlobalNodeIdAny, MemberDecision)> {
        self.segments
            .iter()
            .flat_map(|segment| {
                segment
                    .members
                    .iter()
                    .map(|(source, decision)| (*source, decision.clone()))
            })
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
    pub(in crate::check) fn calls(&self) -> Vec<(dir::GlobalNodeIdAny, CallDecision)> {
        self.segments
            .iter()
            .flat_map(|segment| {
                segment
                    .calls
                    .iter()
                    .map(|(source, decision)| (*source, decision.clone()))
            })
            .collect()
    }

    /// Return active construct decisions in component order.
    pub(in crate::check) fn constructs(&self) -> Vec<(dir::GlobalNodeIdAny, ConstructDecision)> {
        self.segments
            .iter()
            .flat_map(|segment| {
                segment
                    .constructs
                    .iter()
                    .map(|(source, decision)| (*source, decision.clone()))
            })
            .collect()
    }

    /// Return active operator decisions in component order.
    pub(in crate::check) fn operators(&self) -> Vec<(dir::GlobalNodeIdAny, OperatorDecision)> {
        self.segments
            .iter()
            .flat_map(|segment| {
                segment
                    .operators
                    .iter()
                    .map(|(source, decision)| (*source, decision.clone()))
            })
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

    /// Select one stable value in the current segment.
    fn select_once<T: PartialEq>(
        &mut self,
        source: dir::GlobalNodeIdAny,
        label: &str,
        value: T,
        select: impl Fn(&InferenceSegment) -> &IndexMap<dir::GlobalNodeIdAny, T>,
        select_mut: impl Fn(&mut InferenceSegment) -> &mut IndexMap<dir::GlobalNodeIdAny, T>,
    ) -> CompilerResult<()> {
        for segment in &self.segments {
            let Some(existing) = select(segment).get(&source) else {
                continue;
            };
            if existing == &value {
                return Ok(());
            }

            return Err(CompilerError::Internal {
                message: format!("check {label} {source:?} received two different selections"),
            });
        }

        select_mut(self.current_mut()).insert(source, value);

        Ok(())
    }

    /// Return one generic parameter entry with its id.
    fn generic_parameter_entry(
        &self,
        parameter_id: GenericParameterId,
    ) -> Option<(GenericParameterId, &GenericParameterBinding)> {
        let generic = self.generic_parameter(parameter_id)?;

        Some((parameter_id, generic))
    }
}

impl InferenceSegment {
    /// Create an empty inference segment.
    fn new() -> Self {
        Self {
            variables: Vec::new(),
            solutions: IndexMap::new(),
            type_lower_bounds: IndexMap::new(),
            type_upper_bounds: IndexMap::new(),
            static_lower_bounds: IndexMap::new(),
            static_upper_bounds: IndexMap::new(),
            constraints: Vec::new(),
            constraint_dependents_by_variable: IndexMap::new(),
            bound_dependents_by_variable: IndexMap::new(),
            obligations: Vec::new(),
            worklist: IndexSet::new(),
            generic_templates: IndexMap::new(),
            generic_templates_by_source: IndexMap::new(),
            generic_templates_by_symbol: IndexMap::new(),
            generic_instances: IndexMap::new(),
            generic_parameters_by_symbol: IndexMap::new(),
            generic_parameters_by_id: IndexMap::new(),
            generic_induction_sources: Vec::new(),
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
            events: Vec::new(),
        }
    }

    /// Merge one child segment into this segment.
    fn merge(&mut self, mut child: InferenceSegment) -> CompilerResult<()> {
        self.variables.append(&mut child.variables);
        self.solutions.extend(child.solutions);
        merge_bounds(&mut self.type_lower_bounds, child.type_lower_bounds);
        merge_bounds(&mut self.type_upper_bounds, child.type_upper_bounds);
        merge_bounds(&mut self.static_lower_bounds, child.static_lower_bounds);
        merge_bounds(&mut self.static_upper_bounds, child.static_upper_bounds);

        self.constraints.append(&mut child.constraints);
        merge_indexes(
            &mut self.constraint_dependents_by_variable,
            child.constraint_dependents_by_variable,
        );
        merge_indexes(
            &mut self.bound_dependents_by_variable,
            child.bound_dependents_by_variable,
        );
        self.obligations.append(&mut child.obligations);

        self.worklist.extend(child.worklist);

        for (template_id, template) in child.generic_templates {
            let previous = self.generic_templates.insert(template_id, template);

            if previous.is_some() {
                unreachable!("generic template {template_id:?} is already inserted");
            }
        }
        self.generic_templates_by_source
            .extend(child.generic_templates_by_source);
        self.generic_templates_by_symbol
            .extend(child.generic_templates_by_symbol);
        self.generic_instances.extend(child.generic_instances);
        self.generic_parameters_by_symbol
            .extend(child.generic_parameters_by_symbol);
        self.generic_parameters_by_id
            .extend(child.generic_parameters_by_id);
        self.generic_induction_sources
            .append(&mut child.generic_induction_sources);
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
        self.events.append(&mut child.events);

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

/// Merge one child dependency index into its parent.
fn merge_indexes<K, V>(parent: &mut IndexMap<K, Vec<V>>, child: IndexMap<K, Vec<V>>)
where
    K: Eq + std::hash::Hash,
{
    for (key, mut values) in child {
        parent.entry(key).or_default().append(&mut values);
    }
}

impl CheckState<'_> {
    /// Return the selected symbol for one resolved lexical name.
    pub(in crate::check) fn selected_name(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> Option<dir::GlobalSymbolId> {
        Some(self.inference.name(source)?.symbol())
    }
}
