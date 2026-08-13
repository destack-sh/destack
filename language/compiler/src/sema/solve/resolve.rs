use destack_core::FxIndexSet;
use destack_dir as dir;
use smallvec::SmallVec;

use crate::sema::{
    BoundSide, Cause, CauseId, CauseKind, CheckEvent, CheckState, Constraint, DeferredCheck,
    InferenceScope, Origin, PendingWork, Relation, TypeBound, VariableBounds, VariableRole,
    VariableState, Widening, WorkState,
};
use crate::{CompilerError, CompilerResult};

/// How one bound depends on open variables during component resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BoundDependency {
    /// The bound is fully closed.
    Closed,
    /// An alias of another component member.
    Alias,
    /// A construction over component members.
    Recursive,
    /// A reference outside the component.
    External,
}

/// The bounds collected across one variable component.
struct ComponentBounds {
    /// The closed lower bounds.
    closed_lower: SmallVec<[TypeBound; 4]>,
    /// The closed upper bounds.
    closed_upper: SmallVec<[TypeBound; 4]>,
    /// The lower bound types with the admitted literal widening applied.
    lower_types: SmallVec<[dir::GlobalTypeId; 4]>,
    /// The closed contextual expectations.
    contextual_types: SmallVec<[dir::GlobalTypeId; 4]>,
    /// Whether a bound reaches variables outside the component.
    has_external_bound: bool,
    /// Whether a bound constructs a type over component members.
    has_recursive_bound: bool,
    /// The closed upper equation pinning the solution.
    equation: Option<dir::GlobalTypeId>,
    /// Whether an open upper equation is outstanding.
    has_open_equation: bool,
    /// Whether an open contextual expectation is outstanding.
    has_open_context: bool,
}

/// One fallback stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) enum FallbackStage {
    /// Complete components carrying declared defaults.
    Declared,
    /// Complete components resolvable from their bounds alone.
    Bounded,
    /// Complete the remaining components, widening literals.
    Widened,
    /// Complete every component from its bounds as they stand.
    Final,
}

