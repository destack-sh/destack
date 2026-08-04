use destack_core::{DenseGraph, FxIndexSet};
use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    Answer, BoundSide, CauseId, CheckEvent, CheckState, Constraint, Dependency, InferenceScope,
    Origin, Relation, TypeBound, VariableBounds, VariableRole, VariableState, Widening,
};
use crate::{CompilerError, CompilerResult};

/// How one bound depends on open variables during component solving.
#[derive(Debug, Clone, PartialEq, Eq)]
enum BoundDependency {
    /// The bound contains no open variables.
    Closed,
    /// The bound aliases another variable in the same component.
    Alias,
    /// The bound constructs a type containing variables from the same component.
    Recursive,
    /// The bound contains variables outside the component.
    External(SmallVec<[dir::TypeVariableId; 2]>),
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
        let variable = self.solver.alias_root(variable)?;
        let state = self.solver.variable(variable)?;

        Ok(state.state.is_open().then_some(variable))
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

    /// Settle every variable in one scope from its accumulated bounds.
    pub(in crate::check) fn settle_scope(
        &mut self,
        scope: InferenceScope,
    ) -> CompilerResult<Answer<bool>> {
        let adopted = self.solver.adopted_variables(scope)?;
        let mut blockers = SmallVec::<[Dependency; 2]>::new();

        // solve each owned dependency component
        let components = self.variable_components(scope)?;
        let mut solved = false;
        for variables in &components {
            if variables.is_empty() {
                continue;
            }
            match self.solve_component(variables, &[])? {
                Answer::Ready(progress) => solved |= progress,
                Answer::Pending(pending) => blockers.extend(pending),
            }
        }

        // settle preexisting variables bounded by this scope
        for variable in &adopted {
            if self.solver.variable(*variable)?.state.is_open() {
                match self.solve_component(&[*variable], &[])? {
                    Answer::Ready(progress) => solved |= progress,
                    Answer::Pending(pending) => blockers.extend(pending),
                }
            }
        }

        // dependencies within this scope settle in a later component pass
        blockers.retain(|dependency| {
            !matches!(dependency, Dependency::Variable(variable) if scope.owns(*variable))
        });

        if solved || blockers.is_empty() {
            Ok(Answer::Ready(solved))
        } else {
            Ok(Answer::pending(blockers))
        }
    }

    /// Complete every unconstrained variable in one scope from its declared default.
    pub(in crate::check) fn default_scope(
        &mut self,
        scope: InferenceScope,
    ) -> CompilerResult<Answer<bool>> {
        // yield newly available bounds to the task queue before defaulting
        match self.settle_scope(scope)? {
            Answer::Ready(true) => return Ok(Answer::Ready(true)),
            Answer::Ready(false) => {}
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        }

        // declared defaults settle before widening finalizes literals, so
        //  awaiting selections resume before literals commit their widths
        let components = self.variable_components(scope)?;
        let mut defaulted = false;
        let mut blockers = SmallVec::<[Dependency; 2]>::new();
        for declared in [true, false] {
            for variables in &components {
                if variables.is_empty() {
                    continue;
                }
                if self.component_defaults(variables)?.is_empty() == declared {
                    continue;
                }
                match self.default_component(variables)? {
                    Answer::Ready(progress) => defaulted |= progress,
                    Answer::Pending(pending) => blockers.extend(pending),
                }
            }

            // the widening pass waits for a quiescence without declared progress
            if declared && defaulted {
                return Ok(Answer::Ready(true));
            }
        }

        // dependencies within this scope default in a later component pass
        blockers.retain(|dependency| {
            !matches!(dependency, Dependency::Variable(variable) if scope.owns(*variable))
        });

        if defaulted || blockers.is_empty() {
            Ok(Answer::Ready(defaulted))
        } else {
            Ok(Answer::pending(blockers))
        }
    }

    /// Return the open variables owned by one inference scope.
    pub(in crate::check) fn open_variables(
        &self,
        scope: InferenceScope,
    ) -> CompilerResult<SmallVec<[dir::TypeVariableId; 2]>> {
        let mut open = SmallVec::new();
        for index in scope.indices(self.solver.variable_count()) {
            let variable = dir::TypeVariableId(index as u32);
            if self.solver.variable(variable)?.state.is_open() {
                open.push(variable);
            }
        }

        Ok(open)
    }

