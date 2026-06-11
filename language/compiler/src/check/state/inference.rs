use destack_dir as dir;
use indexmap::{IndexMap, IndexSet};
use smallvec::SmallVec;

use crate::check::{
    CallDecision, CheckEvent, Constraint, ConstraintId, ConstructDecision, Dependency,
    GenericArgumentDefault, GenericArgumentKey, GenericInductionParameter, GenericInductionSite,
    GenericInstance, GenericInstanceKey, GenericParameterBinding, GenericParameterId,
    GenericTemplate, GenericTemplateId, IdentityDecision, LayoutDecision, MemberDecision,
    Obligation, OperatorDecision, PatternDecision, ReceiverResolution, Solution, StaticOperand,
    Task, Term, TermCursor, TermId, TermTable, TypeOperand, Variable, VariableId,
};
use crate::{CompilerError, CompilerResult};

/// Segmented inference graph for one checked component.
#[derive(Debug)]
pub(in crate::check) struct InferenceTable {
    /// Component term table.
    pub(in crate::check::state) terms: TermTable,
    /// Ordered inference segments.
    pub(in crate::check::state) segments: Vec<InferenceSegment>,
    /// Solver task currently being reduced.
    pub(in crate::check::state) active_task: Option<Task>,
}

/// Live speculative inference segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct InferenceProbe {
    /// The probed segment depth.
    pub(in crate::check::state) depth: usize,
    /// Variable count before the probe.
    pub(in crate::check::state) variable_count: usize,
    /// Constraint count before the probe.
    pub(in crate::check::state) constraint_count: usize,
    /// Term table cursor before the probe.
    pub(in crate::check::state) term_cursor: TermCursor,
}

impl InferenceProbe {
    /// Return whether one constraint existed before this probe.
    pub(in crate::check) fn precedes_constraint(self, constraint: ConstraintId) -> bool {
        constraint.index() < self.constraint_count
    }

    /// Return whether one variable existed before this probe.
    pub(in crate::check) fn precedes_variable(self, variable: VariableId) -> bool {
        (variable.index as usize) < self.variable_count
    }

    /// Return whether dependencies name unresolved state outside this probe.
    pub(in crate::check) fn has_external_dependency(
        self,
        dependencies: &[Dependency],
        inference: &InferenceTable,
    ) -> bool {
        dependencies.iter().any(|dependency| {
            dependency.precedes_probe(self) && !inference.is_dependency_ready(*dependency)
        })
    }
}

/// One permanent or speculative inference segment.
#[derive(Debug)]
pub(in crate::check::state) struct InferenceSegment {
    /// Variables allocated in this segment.
    pub(in crate::check::state) variables: Vec<Variable>,
    /// Solved variable values.
    pub(in crate::check::state) solutions: IndexMap<VariableId, Solution>,
    /// Type operands that must be assignable to each variable.
    pub(in crate::check::state) type_lower_bounds: IndexMap<VariableId, IndexSet<TypeOperand>>,
    /// Type operands that each variable must be assignable to.
    pub(in crate::check::state) type_upper_bounds: IndexMap<VariableId, IndexSet<TypeOperand>>,
    /// Static operands that must be assignable to each variable.
    pub(in crate::check::state) static_lower_bounds: IndexMap<VariableId, IndexSet<StaticOperand>>,
    /// Static operands that each variable must be assignable to.
    pub(in crate::check::state) static_upper_bounds: IndexMap<VariableId, IndexSet<StaticOperand>>,
    /// Defaults for generic argument variables.
    pub(in crate::check::state) generic_argument_defaults:
        IndexMap<VariableId, GenericArgumentDefault>,
    /// Variable constraints in collection order.
    pub(in crate::check::state) constraints: Vec<Constraint>,
    /// Constraint ids keyed by constraint identity.
    pub(in crate::check::state) constraints_by_constraint: IndexMap<Constraint, ConstraintId>,
    /// Constraints that no longer read unresolved dependencies.
    pub(in crate::check::state) completed_constraints: IndexSet<ConstraintId>,
    /// Tasks blocked by one dependency.
    pub(in crate::check::state) tasks_by_dependency: IndexMap<Dependency, SmallVec<[Task; 2]>>,
    /// Dependencies blocking one solver task.
    pub(in crate::check::state) dependencies_by_task: IndexMap<Task, SmallVec<[Dependency; 2]>>,
    /// Scheduled solver tasks.
    pub(in crate::check::state) worklist: IndexSet<Task>,
    /// Component post-solve obligations.
    pub(in crate::check::state) obligations: Vec<Obligation>,

