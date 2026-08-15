use destack_core::FxIndexSet;
use destack_dir as dir;
use smallvec::SmallVec;

use crate::sema::{
    BoundSide, Cause, CauseId, CauseKind, CheckEvent, CheckState, InferenceScope, Origin, Relation,
    RelationCheck, TypeBound, VariableBounds, VariableRole, VariableState, Verdict, Widening,
    WorkState,
};
use crate::{CompilerError, CompilerResult};

/// One fallback stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) enum FallbackStage {
    /// Complete variables resolvable from their bounds alone.
    Bounded,
    /// Complete the remaining variables, widening literals.
    Widened,
    /// Complete every variable from its bounds as they stand.
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
        // collect the distinct component roots once
        let mut roots = SmallVec::<[dir::TypeVariableId; 4]>::new();
        for variable in variables {
            let root = self.infer.alias_root(*variable)?;
            if !roots.contains(&root) {
                roots.push(root);
            }
        }

        // alternate bound resolution and defaults until nothing applies
        loop {
            // resolve from bounds to a fixpoint, unblocking each dependent solution
            while self.resolve_scope_roots(&roots, FallbackStage::Bounded)? {}

            if !self.default_variables(&roots)? {
                break;
            }
        }

        Ok(())
    }

    /// Apply one default to one of a scope's open variables.
    pub(in crate::sema) fn default_scope(&mut self, scope: InferenceScope) -> CompilerResult<bool> {
        let roots = self.open_scope_variables(scope)?;

        self.default_variables(&roots)
    }

    /// Apply one fallback across the given roots, defaults before widening.
    pub(in crate::sema) fn default_variables(
        &mut self,
        roots: &[dir::TypeVariableId],
    ) -> CompilerResult<bool> {
        // apply one declared default at a time, so bounds re-resolve between them
        for root in roots {
            if self.infer.variable(*root)?.state.is_open()
                && let Some(default) = self.variable_default(*root)?
                && self.resolve_variable(*root, &[default], FallbackStage::Bounded)?
            {
                return Ok(true);
            }
        }

        // widen one literal-bounded variable once the defaults run dry
        for root in roots {
            if self.infer.variable(*root)?.state.is_open() {
                let defaults = match self.variable_default(*root)? {
                    Some(default) => SmallVec::<[dir::GlobalTypeId; 1]>::from_slice(&[default]),
                    None => SmallVec::new(),
                };
                if self.resolve_variable(*root, &defaults, FallbackStage::Widened)? {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Resolve every variable this scope allocated, in allocation order.
    pub(in crate::sema) fn resolve_scope(
        &mut self,
        scope: InferenceScope,
        stage: FallbackStage,
    ) -> CompilerResult<bool> {
        let roots = self.open_scope_variables(scope)?;

        self.resolve_scope_roots(&roots, stage)
    }

    /// Resolve every still-open root once from its bounds, in order.
    fn resolve_scope_roots(
        &mut self,
        roots: &[dir::TypeVariableId],
        stage: FallbackStage,
    ) -> CompilerResult<bool> {
        // resolve each root that is still open
        let mut resolved = false;
        for root in roots {
            let root = self.infer.alias_root(*root)?;
            if self.infer.variable(root)?.state.is_open() {
                resolved |= self.resolve_variable(root, &[], stage)?;
            }
        }

        Ok(resolved)
    }

    /// Return the open variables owned by one inference scope.
    pub(in crate::sema) fn open_scope_variables(
        &self,
        scope: InferenceScope,
    ) -> CompilerResult<SmallVec<[dir::TypeVariableId; 2]>> {
        // collect the scope's still-open variables in allocation order
        let mut open = SmallVec::new();
        for index in scope.indices(self.infer.variable_count()) {
            let variable = dir::TypeVariableId(index as u32);
            if self.infer.variable(variable)?.state.is_open() {
                open.push(variable);
            }
        }

        Ok(open)
    }

    /// Return one variable's declared default, or its canonical memory default.
    pub(in crate::sema) fn variable_default(
        &mut self,
        variable: dir::TypeVariableId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // read the declared default, seeding the canonical memory default once
        let default = match self.infer.variables.variable_default(variable) {
            Some(default) => Some(default),
            None => {
                let memory = match self.variable_memory_parameter(variable)? {
                    Some(dir::MemoryParameter::Lifetime) => {
                        Some(dir::MemoryLiteral::Lifetime(dir::Lifetime::Frame))
                    }
                    Some(dir::MemoryParameter::Access) => {
                        Some(dir::MemoryLiteral::Access(dir::Access::Readonly))
                    }
                    _ => None,
                };
                match memory {
                    Some(memory) => {
                        let default = self.intern_type(dir::Type::Memory(memory))?;
                        self.infer.set_variable_default(variable, default);

                        Some(default)
                    }
                    None => None,
                }
            }
        };

        // require every variable the default references to be solved already
        let Some(default) = default else {
            return Ok(None);
        };
        for dependency in self.type_variables(default)? {
            if dependency == variable || self.infer.solution(dependency)?.is_none() {
                return Ok(None);
            }
        }

        Ok(Some(default))
    }

    /// Resolve one open variable from its bounds and declared defaults.
    pub(in crate::sema) fn resolve_variable(
        &mut self,
        variable: dir::TypeVariableId,
        defaults: &[dir::GlobalTypeId],
        stage: FallbackStage,
    ) -> CompilerResult<bool> {
        // wait for a live producer that may still grow this variable's bounds
        if stage != FallbackStage::Final && self.variable_may_grow(variable)? {
            return Ok(false);
        }

        // split the lower bounds into usable bounds and recursive constructions
        let lower = self
            .infer
            .variables
            .side_bounds(variable, BoundSide::Lower)?
            .collect::<SmallVec<[TypeBound; 2]>>();
        let mut closed_lower = SmallVec::<[TypeBound; 4]>::new();
        let mut has_recursive_bound = false;
        for bound in &lower {
            if self.type_contains_variable(bound.ty, variable)? {
                has_recursive_bound = true;
                continue;
            }
            closed_lower.push(*bound);
        }

        // split the upper bounds, keeping the equation and contextual expectations
        let upper = self
            .infer
            .variables
            .side_bounds(variable, BoundSide::Upper)?
            .collect::<SmallVec<[TypeBound; 2]>>();
        let mut closed_upper = SmallVec::<[TypeBound; 4]>::new();
        let mut contextual_types = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        let mut equation = None;
        for bound in &upper {
            if self.type_contains_variable(bound.ty, variable)? {
                has_recursive_bound = true;
                continue;
            }
            closed_upper.push(*bound);
            if bound.relation == Relation::Equal {
                equation = Some(bound.ty);
            }
            if matches!(bound.relation, Relation::Assignable | Relation::Widens) {
                contextual_types.push(bound.ty);
            }
        }

        // apply this variable's literal policy to its closed lower bounds
        let has_equation = closed_lower
            .iter()
            .any(|bound| bound.relation == Relation::Equal);
        let candidates = closed_lower
            .iter()
            .map(|bound| bound.ty)
            .collect::<SmallVec<[_; 2]>>();
        let state = *self.infer.variable(variable)?;
        let origin = self.infer.origin(state.origin);
        let widens = !has_equation
            && match state.widening {
                Widening::Never => false,
                Widening::Aggregate => false,
                Widening::Multiple => self.has_distinct_types(origin, &candidates)?,
                Widening::Always => true,
                Widening::Const => self.has_only_literal_types(&candidates)?,
            };

        // defer literal widening to its own fallback stage
        if widens && stage != FallbackStage::Widened {
            for bound in &closed_lower {
                if matches!(bound.relation, Relation::Assignable | Relation::Castable)
                    && self.widen_bound_type(state.widening, bound.ty)? != bound.ty
                {
                    return Ok(false);
                }
            }
        }

        // admit the lower candidate types, widened as the policy allows
        let mut lower_types = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        for bound in &closed_lower {
            if has_equation && bound.relation != Relation::Equal {
                continue;
            }
            let ty =
                if widens && matches!(bound.relation, Relation::Assignable | Relation::Castable) {
                    self.widen_bound_type(state.widening, bound.ty)?
                } else {
                    bound.ty
                };
            lower_types.push(ty);
        }

        // form the preferred lower and contextual solutions once
        let is_unconstrained_recursion =
            has_recursive_bound && lower_types.is_empty() && contextual_types.is_empty();
        let lower_solution = match lower_types.is_empty() {
            true => None,
            false => Some(self.best_common(variable, &lower_types)?),
        };
        let contextual = match contextual_types.as_slice() {
            [] => None,
            [single] => Some(*single),
            _ => Some(self.intersect_bounds(variable, &contextual_types)?),
        };

        // choose the solution: the equation, then lower bounds, then context, then defaults
        let solution = if let Some(equation) = equation {
            Some(equation)
        } else if is_unconstrained_recursion {
            None
        } else if let Some(lower_solution) = lower_solution {
            let verdict = self.solution_satisfies_bounds(
                origin,
                lower_solution,
                &closed_lower,
                &closed_upper,
            )?;

            match verdict {
                // take the lower solution its bounds admit
                Verdict::Holds => Some(lower_solution),
                // adopt a closed candidate at the final settle
                Verdict::Ambiguous
                    if stage == FallbackStage::Final
                        && !self.type_flags(lower_solution)?.has_variable() =>
                {
                    Some(lower_solution)
                }
                // retry once the ambiguity resolves
                Verdict::Ambiguous => return Ok(false),
                // fall back to the contextual solution, then report the violated bounds
                Verdict::Fails => match contextual {
                    Some(contextual)
                        if self.solution_satisfies_bounds(
                            origin,
                            contextual,
                            &closed_lower,
                            &closed_upper,
                        )? == Verdict::Holds =>
                    {
                        Some(contextual)
                    }
                    _ => {
                        // report every violated bound, then poison the variable
                        for bound in &closed_lower {
                            self.discharge_bound(BoundSide::Lower, bound, lower_solution)?;
                        }
                        for bound in &closed_upper {
                            self.discharge_bound(BoundSide::Upper, bound, lower_solution)?;
                        }

                        Some(self.intern_type(dir::Type::Error)?)
                    }
                },
            }
        } else if let Some(contextual) = contextual {
            Some(contextual)
        } else if let [default] = defaults {
            Some(*default)
        } else if !defaults.is_empty() {
            Some(self.best_common(variable, defaults)?)
        } else {
            None
        };

        // report a candidate-free structural cycle as an infinite type
        let solution = match solution {
            Some(solution) => solution,
            None if is_unconstrained_recursion => {
                let error = self.circular_type_error(origin)?;
                self.report(origin.module(), error);

                self.intern_type(dir::Type::Error)?
            }
            // retry a variable blocked on other open variables once they solve
            None => return Ok(false),
        };

        // expose the chosen solution's named head
        let solution = self.shallow_resolve(solution)?;
        let solution = self.reduce_redundant_forms(origin, solution)?;

        // commit canonical memory literals for memory variables
        let solution = match self.variable_memory_parameter(variable)? {
            Some(kind) => self.normalize_memory_component(origin, solution, kind)?,
            None => solution,
        };

        self.commit_solution(variable, solution)?;

        Ok(true)
    }

    /// Return whether a live check may still push a new bound onto one variable.
    fn variable_may_grow(&self, variable: dir::TypeVariableId) -> CompilerResult<bool> {
        let Some(producers) = self.fulfill.producers.get(&variable) else {
            return Ok(false);
        };
        let stalled = self.fulfill.watchers.get(&variable);

        // find one live producer standing outside the variable's own watchers
        for id in producers {
            let is_live = self.fulfill.checks.state(*id) != WorkState::Done;
            let is_stalled_on_self = stalled.is_some_and(|watchers| watchers.contains(id));
            if is_live && !is_stalled_on_self {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Return the verdict of one closed solution against every collected bound.
    fn solution_satisfies_bounds(
        &mut self,
        origin: Origin,
        solution: dir::GlobalTypeId,
        lower: &[TypeBound],
        upper: &[TypeBound],
    ) -> CompilerResult<Verdict> {
        let mut verdict = Verdict::Holds;

        // require every produced value to flow into the solution
        for bound in lower {
            verdict =
                verdict.and(self.evaluate_relation(origin, bound.relation, bound.ty, solution)?);
            if verdict == Verdict::Fails {
                return Ok(Verdict::Fails);
            }
        }

        // require the solution to flow into every expectation
        for bound in upper {
            verdict =
                verdict.and(self.evaluate_relation(origin, bound.relation, solution, bound.ty)?);
            if verdict == Verdict::Fails {
                return Ok(Verdict::Fails);
            }
        }

        Ok(verdict)
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
            if !self.relate_equal(origin, cause, first, ty)?.holds() {
                return Ok(true);
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

        // scan the type graph, stopping at symbol references
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

        self.commit_variable_solution(variable, VariableState::Resolved(solution))
    }

    /// Alias one open variable onto an equal variable's component root.
    pub(in crate::sema) fn alias_variable(
        &mut self,
        first: dir::TypeVariableId,
        second: dir::TypeVariableId,
    ) -> CompilerResult<()> {
        // read both component roots
        let first = self.infer.alias_root(first)?;
        let second = self.infer.alias_root(second)?;
        if first == second {
            return Ok(());
        }

        // keep the older variable as the component root
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
        self.fulfill.forward_producers(aliased, &[root]);

        // migrate the collected lower bounds onto the root
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

        // migrate the collected upper bounds onto the root
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

        // carry the declared default over to a root without one
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
        self.commit_variable_solution(variable, VariableState::Error(error))
    }

    /// Record one completed variable state.
    fn commit_variable_solution(
        &mut self,
        variable: dir::TypeVariableId,
        state: VariableState,
    ) -> CompilerResult<()> {
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
        // require the completed state to carry its own type
        let ty = state.ty().ok_or_else(|| CompilerError::Internal {
            message: format!("cannot commit open check variable {variable:?}"),
        })?;

        // require exactly one state transition per variable
        let previous = self.infer.variable(variable)?.state;
        if previous == state {
            return Ok(());
        }

        // reject a second completion
        if !previous.is_open() {
            return Err(CompilerError::Internal {
                message: format!("check variable {variable:?} completed twice"),
            });
        }

        // complete the variable and wake the work watching it
        self.infer.variable_mut(variable)?.state = state;
        self.fulfill.wake_variable(variable);

        // forward the producers onto the solution's still-open variables
        let successors = self.type_variables(ty)?;
        self.fulfill.forward_producers(variable, &successors);
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
        // orient the bound against the committed solution
        let (source, target) = match side {
            BoundSide::Lower => (bound.ty, solution),
            BoundSide::Upper => (solution, bound.ty),
        };

        // solve the bound as one collected relation, queued while ambiguous
        self.push_relation(RelationCheck::new(
            bound.origin,
            bound.relation,
            source,
            target,
            bound.cause,
        ))?;

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
        // drop a bare self-reference bound, which carries no information
        if self.root_variable(bound)? == Some(self.infer.alias_root(variable)?) {
            return Ok(());
        }

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

        // trace the pushed bound
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

            self.push_relation(RelationCheck::new(
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