impl CheckState<'_> {
    /// Allocate one inference variable.
    pub(in crate::sema) fn allocate_variable(
        &mut self,
        origin: Origin,
        widening: Widening,
        role: VariableRole,
    ) -> dir::TypeVariableId {
        let variable = self.infer.allocate_variable(origin, widening, role);
        self.record_event(CheckEvent::VariableAllocated { variable, widening });

        variable
    }

    /// Record the declared default completing one variable when inference stays dry.
    pub(in crate::sema) fn set_variable_default(
        &mut self,
        variable: dir::TypeVariableId,
        default: dir::GlobalTypeId,
    ) {
        self.infer.set_variable_default(variable, default);
    }

    /// Return one canonical open variable, or none when the variable is solved.
    pub(in crate::sema) fn open_variable(
        &self,
        variable: dir::TypeVariableId,
    ) -> CompilerResult<Option<dir::TypeVariableId>> {
        let variable = self.infer.alias_root(variable)?;
        let state = self.infer.variable(variable)?;

        Ok(state.state.is_open().then_some(variable))
    }

    /// Return the special role attached to one open variable.
    pub(in crate::sema) fn variable_role(
        &self,
        variable: dir::TypeVariableId,
    ) -> CompilerResult<VariableRole> {
        let Some(variable) = self.open_variable(variable)? else {
            return Ok(VariableRole::Regular);
        };

        self.infer.variable_role(variable)
    }

    /// Return the memory parameter kind one variable ranges over, however it arose.
    pub(in crate::sema) fn variable_memory_parameter(
        &self,
        variable: dir::TypeVariableId,
    ) -> CompilerResult<Option<dir::MemoryParameter>> {
        let kind = match self.infer.variable_role(variable)? {
            VariableRole::Memory { kind, .. } => Some(kind),
            VariableRole::Instantiation { parameter } => self
                .generic_parameter(parameter)
                .and_then(|binding| binding.memory_parameter()),
            _ => None,
        };

        Ok(kind)
    }

    /// Return whether one open variable already carries a closed contextual expectation.
    pub(in crate::sema) fn has_contextual_expectation(
        &mut self,
        variable: dir::TypeVariableId,
    ) -> CompilerResult<bool> {
        let variable = self.infer.alias_root(variable)?;
        let bounds = self
            .infer
            .variables
            .side_bounds(variable, BoundSide::Upper)?
            .collect::<SmallVec<[TypeBound; 2]>>();

        // accept the first directed expectation whose type has already closed
        for bound in bounds {
            if matches!(bound.relation, Relation::Assignable | Relation::Widens)
                && self.type_variables(bound.ty)?.is_empty()
            {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Resolve the given variables in place through every fallback stage.
    pub(in crate::sema) fn resolve_variables(
        &mut self,
        variables: &[dir::TypeVariableId],
    ) -> CompilerResult<()> {
        // read the current roots once; applying defaults never re-aliases them
        let mut roots = SmallVec::<[dir::TypeVariableId; 4]>::new();
        for variable in variables {
            let root = self.infer.alias_root(*variable)?;
            if !roots.contains(&root) {
                roots.push(root);
            }
        }

        // alternate bound resolution and defaults until nothing applies
        loop {
            self.resolve_bounded_components(&roots)?;
            if !self.default_components(&roots)? {
                break;
            }
        }

        Ok(())
    }

    /// Apply one default to one of a scope's open variables.
    pub(in crate::sema) fn default_scope(
        &mut self,
        scope: InferenceScope,
    ) -> CompilerResult<bool> {
        let roots = self.open_scope_variables(scope)?;

        self.default_components(&roots)
    }

    /// Apply one default across the given roots, staged.
    pub(in crate::sema) fn default_components(
        &mut self,
        roots: &[dir::TypeVariableId],
    ) -> CompilerResult<bool> {
        for stage in [
            FallbackStage::Declared,
            FallbackStage::Bounded,
            FallbackStage::Widened,
        ] {
            for root in roots {
                // complete roots carrying declared defaults in their own stage
                let variables = [*root];
                let has_defaults = !self.component_defaults(&variables)?.is_empty();
                if stage == FallbackStage::Declared && !has_defaults {
                    continue;
                }
                if stage == FallbackStage::Bounded && has_defaults {
                    continue;
                }

                // return on the first applied default, so bounds re-resolve between defaults
                if self.infer.variable(*root)?.state.is_open()
                    && self.default_component(&variables, stage)?
                {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Resolve every open component one scope owns or bounds.
    pub(in crate::sema) fn resolve_scope(
        &mut self,
        scope: InferenceScope,
        stage: FallbackStage,
    ) -> CompilerResult<bool> {
        // resolve each owned open root
        let roots = self.open_scope_variables(scope)?;
        let mut resolved = false;
        for root in &roots {
            resolved |= self.resolve_component(&[*root], &[], stage)?;
        }

        // resolve preexisting variables bounded by this scope
        let adopted = self.infer.adopted_variables(scope)?;
        for variable in &adopted {
            if self.infer.variable(*variable)?.state.is_open() {
                resolved |= self.resolve_component(&[*variable], &[], stage)?;
            }
        }

        Ok(resolved)
    }

    /// Return the open variables owned by one inference scope.
    pub(in crate::sema) fn open_scope_variables(
        &self,
        scope: InferenceScope,
    ) -> CompilerResult<SmallVec<[dir::TypeVariableId; 2]>> {
        let mut open = SmallVec::new();
        for index in scope.indices(self.infer.variable_count()) {
            let variable = dir::TypeVariableId(index as u32);
            if self.infer.variable(variable)?.state.is_open() {
                open.push(variable);
            }
        }

        Ok(open)
    }

    /// Resolve bounded components to a fixpoint, without defaults.
    pub(in crate::sema) fn resolve_bounded_components(
        &mut self,
        variables: &[dir::TypeVariableId],
    ) -> CompilerResult<bool> {
        // iterate to a fixpoint: each solution can unblock another component
        let mut resolved = false;
        loop {
            let mut progress = false;
            for variable in variables {
                let root = self.infer.alias_root(*variable)?;
                if self.infer.variable(root)?.state.is_open() {
                    progress |= self.resolve_component(&[root], &[], FallbackStage::Bounded)?;
                }
            }

            resolved |= progress;
            if !progress {
                break;
            }
        }

        Ok(resolved)
    }

    /// Complete one variable component from its declared defaults.
    pub(in crate::sema) fn default_component(
        &mut self,
        variables: &[dir::TypeVariableId],
        stage: FallbackStage,
    ) -> CompilerResult<bool> {
        let defaults = self.component_defaults(variables)?;

        self.resolve_component(variables, &defaults, stage)
    }

    /// Collect the declared and memory defaults of one variable component.
    pub(in crate::sema) fn component_defaults(
        &mut self,
        variables: &[dir::TypeVariableId],
    ) -> CompilerResult<SmallVec<[dir::GlobalTypeId; 2]>> {
        // read each variable's declared default, or its canonical memory default
        let mut defaults = SmallVec::<[dir::GlobalTypeId; 2]>::new();
        for variable in variables {
            let default = match self.infer.variables.variable_default(*variable) {
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

            // keep a default whose every variable already solved outside the component
            if let Some(default) = default {
                let mut open = false;
                for variable in self.type_variables(default)? {
                    if variables.contains(&variable) || self.infer.solution(variable)?.is_none() {
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

    /// Return whether one pending selection still derives a component member.
    fn has_pending_producer(&self, variables: &[dir::TypeVariableId]) -> CompilerResult<bool> {
        for work in &self.fulfill.work {
            if work.state == WorkState::Done {
                continue;
            }

            let PendingWork::Check(DeferredCheck::Infer { site, .. }) = &work.kind else {
                continue;
            };

            // derive a member when the selection shares the variable's node
            for variable in variables {
                let origin = self.infer.origin(self.infer.variable(*variable)?.origin);
                if let Origin::Node(node, _) = origin
                    && node == site.node
                {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Resolve one variable component from its bounds and declared defaults.
    pub(in crate::sema) fn resolve_component(
        &mut self,
        variables: &[dir::TypeVariableId],
        defaults: &[dir::GlobalTypeId],
        stage: FallbackStage,
    ) -> CompilerResult<bool> {
        if stage != FallbackStage::Final && self.has_pending_producer(variables)? {
            return Ok(false);
        }

        // classify the component's bounds, deferring while widening waits
        let Some(bounds) = self.classify_component_bounds(variables, stage)? else {
            return Ok(false);
        };

        // choose the solution the bounds and declared defaults admit
        let Some(solution) = self.choose_component_solution(variables, defaults, &bounds, stage)?
        else {
            return Ok(false);
        };

        // commit the same type to every variable in the cycle
        for variable in variables {
            self.commit_solution(*variable, solution)?;
        }

        Ok(true)
    }

    /// Classify every bound of one variable component against the component.
    fn classify_component_bounds(
        &mut self,
        variables: &[dir::TypeVariableId],
        stage: FallbackStage,
    ) -> CompilerResult<Option<ComponentBounds>> {
        // collect proper bounds while retaining structural cycles as errors
        let mut closed_lower = SmallVec::<[TypeBound; 4]>::new();
        let mut closed_upper = SmallVec::<[TypeBound; 4]>::new();
        let mut lower_types = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        let mut contextual_types = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        let mut has_external_bound = false;
        let mut has_recursive_bound = false;
        let mut equation = None;
        let mut has_open_equation = false;
        let mut has_open_context = false;
        for variable in variables {
            let lower = self
                .infer
                .variables
                .side_bounds(*variable, BoundSide::Lower)?
                .collect::<SmallVec<[TypeBound; 2]>>();
            let upper = self
                .infer
                .variables
                .side_bounds(*variable, BoundSide::Upper)?
                .collect::<SmallVec<[TypeBound; 2]>>();
            let mut variable_lower = SmallVec::<[TypeBound; 2]>::new();

            // classify each lower bound against this component
            for bound in &lower {
                match self.bound_dependency(bound.ty, variables)? {
                    BoundDependency::Closed => {
                        closed_lower.push(*bound);
                        variable_lower.push(*bound);
                    }
                    BoundDependency::Alias => {}
                    BoundDependency::Recursive => {
                        has_recursive_bound = true;
                    }
                    BoundDependency::External => {
                        has_external_bound = true;
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
            let state = *self.infer.variable(*variable)?;
            let origin = self.infer.origin(state.origin);
            let widens = !has_equation
                && match state.widening {
                    Widening::Never => false,
                    Widening::Aggregate => false,
                    Widening::Multiple => self.has_distinct_types(origin, &candidates)?,
                    Widening::Always => true,
                    Widening::Const => self.has_only_literal_types(&candidates)?,
                };

            // defer literal widening to the final fallback stage
            if widens && stage != FallbackStage::Widened {
                for bound in &variable_lower {
                    if matches!(bound.relation, Relation::Assignable | Relation::Castable)
                        && self.widen_bound_type(state.widening, bound.ty)? != bound.ty
                    {
                        return Ok(None);
                    }
                }
            }

            // widen the literal lower bounds this variable's policy admits
            for bound in variable_lower {
                if has_equation && bound.relation != Relation::Equal {
                    continue;
                }
                let ty = if widens
                    && matches!(bound.relation, Relation::Assignable | Relation::Castable)
                {
                    self.widen_bound_type(state.widening, bound.ty)?
                } else {
                    bound.ty
                };
                lower_types.push(ty);
            }

            // classify each upper bound, keeping its contextual expectation
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
                    BoundDependency::Alias => {}
                    BoundDependency::Recursive => {
                        has_recursive_bound = true;
                        has_open_equation |= bound.relation == Relation::Equal;
                    }
                    BoundDependency::External => {
                        has_external_bound = true;
                        has_open_equation |= bound.relation == Relation::Equal;

                        // adopt an open contextual expectation once it closes
                        has_open_context |=
                            matches!(bound.relation, Relation::Widens | Relation::Assignable);
                    }
                }
            }
        }

        Ok(Some(ComponentBounds {
            closed_lower,
            closed_upper,
            lower_types,
            contextual_types,
            has_external_bound,
            has_recursive_bound,
            equation,
            has_open_equation,
            has_open_context,
        }))
    }

    /// Choose one variable component's solution, or none while it stays blocked.
    fn choose_component_solution(
        &mut self,
        variables: &[dir::TypeVariableId],
        defaults: &[dir::GlobalTypeId],
        bounds: &ComponentBounds,
        stage: FallbackStage,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // read the component's root state
        let root = variables[0];
        let state = *self.infer.variable(root)?;
        let origin = self.infer.origin(state.origin);
        let is_unconstrained_recursion = bounds.has_recursive_bound
            && bounds.lower_types.is_empty()
            && bounds.contextual_types.is_empty()
            && !bounds.has_external_bound;

        // form the preferred lower and contextual solutions once
        let lower = match bounds.lower_types.is_empty() {
            true => None,
            false => Some(self.best_common(root, &bounds.lower_types)?),
        };
        let contextual = match bounds.contextual_types.as_slice() {
            [] => None,
            [single] => Some(*single),
            _ => Some(self.intersect_bounds(root, &bounds.contextual_types)?),
        };

        // choose the component solution from lower bounds, then upper context, then defaults
        let solution = if let Some(equation) = bounds.equation {
            Some(equation)
        } else if bounds.has_open_equation || is_unconstrained_recursion {
            None
        } else if bounds.has_open_context && stage != FallbackStage::Final {
            // wait for the contextual expectation to close before choosing
            None
        } else if bounds.has_external_bound && lower.is_none() && stage != FallbackStage::Final {
            // wait for open lower bounds to close before adopting the context
            None
        } else if let Some(lower) = lower {
            if self.solution_satisfies_bounds(
                origin,
                lower,
                &bounds.closed_lower,
                &bounds.closed_upper,
            )? {
                Some(lower)
            } else if let Some(contextual) = contextual
                && self.solution_satisfies_bounds(
                    origin,
                    contextual,
                    &bounds.closed_lower,
                    &bounds.closed_upper,
                )?
            {
                Some(contextual)
            } else {
                // report every violated bound, then poison the component
                for bound in &bounds.closed_lower {
                    self.discharge_bound(BoundSide::Lower, bound, lower)?;
                }
                for bound in &bounds.closed_upper {
                    self.discharge_bound(BoundSide::Upper, bound, lower)?;
                }

                Some(self.intern_type(dir::Type::Error)?)
            }
        } else if let Some(contextual) = contextual {
            Some(contextual)
        } else if bounds.has_external_bound {
            None
        } else if let [default] = defaults {
            Some(*default)
        } else if !defaults.is_empty() {
            Some(self.best_common(root, defaults)?)
        } else {
            None
        };

        // report a candidate-free structural cycle as an infinite type
        let solution = match solution {
            Some(solution) => solution,
            None if is_unconstrained_recursion && variables.len() == 1 => {
                let error = self.circular_type_error(origin)?;
                self.report(origin.module(), error);

                self.intern_type(dir::Type::Error)?
            }
            // retry an externally blocked component once its variables solve
            None => return Ok(None),
        };

        // expose the chosen solution's named head
        let solution = self.shallow_resolve(solution)?;
        let solution = self.reduce_redundant_forms(origin, solution)?;

        // commit canonical memory literals for memory variables
        let solution = match self.variable_memory_parameter(root)? {
            Some(kind) => self.normalize_memory_component(origin, solution, kind)?,
            None => solution,
        };

        Ok(Some(solution))
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

        // report bounds that reach variables outside this component
        if dependencies
            .iter()
            .any(|dependency| !variables.contains(dependency))
        {
            return Ok(BoundDependency::External);
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
            match self.evaluate_relation(origin, bound.relation, bound.ty, solution)? {
                true => {}
                false => return Ok(false),
            }
        }

        // require the solution to flow into every expectation
        for bound in upper {
            match self.evaluate_relation(origin, bound.relation, solution, bound.ty)? {
                true => {}
                false => return Ok(false),
            }
        }

        Ok(true)
    }

    /// Widen one literal lower bound under a variable's widening policy.
    fn widen_bound_type(
        &mut self,
        widening: Widening,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        match widening {
            Widening::Const => self.widen_const_type(ty),
            Widening::Never | Widening::Aggregate | Widening::Multiple | Widening::Always => {
                self.widen_type(ty)
            }
        }
    }

    /// Return whether every candidate is an exact literal type.
    fn has_only_literal_types(&mut self, types: &[dir::GlobalTypeId]) -> CompilerResult<bool> {
        for ty in types {
            let ty = self.shallow_resolve(*ty)?;
            if !matches!(self.ty(ty)?, dir::Type::Literal(_)) {
                return Ok(false);
            }
        }

        Ok(!types.is_empty())
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

        let first = self.shallow_resolve(*first)?;

        // compare each remaining candidate with the first
        for ty in &types[1..] {
            let ty = self.shallow_resolve(*ty)?;
            let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
            match self.relate_equal(origin, cause, first, ty)? {
                true => {}
                false => return Ok(true),
            }
        }

        Ok(false)
    }

    /// Collect the unsolved variables one type transitively references.
    pub(in crate::sema) fn type_variables(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<SmallVec<[dir::TypeVariableId; 2]>> {
        // skip the scan when the interned flags name no variable
        if !self.type_flags(id)?.has_variable() {
            return Ok(SmallVec::new());
        }

        // scan the type graph without following symbol references
        let mut variables = SmallVec::new();
        let mut pending = SmallVec::<[dir::GlobalTypeId; 8]>::new();
        let mut visited = FxIndexSet::default();
        pending.push(id);
        while let Some(id) = pending.pop() {
            if !visited.insert(id) {
                continue;
            }

            let ty = self.ty(id)?;

            // follow solved variables and collect open variables once
            if let dir::Type::Variable(variable) = ty {
                if let Some(solution) = self.infer.solution(variable)? {
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

    /// Record one variable solution.
    pub(in crate::sema) fn commit_solution(
        &mut self,
        variable: dir::TypeVariableId,
        solution: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        // require a committed solution to be closed
        let variable = self.infer.alias_root(variable)?;
        let solution = self.shallow_resolve(solution)?;
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
            let origin_id = self.infer.variable(variable)?.origin;
            let origin = self.infer.origin(origin_id);
            let error = self.circular_type_error(origin)?;
            self.report(origin.module(), error);
            let error = self.intern_type(dir::Type::Error)?;

            return self.commit_error_solution(variable, error);
        }

        // carry the collected bounds into the completed state
        let bounds = VariableBounds {
            lower: self
                .infer
                .variables
                .side_bounds(variable, BoundSide::Lower)?
                .collect(),
            upper: self
                .infer
                .variables
                .side_bounds(variable, BoundSide::Upper)?
                .collect(),
        };
        self.commit_variable_solution(variable, VariableState::Resolved(solution), bounds)
    }

    /// Alias one open variable onto an equal variable's component root.
    pub(in crate::sema) fn alias_variable(
        &mut self,
        first: dir::TypeVariableId,
        second: dir::TypeVariableId,
    ) -> CompilerResult<()> {
        // forward the younger root, keeping the older as the component root
        let first = self.infer.alias_root(first)?;
        let second = self.infer.alias_root(second)?;
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
            .infer
            .variables
            .side_bounds(aliased, BoundSide::Lower)?
            .collect::<SmallVec<[TypeBound; 2]>>();
        let upper = self
            .infer
            .variables
            .side_bounds(aliased, BoundSide::Upper)?
            .collect::<SmallVec<[TypeBound; 2]>>();
        let default = self.infer.variables.variable_default(aliased);

        // forward the aliased variable onto the root
        self.infer.variable_mut(aliased)?.state = VariableState::Alias(root);
        self.fulfill.wake_variable(aliased);

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
            && self.infer.variables.variable_default(root).is_none()
        {
            self.infer.set_variable_default(root, default);
        }

        Ok(())
    }

    /// Record one failed variable solution.
    pub(in crate::sema) fn commit_error_solution(
        &mut self,
        variable: dir::TypeVariableId,
        error: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        // carry the collected bounds into the failed state
        let bounds = VariableBounds {
            lower: self
                .infer
                .variables
                .side_bounds(variable, BoundSide::Lower)?
                .collect(),
            upper: self
                .infer
                .variables
                .side_bounds(variable, BoundSide::Upper)?
                .collect(),
        };

        self.commit_variable_solution(variable, VariableState::Error(error), bounds)
    }

    /// Record one completed variable state.
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
        let previous = self.infer.variable(variable)?.state;
        if previous == state {
            return Ok(());
        }

        if !previous.is_open() {
            return Err(CompilerError::Internal {
                message: format!("check variable {variable:?} completed twice"),
            });
        }

        // complete the variable and wake the work watching it
        self.infer.variable_mut(variable)?.state = state;
        self.fulfill.wake_variable(variable);
        self.record_event(CheckEvent::VariableSolved {
            variable,
            bounds: Box::new(bounds.clone()),
            solution: ty,
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

        // relations that cannot resolve in place become constraints
        let resolved =
            self.constrain_type(bound.origin, bound.cause, bound.relation, source, target)?;
        if !resolved {
            self.push_constraint(Constraint::r#type(
                bound.origin,
                bound.relation,
                source,
                target,
                bound.cause,
            ))?;
        }

        Ok(())
    }

    /// Push one lower bound onto a variable and schedule it.
    pub(in crate::sema) fn push_lower_bound(
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
    pub(in crate::sema) fn push_upper_bound(
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
        // discharge a late bound as a relation check
        if let Some(solution) = self.infer.variable(variable)?.state.ty() {
            let late = TypeBound::new(origin, bound, relation, cause);
            self.discharge_bound(side, &late, solution)?;

            return Ok(());
        }

        // collect each distinct bound once
        let bound = TypeBound::new(origin, bound, relation, cause);
        if !self.infer.push_bound(variable, side, bound)? {
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
            .infer
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
            ))?;
        }

        Ok(())
    }
}