    /// Partition the open variables of one scope into dependency components.
    fn variable_components(
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
            if state.state.is_open() {
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
            if state.state.is_open() {
                let component = partition.component(index - first_index) as usize;
                components[component].push(variable);
            }
        }

        Ok(components)
    }

    /// Complete one variable component from its declared defaults.
    fn default_component(
        &mut self,
        variables: &[dir::TypeVariableId],
    ) -> CompilerResult<Answer<bool>> {
        let defaults = self.component_defaults(variables)?;

        self.solve_component(variables, &defaults)
    }

    /// Collect the declared and memory defaults of one variable component.
    fn component_defaults(
        &mut self,
        variables: &[dir::TypeVariableId],
    ) -> CompilerResult<SmallVec<[dir::GlobalTypeId; 2]>> {
        let mut defaults = SmallVec::<[dir::GlobalTypeId; 2]>::new();
        for variable in variables {
            let state = *self.solver.variable(*variable)?;
            let _origin = self.solver.origin(state.origin);
            let default = match self.solver.variables.variable_default(*variable) {
                Some(default) => Some(default),
                None => match self.variable_memory_parameter(*variable)? {
                    Some(dir::MemoryParameter::Lifetime) => {
                        let memory = dir::MemoryLiteral::Lifetime(dir::Lifetime::Frame);

                        Some(self.intern_type(dir::Type::Memory(memory))?)
                    }
                    Some(dir::MemoryParameter::Access) => {
                        let memory = dir::MemoryLiteral::Access(dir::Access::Readonly);

                        Some(self.intern_type(dir::Type::Memory(memory))?)
                    }
                    Some(
                        dir::MemoryParameter::Ownership
                        | dir::MemoryParameter::Place
                        | dir::MemoryParameter::Space,
                    )
                    | None => None,
                },
            };
            if let Some(default) = default {
                // require a default without an open or component-own variable
                let mut open = false;
                for variable in self.type_variables(default)? {
                    if variables.contains(&variable) || self.solver.solution(variable)?.is_none() {
                        open = true;
                        break;
                    }
                }
                if !open {
                    defaults.push(default);
                }
            }
        }

        Ok(defaults)
    }