    /// Generic templates keyed by template id.
    pub(in crate::check::state) generic_templates: IndexMap<GenericTemplateId, GenericTemplate>,
    /// Generic template ids keyed by source node.
    pub(in crate::check::state) generic_templates_by_source:
        IndexMap<dir::GlobalNodeIdAny, GenericTemplateId>,
    /// Generic template ids keyed by declaration symbol.
    pub(in crate::check::state) generic_templates_by_symbol:
        IndexMap<dir::GlobalSymbolId, GenericTemplateId>,
    /// Generic instances keyed by source node and owner.
    pub(in crate::check::state) generic_instances: IndexMap<GenericInstanceKey, GenericInstance>,
    /// Generic argument variables keyed by source node and parameter.
    pub(in crate::check::state) generic_arguments: IndexMap<GenericArgumentKey, VariableId>,
    /// Generic parameter ids keyed by parameter symbol.
    pub(in crate::check::state) generic_parameters_by_symbol:
        IndexMap<dir::GlobalSymbolId, GenericParameterId>,
    /// Generic parameters keyed by parameter id.
    pub(in crate::check::state) generic_parameters_by_id:
        IndexMap<GenericParameterId, GenericParameterBinding>,
    /// Declaration operands scanned for induced template generics.
    pub(in crate::check::state) generic_induction_sites: Vec<GenericInductionSite>,
    /// Variables that can induce template generics.
    pub(in crate::check::state) generic_inductions: IndexMap<VariableId, GenericInductionParameter>,

    /// Runtime calls resolved or rejected by solve.
    pub(in crate::check::state) calls: IndexMap<dir::GlobalNodeIdAny, CallDecision>,
    /// Runtime construct expressions resolved or rejected by solve.
    pub(in crate::check::state) constructs: IndexMap<dir::GlobalNodeIdAny, ConstructDecision>,
    /// Runtime operators resolved or rejected by solve.
    pub(in crate::check::state) operators: IndexMap<dir::GlobalNodeIdAny, OperatorDecision>,
    /// Runtime identity checks resolved or rejected by solve.
    pub(in crate::check::state) identities: IndexMap<dir::GlobalNodeIdAny, IdentityDecision>,
    /// Layout queries resolved or rejected by solve.
    pub(in crate::check::state) layouts: IndexMap<dir::GlobalNodeIdAny, LayoutDecision>,
    /// Runtime members resolved or rejected by solve.
    pub(in crate::check::state) members: IndexMap<dir::GlobalNodeIdAny, MemberDecision>,
    /// Patterns resolved or rejected by check.
    pub(in crate::check::state) patterns: IndexMap<dir::GlobalNodeIdAny, PatternDecision>,
    /// Contextual receivers resolved by check.
    pub(in crate::check::state) receivers: IndexMap<dir::GlobalNodeIdAny, ReceiverResolution>,
    /// Lexical names resolved by check.
    pub(in crate::check::state) names: IndexMap<dir::GlobalNodeIdAny, dir::NameResolution>,

    /// Trace events emitted by this segment.
    pub(in crate::check::state) events: Vec<CheckEvent>,
}

impl InferenceTable {
    /// Create an empty inference graph.
    pub(in crate::check) fn new() -> Self {
        Self {
            terms: TermTable::new(),
            segments: vec![InferenceSegment::new()],
            active_task: None,
        }
    }

    /// Return the current mutable inference segment.
    pub(in crate::check::state) fn current_mut(&mut self) -> &mut InferenceSegment {
        self.segments
            .last_mut()
            .unwrap_or_else(|| unreachable!("inference table has no current segment"))
    }

    /// Begin one speculative inference segment.
    pub(in crate::check) fn begin_probe(&mut self) -> InferenceProbe {
        let depth = self.segments.len();
        let variable_count = self.variable_count();
        let constraint_count = self.constraint_count();
        let term_cursor = self.terms.cursor();
        self.segments.push(InferenceSegment::new());

        InferenceProbe {
            depth,
            variable_count,
            constraint_count,
            term_cursor,
        }
    }

