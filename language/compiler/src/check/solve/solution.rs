use destack_core::{DenseGraph, FxIndexSet};
use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    Answer, BoundSide, CauseId, CheckEvent, CheckState, Constraint, Dependency, InferenceScope,
    Origin, Relation, TypeBound, VariableBounds, VariableRole, Widening, answer,
};
use crate::{CompilerError, CompilerResult};

/// Outcome of settling one task's scope.
#[derive(Debug)]
pub(in crate::check) struct ScopeSettlement {
    /// Whether settlement solved at least one component.
    pub(in crate::check) settled: bool,
    /// The owned variables that stay open.
    pub(in crate::check) open: SmallVec<[dir::TypeVariableId; 2]>,
}

/// How one bound depends on open variables during component solving.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BoundDependency {
    /// The bound contains no open variables.
    Closed,
    /// The bound is rooted at another variable in the same component.
    Root,
    /// The bound nests variables from the same component.
    Nested,
    /// The bound contains a variable outside the component.
    External,
}

impl CheckState<'_> {
    /// Allocate one inference variable.
    pub(in crate::check) fn allocate_variable(
        &mut self,
        origin: Origin,
        widening: Widening,
        role: VariableRole,
    ) -> dir::TypeVariableId {
        let variable = self.solver.allocate_variable(origin, widening, role);
        self.record_event(CheckEvent::VariableAllocated { variable, widening });

        variable
    }

    /// Record the declared parameter bound one variable discharges when solved.
    pub(in crate::check) fn set_variable_parameter_bound(
        &mut self,
        variable: dir::TypeVariableId,
        bound: dir::GlobalTypeId,
        cause: CauseId,
    ) {
        self.solver
            .set_variable_parameter_bound(variable, bound, cause);
    }

    /// Record the declared default completing one variable when inference stays dry.
    pub(in crate::check) fn set_variable_default(
        &mut self,
        variable: dir::TypeVariableId,
        default: dir::GlobalTypeId,
    ) {
        self.solver.set_variable_default(variable, default);
    }

    /// Return one canonical open variable, or none when the variable is solved.
    pub(in crate::check) fn open_variable(
        &self,
        variable: dir::TypeVariableId,
    ) -> CompilerResult<Option<dir::TypeVariableId>> {
        let state = self.solver.variable(variable)?;

        Ok(state.solution.is_none().then_some(variable))
    }

    /// Return the special role attached to one open variable.
    pub(in crate::check) fn variable_role(
        &self,
        variable: dir::TypeVariableId,
    ) -> CompilerResult<VariableRole> {
        let Some(variable) = self.open_variable(variable)? else {
            return Ok(VariableRole::Regular);
        };

        self.solver.variable_role(variable)
    }

    /// Settle every variable one completed task touched.
    pub(in crate::check) fn settle_task(
        &mut self,
        scope: InferenceScope,
        mark: usize,
        is_defaulting: bool,
    ) -> CompilerResult<ScopeSettlement> {
        // solve dependency chains to quiescence, dependencies first
        let mut settled = false;
        loop {
            // relate closed evidence onward before solving components
            let mut forwarded = false;
            let adopted = self.solver.adopted_since(mark, scope);
            for index in scope.indices(self.solver.variable_count()) {
                let variable = dir::TypeVariableId(index as u32);
                if self.solver.variable(variable)?.solution.is_none() {
                    forwarded |= self.forward_evidence(variable)?;
                }
            }
            for variable in &adopted {
                if self.solver.variable(*variable)?.solution.is_none() {
                    forwarded |= self.forward_evidence(*variable)?;
                }
            }

            // solve each owned dependency component under the caller's phase
            let components = self.scope_variable_components(scope)?;
            let mut solved = false;
            for variables in &components {
                if variables.is_empty() {
                    continue;
                }
                solved |= self.solve_variable_component(variables, is_defaulting)?;
            }

            // settle adopted holes from evidence, leaving defaults to their owners
            for variable in &adopted {
                if self.solver.variable(*variable)?.solution.is_none() {
                    solved |= self.solve_variable_component(&[*variable], false)?;
                }
            }

            if !solved && !forwarded {
                break;
            }
            settled = true;
        }

        // collect the owned variables that stay open
        let mut open = SmallVec::new();
        for index in scope.indices(self.solver.variable_count()) {
            let variable = dir::TypeVariableId(index as u32);
            if self.solver.variable(variable)?.solution.is_none() {
                open.push(variable);
            }
        }

        Ok(ScopeSettlement { settled, open })
    }

    /// Complete every open variable of one failed scope with the error type.
    pub(in crate::check) fn poison_scope(&mut self, scope: InferenceScope) -> CompilerResult<()> {
        // retain every solution available from evidence or declared defaults
        let mark = self.solver.snapshot_watermark();
        let settlement = self.settle_task(scope, mark, true)?;

        // taint only variables the failed task could not determine
        for variable in settlement.open {
            if self.solver.variable(variable)?.solution.is_some() {
                continue;
            }
            let origin_id = self.solver.variable(variable)?.origin;
            let origin = self.solver.origin(origin_id);
            let error = self.intern_type(origin.module(), dir::Type::Error)?;
            self.commit_solution(variable, error)?;
        }

        Ok(())
    }

    /// Settle one produced cell from its accumulated evidence.
    pub(in crate::check) fn settle_produced(
        &mut self,
        variable: dir::TypeVariableId,
    ) -> CompilerResult<bool> {
        if self.solver.variable(variable)?.solution.is_some() {
            return Ok(false);
        }

        self.solve_variable_component(&[variable], true)
    }

    /// Relate one variable's closed lower evidence onto its open uppers.
    ///
    /// Evidence flows through open slots: candidates relate onward into
    /// the inference variables of the slot's own targets.
    fn forward_evidence(&mut self, variable: dir::TypeVariableId) -> CompilerResult<bool> {
        // collect the closed evidence and the open targets
        let mut lowers = SmallVec::<[TypeBound; 2]>::new();
        let mut uppers = SmallVec::<[TypeBound; 2]>::new();
        for bound in self
            .solver
            .variables
            .side_bounds(variable, BoundSide::Lower)?
        {
            if self.type_variables(bound.ty)?.is_empty() {
                lowers.push(bound);
            }
        }
        for bound in self
            .solver
            .variables
            .side_bounds(variable, BoundSide::Upper)?
        {
            if !self.type_variables(bound.ty)?.is_empty() {
                uppers.push(bound);
            }
        }
        if lowers.is_empty() || uppers.is_empty() {
            return Ok(false);
        }

        // evidence flows onward as first-class constraints, so an
        //  unsatisfiable transit rejects the task that induced it
        let bounds_before = self.solver.variables.bound_count();
        let constraints_before = self.solver.constraint_count();
        for upper in &uppers {
            for lower in &lowers {
                let relation = match upper.relation {
                    Relation::Equal => lower.relation,
                    relation => relation,
                };
                self.push_constraint(Constraint::derived(
                    relation,
                    lower.ty,
                    upper.ty,
                    upper.cause,
                ));
            }
        }

        let progressed = self.solver.variables.bound_count() > bounds_before
            || self.solver.constraint_count() > constraints_before;

        Ok(progressed)
    }

    /// Partition the open variables of one scope into dependency components.
    fn scope_variable_components(
        &mut self,
        scope: InferenceScope,
    ) -> CompilerResult<Vec<SmallVec<[dir::TypeVariableId; 2]>>> {
        let variable_count = self.solver.variable_count();
        let indices = scope.indices(variable_count);
        let first_index = indices.start;
        let first_variable = first_index as u32;
        let mut edge_offsets = Vec::with_capacity(indices.len() + 1);
        let mut edge_targets = Vec::<u32>::new();
        edge_offsets.push(0);

        // build dependencies from every open root to variables in its bounds
        for index in indices.clone() {
            let variable = dir::TypeVariableId(index as u32);
            let state = self.solver.variable(variable)?;
            let edge_start = edge_targets.len();
            if state.solution.is_none() {
                for side in [BoundSide::Lower, BoundSide::Upper] {
                    for bound in self.solver.variables.side_bounds(variable, side)? {
                        for dependency in self.type_variables(bound.ty)? {
                            if scope.owns(dependency)
                                && !edge_targets[edge_start..]
                                    .contains(&(dependency.0 - first_variable))
                            {
                                edge_targets.push(dependency.0 - first_variable);
                            }
                        }
                    }
                }
            }
            edge_offsets.push(edge_targets.len() as u32);
        }

        // partition mutually dependent variables for simultaneous settlement
        let graph = DenseGraph::new(&edge_offsets, &edge_targets);
        let partition = graph.strongly_connected_components();
        let mut components =
            vec![SmallVec::<[dir::TypeVariableId; 2]>::new(); partition.component_count() as usize];
        for index in indices {
            let variable = dir::TypeVariableId(index as u32);
            let state = self.solver.variable(variable)?;
            if state.solution.is_none() {
                let component = partition.component(index - first_index) as usize;
                components[component].push(variable);
            }
        }

        Ok(components)
    }

    /// Solve one mutually dependent variable component.
    fn solve_variable_component(
        &mut self,
        variables: &[dir::TypeVariableId],
        is_defaulting: bool,
    ) -> CompilerResult<bool> {
        let mut lower_candidates = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        let mut upper_candidates = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        let mut closed_upper = SmallVec::<[TypeBound; 4]>::new();
        let mut defaults = SmallVec::<[dir::GlobalTypeId; 2]>::new();
        let mut has_external_dependency = false;
        let mut has_recursive_bound = false;
        let mut has_return_cycle = false;
        let mut equation = None;
        let mut has_open_equation = false;

        // collect proper bounds while retaining structural cycles as errors
        for variable in variables {
            let state = *self.solver.variable(*variable)?;
            let is_return = self.solver.variable_role(*variable)? == VariableRole::Return;
            let lower = self
                .solver
                .variables
                .side_bounds(*variable, BoundSide::Lower)?
                .collect::<SmallVec<[TypeBound; 2]>>();
            let upper = self
                .solver
                .variables
                .side_bounds(*variable, BoundSide::Upper)?
                .collect::<SmallVec<[TypeBound; 2]>>();

            for bound in &lower {
                match self.bound_dependency(bound.ty, variables)? {
                    BoundDependency::Closed => {
                        lower_candidates.extend(self.lower_solution_candidates(
                            std::slice::from_ref(bound),
                            state.widening,
                        )?)
                    }
                    BoundDependency::Root => {
                        has_return_cycle |= is_return && bound.relation != Relation::Equal
                    }
                    BoundDependency::Nested => {
                        has_recursive_bound = true;
                        has_return_cycle |= is_return && bound.relation != Relation::Equal;
                    }
                    BoundDependency::External => has_external_dependency = true,
                }
            }
            for bound in &upper {
                match self.bound_dependency(bound.ty, variables)? {
                    BoundDependency::Closed => {
                        closed_upper.push(*bound);
                        if bound.relation == Relation::Equal {
                            equation = Some(bound.ty);
                        }
                        if bound.relation != Relation::Satisfies {
                            upper_candidates.push(bound.ty);
                        }
                    }
                    BoundDependency::Root => {
                        has_return_cycle |= is_return && bound.relation != Relation::Equal
                    }
                    BoundDependency::Nested => {
                        has_recursive_bound = true;
                        has_return_cycle |= is_return && bound.relation != Relation::Equal;
                        has_open_equation |= bound.relation == Relation::Equal;
                    }
                    BoundDependency::External => {
                        has_external_dependency = true;
                        has_open_equation |= bound.relation == Relation::Equal;
                    }
                }
            }

            if is_defaulting {
                let origin = self.solver.origin(state.origin);
                let default = match self.solver.variables.variable_default(*variable) {
                    Some(default) => Some(default),
                    None => match self.variable_memory_parameter(*variable)? {
                        Some(dir::MemoryParameter::Lifetime) => {
                            let memory = dir::MemoryLiteral::Lifetime(dir::Lifetime::Frame);

                            Some(self.intern_type(origin.module(), dir::Type::Memory(memory))?)
                        }
                        Some(dir::MemoryParameter::Access) => {
                            let memory = dir::MemoryLiteral::Access(dir::Access::Readonly);

                            Some(self.intern_type(origin.module(), dir::Type::Memory(memory))?)
                        }
                        Some(
                            dir::MemoryParameter::Ownership
                            | dir::MemoryParameter::Place
                            | dir::MemoryParameter::Space,
                        )
                        | None => None,
                    },
                };
                if let Some(default) = default
                    && self.type_variables(default)?.is_empty()
                {
                    defaults.push(default);
                }
            }
        }

        // choose the component solution from lower evidence, then upper context, then defaults
        let root = variables[0];
        let state = *self.solver.variable(root)?;
        let origin = self.solver.origin(state.origin);
        let is_unconstrained_recursion = has_recursive_bound
            && lower_candidates.is_empty()
            && upper_candidates.is_empty()
            && !has_external_dependency;
        // a closed equation pins the solution; open equations, return
        //  cycles, and unconstrained recursion defer
        let solution = if let Some(equation) = equation {
            Some(equation)
        } else if has_open_equation || has_return_cycle || is_unconstrained_recursion {
            None
        } else if !lower_candidates.is_empty() {
            let joined = self.best_common(root, &lower_candidates)?;
            let override_ =
                self.widens_upper_override(origin, joined, &closed_upper, &lower_candidates)?;
            let Answer::Ready(solution) = override_ else {
                return Err(CompilerError::Internal {
                    message: format!(
                        "closed variable component {variables:?} has a pending widening decision"
                    ),
                });
            };

            Some(solution)
        } else if let [candidate] = upper_candidates.as_slice() {
            Some(*candidate)
        } else if !upper_candidates.is_empty() {
            Some(self.intersect_bounds(root, &upper_candidates)?)
        } else if has_external_dependency {
            None
        } else if let [default] = defaults.as_slice() {
            Some(*default)
        } else if !defaults.is_empty() {
            Some(self.best_common(root, &defaults)?)
        } else {
            None
        };

        // a structural cycle without any proper candidate is an infinite inferred type
        let solution = match solution {
            Some(solution) => solution,
            None if (has_return_cycle || is_unconstrained_recursion) && variables.len() == 1 => {
                let error = self.circular_type_error(origin)?;
                self.report(origin.module(), error);

                self.intern_type(origin.module(), dir::Type::Error)?
            }
            None => return Ok(false),
        };
        let solution = self.reduce_named_head(origin, solution)?;
        let Answer::Ready(solution) = solution else {
            return Err(CompilerError::Internal {
                message: format!(
                    "closed variable component {variables:?} has a pending solution head"
                ),
            });
        };

        // commit canonical memory literals for memory variables
        let solution = match self.variable_memory_parameter(root)? {
            Some(kind) => self.normalize_memory_component(origin, solution, kind)?,
            None => solution,
        };

        // every variable in one dependency cycle receives the same fixed type
        for variable in variables {
            self.commit_solution(*variable, solution)?;
        }

        Ok(true)
    }

    /// Classify one bound's dependency on a variable component.
    fn bound_dependency(
        &self,
        ty: dir::GlobalTypeId,
        variables: &[dir::TypeVariableId],
    ) -> CompilerResult<BoundDependency> {
        let dependencies = self.type_variables(ty)?;
        if dependencies.is_empty() {
            return Ok(BoundDependency::Closed);
        }

        // reject components that still depend on an outside variable
        for dependency in &dependencies {
            if !variables.contains(dependency) {
                return Ok(BoundDependency::External);
            }
        }

        // distinguish aliases from recursive type construction
        let dependency = self.root_variable(ty)?;
        if dependency.is_some_and(|dependency| variables.contains(&dependency)) {
            Ok(BoundDependency::Root)
        } else {
            Ok(BoundDependency::Nested)
        }
    }

    /// Absorb one lower-bound join into an identity-restricted upper.
    ///
    /// A join built from argument bounds cannot widen into a `Widens`
    /// upper unless it already matches it, so when every lower still
    /// assigns into the upper, the upper decides the solution and the
    /// arguments convert at their own value positions.
    fn widens_upper_override(
        &mut self,
        origin: Origin,
        joined: dir::GlobalTypeId,
        upper: &[TypeBound],
        lower_candidates: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        for bound in upper {
            if bound.relation != Relation::Widens || !self.type_variables(bound.ty)?.is_empty() {
                continue;
            }
            if answer!(self.decide_relation(origin, Relation::Widens, joined, bound.ty)?) {
                continue;
            }

            // the join misses this upper: absorb when every lower assigns
            let mut absorbs = true;
            for candidate in lower_candidates {
                if !answer!(self.decide_relation(
                    origin,
                    Relation::Assignable,
                    *candidate,
                    bound.ty,
                )?) {
                    absorbs = false;

                    break;
                }
            }
            if absorbs {
                return Ok(Answer::Ready(self.settled_root(bound.ty)?));
            }
        }

        Ok(Answer::Ready(joined))
    }

    /// Choose the lower-bound solution candidates under one widening policy.
    fn lower_solution_candidates(
        &mut self,
        lower: &[TypeBound],
        widening: Widening,
    ) -> CompilerResult<SmallVec<[dir::GlobalTypeId; 2]>> {
        let mut candidates = lower.iter().map(|bound| bound.ty).collect();

        // let equality-pinned bounds choose the solution outright
        let pinned: SmallVec<[dir::GlobalTypeId; 2]> = lower
            .iter()
            .filter(|bound| bound.relation == Relation::Equal)
            .map(|bound| bound.ty)
            .collect();
        if !pinned.is_empty() {
            candidates = pinned;
        }

        // widen fresh literal candidates under the variable's policy
        for candidate in candidates.iter_mut() {
            let written = lower
                .iter()
                .any(|bound| bound.ty == *candidate && bound.relation == Relation::Writable);
            let flowed = lower.iter().any(|bound| {
                bound.ty == *candidate
                    && matches!(
                        bound.relation,
                        Relation::Assignable | Relation::Writable | Relation::Castable
                    )
            });
            let pinned = lower
                .iter()
                .any(|bound| bound.ty == *candidate && bound.relation == Relation::Equal);
            let widens = match widening {
                Widening::Always => flowed && !pinned,
                Widening::WhenWritten => written,
                Widening::Never => false,
            };
            if widens {
                *candidate = self.widen_type(*candidate)?;
            }
        }

        Ok(candidates)
    }

    /// Collect the unsolved variables one type transitively references.
    pub(in crate::check) fn type_variables(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<SmallVec<[dir::TypeVariableId; 2]>> {
        let mut variables = SmallVec::new();
        let mut pending = SmallVec::<[dir::GlobalTypeId; 8]>::new();
        let mut visited = FxIndexSet::default();
        pending.push(id);

        // scan the type graph without following symbol references;
        //  solutions may be cyclic, so every id visits exactly once
        while let Some(id) = pending.pop() {
            if !visited.insert(id) {
                continue;
            }
            let ty = self.ty(id)?;

            // follow solved variables and collect open variables once
            if let dir::Type::Variable(variable) = ty {
                if let Some(solution) = self.solver.solution(variable)? {
                    pending.push(solution);
                } else if let Some(variable) = self.open_variable(variable)?
                    && !variables.contains(&variable)
                {
                    variables.push(variable);
                }

                continue;
            }

            self.for_each_type_child(id.module_id, &ty, |child| pending.push(child))?;
        }

        Ok(variables)
    }

    /// Return whether one type contains a variable root.
    fn type_contains_variable(
        &self,
        ty: dir::GlobalTypeId,
        variable: dir::TypeVariableId,
    ) -> CompilerResult<bool> {
        for dependency in self.type_variables(ty)? {
            if dependency == variable {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Record one variable solution and wake its waiters.
    pub(in crate::check) fn commit_solution(
        &mut self,
        variable: dir::TypeVariableId,
        solution: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let solution = self.settled_root(solution)?;

        if self.root_variable(solution)?.is_some() {
            return Err(CompilerError::Internal {
                message: format!(
                    "check variable {variable:?} received open solution {} ({solution:?})",
                    self.format_type(solution),
                ),
            });
        }

        // circular solutions poison the variable with an error type
        if self.type_contains_variable(solution, variable)? {
            let origin_id = self.solver.variable(variable)?.origin;
            let origin = self.solver.origin(origin_id);
            let error = self.circular_type_error(origin)?;
            self.report(origin.module(), error);
            let poisoned = self.intern_type(origin.module(), dir::Type::Error)?;

            return self.commit_solution(variable, poisoned);
        }

        // require exactly one solution per variable
        if let Some(previous) = self.solver.variable(variable)?.solution {
            if previous != solution {
                return Err(CompilerError::Internal {
                    message: format!("check variable {variable:?} was solved twice"),
                });
            }

            return Ok(());
        }
        let bounds = VariableBounds {
            lower: self
                .solver
                .variables
                .side_bounds(variable, BoundSide::Lower)?
                .collect(),
            upper: self
                .solver
                .variables
                .side_bounds(variable, BoundSide::Upper)?
                .collect(),
        };
        self.solver.variable_mut(variable)?.solution = Some(solution);

        // wake tasks parked on the solved variable
        let waiters = self.solver.wake(Dependency::Variable(variable));
        for waiter in waiters.iter().cloned() {
            self.queue_task(waiter);
        }

        self.record_event(CheckEvent::VariableSolved {
            variable,
            bounds: bounds.clone(),
            solution,
            waiters: waiters.len(),
        });

        // accumulated bounds discharge against the committed solution
        for bound in &bounds.lower {
            self.discharge_bound(BoundSide::Lower, bound, solution)?;
        }
        for bound in &bounds.upper {
            self.discharge_bound(BoundSide::Upper, bound, solution)?;
        }

        // the declared parameter bound discharges against the committed solution
        if let Some((bound, cause)) = self.solver.variables.parameter_bound(variable) {
            match self.constrain_type(cause, Relation::Satisfies, solution, bound)? {
                Answer::Ready(true) => {}
                Answer::Ready(false) | Answer::Pending(_) => {
                    self.push_constraint(Constraint::r#type(
                        Relation::Satisfies,
                        solution,
                        bound,
                        cause,
                    ));
                }
            }
        }

        Ok(())
    }

    /// Discharge one accumulated bound against a committed solution.
    fn discharge_bound(
        &mut self,
        side: BoundSide,
        bound: &TypeBound,
        solution: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let (source, target) = match side {
            BoundSide::Lower => (bound.ty, solution),
            BoundSide::Upper => (solution, bound.ty),
        };

        // relations that cannot settle in place park as constraints
        match self.constrain_type(bound.cause, bound.relation, source, target)? {
            Answer::Ready(true) => {}
            Answer::Ready(false) | Answer::Pending(_) => {
                self.push_constraint(Constraint::r#type(
                    bound.relation,
                    source,
                    target,
                    bound.cause,
                ));
            }
        }

        Ok(())
    }

    /// Push one lower bound onto a variable and schedule it.
    pub(in crate::check) fn push_lower_bound(
        &mut self,
        variable: dir::TypeVariableId,
        cause: CauseId,
        bound: dir::GlobalTypeId,
        relation: Relation,
    ) -> CompilerResult<()> {
        self.push_variable_bound(variable, BoundSide::Lower, cause, bound, relation)
    }

    /// Push one upper bound onto a variable and schedule it.
    pub(in crate::check) fn push_upper_bound(
        &mut self,
        variable: dir::TypeVariableId,
        cause: CauseId,
        bound: dir::GlobalTypeId,
        relation: Relation,
    ) -> CompilerResult<()> {
        self.push_variable_bound(variable, BoundSide::Upper, cause, bound, relation)
    }

    /// Push one bound onto a variable side and schedule it.
    fn push_variable_bound(
        &mut self,
        variable: dir::TypeVariableId,
        side: BoundSide,
        cause: CauseId,
        bound: dir::GlobalTypeId,
        relation: Relation,
    ) -> CompilerResult<()> {
        // late bounds against a solved variable discharge as relation checks
        if let Some(solution) = self.solver.variable(variable)?.solution {
            let late = TypeBound::new(bound, relation, cause);
            self.discharge_bound(side, &late, solution)?;

            return Ok(());
        }

        // collect the bound and schedule the variable
        let bound = TypeBound::new(bound, relation, cause);
        if self.solver.push_bound(variable, side, bound)? {
            self.record_event(match side {
                BoundSide::Lower => CheckEvent::LowerBoundPushed { variable, bound },
                BoundSide::Upper => CheckEvent::UpperBoundPushed { variable, bound },
            });
        }

        Ok(())
    }
}