    /// Solve one variable component from its bounds and declared defaults.
    fn solve_component(
        &mut self,
        variables: &[dir::TypeVariableId],
        defaults: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<bool>> {
        let mut closed_lower = SmallVec::<[TypeBound; 4]>::new();
        let mut closed_upper = SmallVec::<[TypeBound; 4]>::new();
        let mut lower_types = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        let mut contextual_types = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        let mut blockers = SmallVec::<[Dependency; 2]>::new();
        let mut has_recursive_bound = false;
        let mut has_return_cycle = false;
        let mut equation = None;
        let mut has_open_equation = false;

        // collect proper bounds while retaining structural cycles as errors
        for variable in variables {
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
            let mut variable_lower = SmallVec::<[TypeBound; 2]>::new();

            for bound in &lower {
                match self.bound_dependency(bound.ty, variables)? {
                    BoundDependency::Closed => {
                        closed_lower.push(*bound);
                        variable_lower.push(*bound);
                    }
                    BoundDependency::Alias => {
                        has_return_cycle |= is_return && bound.relation != Relation::Equal
                    }
                    BoundDependency::Recursive => {
                        has_recursive_bound = true;
                        has_return_cycle |= is_return && bound.relation != Relation::Equal;
                    }
                    BoundDependency::External(variables) => {
                        blockers.extend(variables.into_iter().map(Dependency::Variable));
                    }
                }
            }

            // apply this variable's literal policy to its closed bounds
            let has_equation = variable_lower
                .iter()
                .any(|bound| bound.relation == Relation::Equal);
            let candidates = variable_lower
                .iter()
                .map(|bound| bound.ty)
                .collect::<SmallVec<[_; 2]>>();
            let state = *self.solver.variable(*variable)?;
            let origin = self.solver.origin(state.origin);
            let widens = !has_equation
                && match state.widening {
                    Widening::Never => false,
                    Widening::Aggregate => false,
                    Widening::Multiple => self.has_distinct_types(origin, &candidates)?,
                    Widening::Always => true,
                };

            for bound in variable_lower {
                if has_equation && bound.relation != Relation::Equal {
                    continue;
                }
                let ty = if widens
                    && matches!(bound.relation, Relation::Assignable | Relation::Castable)
                {
                    self.widen_type(bound.ty)?
                } else {
                    bound.ty
                };
                lower_types.push(ty);
            }

            for bound in &upper {
                match self.bound_dependency(bound.ty, variables)? {
                    BoundDependency::Closed => {
                        closed_upper.push(*bound);
                        if bound.relation == Relation::Equal {
                            equation = Some(bound.ty);
                        }
                        if matches!(bound.relation, Relation::Assignable | Relation::Widens) {
                            contextual_types.push(bound.ty);
                        }
                    }
                    BoundDependency::Alias => {
                        has_return_cycle |= is_return && bound.relation != Relation::Equal
                    }
                    BoundDependency::Recursive => {
                        has_recursive_bound = true;
                        has_return_cycle |= is_return && bound.relation != Relation::Equal;
                        has_open_equation |= bound.relation == Relation::Equal;
                    }
                    BoundDependency::External(variables) => {
                        blockers.extend(variables.into_iter().map(Dependency::Variable));
                        has_open_equation |= bound.relation == Relation::Equal;
                    }
                }
            }
        }

        // choose the component solution from lower bounds, then upper context, then defaults
        let root = variables[0];
        let state = *self.solver.variable(root)?;
        let origin = self.solver.origin(state.origin);
        let is_unconstrained_recursion = has_recursive_bound
            && lower_types.is_empty()
            && contextual_types.is_empty()
            && blockers.is_empty();

        // form the preferred lower and contextual solutions once
        let lower = match lower_types.is_empty() {
            true => None,
            false => Some(self.best_common(root, &lower_types)?),
        };
        let contextual = match contextual_types.as_slice() {
            [] => None,
            [single] => Some(*single),
            _ => Some(self.intersect_bounds(root, &contextual_types)?),
        };

        // a closed equation pins the solution; open equations defer
        //  until their composite closes
        let solution = if let Some(equation) = equation {
            Some(equation)
        } else if has_open_equation || has_return_cycle || is_unconstrained_recursion {
            None
        } else if let Some(lower) = lower {
            if self.solution_satisfies_bounds(origin, lower, &closed_lower, &closed_upper)? {
                Some(lower)
            } else if let Some(contextual) = contextual
                && self.solution_satisfies_bounds(
                    origin,
                    contextual,
                    &closed_lower,
                    &closed_upper,
                )?
            {
                Some(contextual)
            } else {
                Some(lower)
            }
        } else if let Some(contextual) = contextual {
            Some(contextual)
        } else if !blockers.is_empty() {
            None
        } else if let [default] = defaults {
            Some(*default)
        } else if !defaults.is_empty() {
            Some(self.best_common(root, defaults)?)
        } else {
            None
        };

        // a structural cycle without any proper candidate is an infinite inferred type
        let solution = match solution {
            Some(solution) => solution,
            None if !has_return_cycle && is_unconstrained_recursion && variables.len() == 1 => {
                let error = self.circular_type_error(origin)?;
                self.report(origin.module(), error);

                self.intern_type(dir::Type::Error)?
            }
            None if !blockers.is_empty() => return Ok(Answer::pending(blockers)),
            None => return Ok(Answer::Ready(false)),
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

        Ok(Answer::Ready(true))
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

        // retain variables outside this component as explicit blockers
        let mut external = SmallVec::new();
        for dependency in &dependencies {
            if !variables.contains(dependency) && !external.contains(dependency) {
                external.push(*dependency);
            }
        }
        if !external.is_empty() {
            return Ok(BoundDependency::External(external));
        }

        // distinguish aliases from recursive type construction
        let dependency = self.root_variable(ty)?;
        if dependency.is_some_and(|dependency| variables.contains(&dependency)) {
            Ok(BoundDependency::Alias)
        } else {
            Ok(BoundDependency::Recursive)
        }
    }

    /// Return whether one closed solution satisfies every collected bound.
    fn solution_satisfies_bounds(
        &mut self,
        origin: Origin,
        solution: dir::GlobalTypeId,
        lower: &[TypeBound],
        upper: &[TypeBound],
    ) -> CompilerResult<bool> {
        // require every produced value to flow into the solution
        for bound in lower {
            match self.decide_relation(origin, bound.relation, bound.ty, solution)? {
                Answer::Ready(true) => {}
                Answer::Ready(false) => return Ok(false),
                Answer::Pending(blockers) => {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "closed lower bound depends on unfinished state {blockers:?}"
                        ),
                    });
                }
            }
        }