    /// Merge the current speculative segment into its parent.
    pub(in crate::check) fn commit_probe(&mut self, probe: InferenceProbe) -> CompilerResult<()> {
        if probe.depth + 1 != self.segments.len() {
            return Err(CompilerError::Internal {
                message: "inference probes must be committed in LIFO order".into(),
            });
        }

        let Some(segment) = self.segments.pop() else {
            return Err(CompilerError::Internal {
                message: "inference probe segment is missing".into(),
            });
        };
        let Some(parent) = self.segments.last_mut() else {
            return Err(CompilerError::Internal {
                message: "inference probe has no parent segment".into(),
            });
        };

        parent.merge(segment)?;

        Ok(())
    }

    /// Drop the current speculative segment.
    pub(in crate::check) fn drop_probe(&mut self, probe: InferenceProbe) -> CompilerResult<()> {
        if probe.depth + 1 != self.segments.len() {
            return Err(CompilerError::Internal {
                message: "inference probes must be dropped in LIFO order".into(),
            });
        }

        self.segments.pop();
        self.terms.truncate(probe.term_cursor);

        Ok(())
    }

    /// Return whether inference is currently in a speculative probe.
    pub(in crate::check) fn is_probing(&self) -> bool {
        self.segments.len() > 1
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
            generic_argument_defaults: IndexMap::new(),
            constraints: Vec::new(),
            constraints_by_constraint: IndexMap::new(),
            completed_constraints: IndexSet::new(),
            tasks_by_dependency: IndexMap::new(),
            dependencies_by_task: IndexMap::new(),
            worklist: IndexSet::new(),
            obligations: Vec::new(),

            generic_templates: IndexMap::new(),
            generic_templates_by_source: IndexMap::new(),
            generic_templates_by_symbol: IndexMap::new(),
            generic_instances: IndexMap::new(),
            generic_arguments: IndexMap::new(),
            generic_parameters_by_symbol: IndexMap::new(),
            generic_parameters_by_id: IndexMap::new(),
            generic_induction_sites: Vec::new(),
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
        merge_defaults(
            &mut self.generic_argument_defaults,
            child.generic_argument_defaults,
        )?;
        self.constraints.append(&mut child.constraints);
        self.constraints_by_constraint
            .extend(child.constraints_by_constraint);
        self.completed_constraints
            .extend(child.completed_constraints);
        for (dependency, tasks) in child.tasks_by_dependency {
            let target = self.tasks_by_dependency.entry(dependency).or_default();

            // merge dependent tasks once
            for task in tasks {
                if !target.contains(&task) {
                    target.push(task);
                }
            }
        }
        for (task, dependencies) in child.dependencies_by_task {
            let target = self.dependencies_by_task.entry(task).or_default();

            // merge task dependencies once
            for dependency in dependencies {
                if !target.contains(&dependency) {
                    target.push(dependency);
                }
            }
        }
        self.obligations.append(&mut child.obligations);

        self.worklist.extend(child.worklist);

        for (template_id, template) in child.generic_templates {
            let previous = self.generic_templates.insert(template_id, template);

            if previous.is_some() {
                return Err(CompilerError::Internal {
                    message: format!("generic template {template_id:?} is already inserted"),
                });
            }
        }
        self.generic_templates_by_source
            .extend(child.generic_templates_by_source);
        self.generic_templates_by_symbol
            .extend(child.generic_templates_by_symbol);
        self.generic_instances.extend(child.generic_instances);
        self.generic_arguments.extend(child.generic_arguments);
        self.generic_parameters_by_symbol
            .extend(child.generic_parameters_by_symbol);
        self.generic_parameters_by_id
            .extend(child.generic_parameters_by_id);
        self.generic_induction_sites
            .append(&mut child.generic_induction_sites);
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
fn merge_bounds<T>(
    parent: &mut IndexMap<VariableId, IndexSet<T>>,
    child: IndexMap<VariableId, IndexSet<T>>,
) where
    T: Eq + std::hash::Hash,
{
    for (variable, bounds) in child {
        parent.entry(variable).or_default().extend(bounds);
    }
}

/// Merge generic argument defaults without overwriting conflicting defaults.
fn merge_defaults(
    parent: &mut IndexMap<VariableId, GenericArgumentDefault>,
    child: IndexMap<VariableId, GenericArgumentDefault>,
) -> CompilerResult<()> {
    for (variable, default) in child {
        if let Some(existing) = parent.get(&variable) {
            if existing.parameter() != default.parameter() {
                return Err(CompilerError::Internal {
                    message: format!(
                        "check variable {variable:?} has conflicting defaults: existing {existing:?}, new {default:?}"
                    ),
                });
            }

            continue;
        }

        parent.insert(variable, default);
    }

    Ok(())
}