        // require the solution to flow into every expectation
        for bound in upper {
            match self.decide_relation(origin, bound.relation, solution, bound.ty)? {
                Answer::Ready(true) => {}
                Answer::Ready(false) => return Ok(false),
                Answer::Pending(blockers) => {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "closed upper bound depends on unfinished state {blockers:?}"
                        ),
                    });
                }
            }
        }

        Ok(true)
    }

    /// Return whether a closed type list contains unequal types.
    fn has_distinct_types(
        &mut self,
        origin: Origin,
        types: &[dir::GlobalTypeId],
    ) -> CompilerResult<bool> {
        let Some(first) = types.first() else {
            return Ok(false);
        };
        let first = self.settled_root(*first)?;

        // compare each remaining candidate with the first
        for ty in &types[1..] {
            let ty = self.settled_root(*ty)?;
            match self.decide_equal(origin, first, ty)? {
                Answer::Ready(true) => {}
                Answer::Ready(false) => return Ok(true),
                Answer::Pending(blockers) => {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "closed inference candidates depend on unfinished state {blockers:?}"
                        ),
                    });
                }
            }
        }

        Ok(false)
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
        let variable = self.solver.alias_root(variable)?;
        let solution = self.settled_root(solution)?;

        if self.root_variable(solution)?.is_some() {
            return Err(CompilerError::Internal {
                message: format!(
                    "check variable {variable:?} received open solution {} ({solution:?})",
                    self.format_type(solution),
                ),
            });
        }

        // complete circular solutions with the error type
        if self.type_contains_variable(solution, variable)? {
            let origin_id = self.solver.variable(variable)?.origin;
            let origin = self.solver.origin(origin_id);
            let error = self.circular_type_error(origin)?;
            self.report(origin.module(), error);
            let error = self.intern_type(dir::Type::Error)?;

            return self.commit_error_solution(variable, error);
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
        self.commit_variable_solution(variable, VariableState::Resolved(solution), bounds)
    }

    /// Alias one open variable onto an equal variable's component root.
    pub(in crate::check) fn alias_variable(
        &mut self,
        first: dir::TypeVariableId,
        second: dir::TypeVariableId,
    ) -> CompilerResult<()> {
        // forward the younger root, keeping the older as the component root
        let first = self.solver.alias_root(first)?;
        let second = self.solver.alias_root(second)?;
        if first == second {
            return Ok(());
        }
        let (root, aliased) = if first.0 < second.0 {
            (first, second)
        } else {
            (second, first)
        };

        // collect the forwarded bounds and default before rewriting the state
        let lower = self
            .solver
            .variables
            .side_bounds(aliased, BoundSide::Lower)?
            .collect::<SmallVec<[TypeBound; 2]>>();
        let upper = self
            .solver
            .variables
            .side_bounds(aliased, BoundSide::Upper)?
            .collect::<SmallVec<[TypeBound; 2]>>();
        let default = self.solver.variables.variable_default(aliased);

        // forward the aliased variable onto the root
        self.solver.variable_mut(aliased)?.state = VariableState::Alias(root);

        // wake tasks parked on the forwarded variable
        let waiters = self.solver.wake(Dependency::Variable(aliased));
        for waiter in waiters.iter().cloned() {
            self.queue_task(waiter);
        }

        // migrate collected bounds and the declared default onto the root
        for bound in lower {
            self.push_variable_bound(
                root,
                BoundSide::Lower,
                bound.origin,
                bound.cause,
                bound.ty,
                bound.relation,
            )?;
        }
        for bound in upper {
            self.push_variable_bound(
                root,
                BoundSide::Upper,
                bound.origin,
                bound.cause,
                bound.ty,
                bound.relation,
            )?;
        }
        if let Some(default) = default
            && self.solver.variables.variable_default(root).is_none()
        {
            self.solver.set_variable_default(root, default);
        }

        Ok(())
    }

    /// Record one failed variable solution and wake its waiters.
    pub(in crate::check) fn commit_error_solution(
        &mut self,
        variable: dir::TypeVariableId,
        error: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
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

        self.commit_variable_solution(variable, VariableState::Error(error), bounds)
    }

    /// Record one completed variable state and wake its waiters.
    fn commit_variable_solution(
        &mut self,
        variable: dir::TypeVariableId,
        state: VariableState,
        bounds: VariableBounds,
    ) -> CompilerResult<()> {
        let ty = state.ty().ok_or_else(|| CompilerError::Internal {
            message: format!("cannot commit open check variable {variable:?}"),
        })?;

        // require exactly one state transition per variable
        let previous = self.solver.variable(variable)?.state;
        if previous == state {
            return Ok(());
        }
        if !previous.is_open() {
            return Err(CompilerError::Internal {
                message: format!("check variable {variable:?} completed twice"),
            });
        }
        self.solver.variable_mut(variable)?.state = state;

        // wake tasks parked on the solved variable
        let waiters = self.solver.wake(Dependency::Variable(variable));
        for waiter in waiters.iter().cloned() {
            self.queue_task(waiter);
        }

        self.record_event(CheckEvent::VariableSolved {
            variable,
            bounds: bounds.clone(),
            solution: ty,
            waiters: waiters.len(),
        });

        // discharge accumulated bounds against the completed type
        for bound in &bounds.lower {
            self.discharge_bound(BoundSide::Lower, bound, ty)?;
        }
        for bound in &bounds.upper {
            self.discharge_bound(BoundSide::Upper, bound, ty)?;
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
        match self.constrain_type(bound.origin, bound.cause, bound.relation, source, target)? {
            Answer::Ready(true) => {}
            Answer::Ready(false) | Answer::Pending(_) => {
                self.push_constraint(Constraint::r#type(
                    bound.origin,
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
        origin: Origin,
        cause: CauseId,
        bound: dir::GlobalTypeId,
        relation: Relation,
    ) -> CompilerResult<()> {
        self.push_variable_bound(variable, BoundSide::Lower, origin, cause, bound, relation)
    }

    /// Push one upper bound onto a variable and schedule it.
    pub(in crate::check) fn push_upper_bound(
        &mut self,
        variable: dir::TypeVariableId,
        origin: Origin,
        cause: CauseId,
        bound: dir::GlobalTypeId,
        relation: Relation,
    ) -> CompilerResult<()> {
        self.push_variable_bound(variable, BoundSide::Upper, origin, cause, bound, relation)
    }

    /// Push one bound onto a variable side and schedule it.
    fn push_variable_bound(
        &mut self,
        variable: dir::TypeVariableId,
        side: BoundSide,
        origin: Origin,
        cause: CauseId,
        bound: dir::GlobalTypeId,
        relation: Relation,
    ) -> CompilerResult<()> {
        // late bounds against a solved variable discharge as relation checks
        if let Some(solution) = self.solver.variable(variable)?.state.ty() {
            let late = TypeBound::new(origin, bound, relation, cause);
            self.discharge_bound(side, &late, solution)?;

            return Ok(());
        }

        // collect each distinct bound once
        let bound = TypeBound::new(origin, bound, relation, cause);
        if !self.solver.push_bound(variable, side, bound)? {
            return Ok(());
        }
        self.record_event(match side {
            BoundSide::Lower => CheckEvent::LowerBoundPushed { variable, bound },
            BoundSide::Upper => CheckEvent::UpperBoundPushed { variable, bound },
        });

        // propagate the new bound through every bound on the opposite side
        let opposite = match side {
            BoundSide::Lower => BoundSide::Upper,
            BoundSide::Upper => BoundSide::Lower,
        };
        let opposite = self
            .solver
            .variables
            .side_bounds(variable, opposite)?
            .collect::<SmallVec<[_; 2]>>();
        for paired in opposite {
            let (lower, upper) = match side {
                BoundSide::Lower => (bound, paired),
                BoundSide::Upper => (paired, bound),
            };
            let Some(relation) = lower.relation.transitive_with(upper.relation) else {
                continue;
            };
            self.push_constraint(Constraint::r#type(
                upper.origin,
                relation,
                lower.ty,
                upper.ty,
                upper.cause,
            ));
        }

        Ok(())
    }
}
