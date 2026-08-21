use destack_core::FxIndexSet;
use destack_dir as dir;
use smallvec::SmallVec;

use crate::sema::{
    BoundSide, CauseId, CheckEvent, CheckState, GenericParameterId, InferenceScope, Origin,
    Relation, RelationCheck, TypeBound, VariableBounds, VariableKind, VariableRole, VariableState,
    Verdict, Wake, WorkState,
};
use crate::{CompilerError, CompilerResult};

/// One fallback stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) enum FallbackStage {
    /// Complete variables resolvable from their bounds alone.
    Bounded,
    /// Complete every variable from its bounds, then from its kind's fallback.
    Final,
}

impl CheckState<'_> {
    /// Allocate one inference variable.
    pub(in crate::sema) fn allocate_variable(
        &mut self,
        origin: Origin,
        role: VariableRole,
    ) -> dir::TypeVariableId {
        self.allocate_variable_of(origin, VariableKind::Type, role)
    }

    /// Allocate one inference variable of the given kind.
    pub(in crate::sema) fn allocate_variable_of(
        &mut self,
        origin: Origin,
        kind: VariableKind,
        role: VariableRole,
    ) -> dir::TypeVariableId {
        let variable = self.infer.allocate_variable(origin, kind, role);
        self.record_event(CheckEvent::VariableAllocated { variable, kind });

        variable
    }

    /// Narrow one open variable's component to a numeric kind.
    pub(in crate::sema) fn join_variable_kind(
        &mut self,
        variable: dir::TypeVariableId,
        kind: VariableKind,
    ) -> CompilerResult<()> {
        let root = self.infer.alias_root(variable)?;
        let joined = self.infer.variable(root)?.kind.join(kind);
        self.infer.variable_mut(root)?.kind = joined;

        Ok(())
    }

    /// Return the kind of one variable's alias root.
    pub(in crate::sema) fn root_kind(
        &self,
        variable: dir::TypeVariableId,
    ) -> CompilerResult<VariableKind> {
        Ok(self.infer.variable(self.infer.alias_root(variable)?)?.kind)
    }

    /// Return the alias root of one open numeric variable type, `None` for every other type.
    pub(in crate::sema) fn numeric_root(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::TypeVariableId>> {
        let Some(root) = self.root_variable(ty)? else {
            return Ok(None);
        };

        Ok((self.root_kind(root)? != VariableKind::Type).then_some(root))
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

    /// Resolve the given variables in place through every fallback stage.
    pub(in crate::sema) fn resolve_variables(
        &mut self,
        variables: &[dir::TypeVariableId],
    ) -> CompilerResult<()> {
        let roots = self.variable_roots(variables)?;

        // alternate bound resolution and declared defaults until nothing applies
        loop {
            // resolve from bounds to a fixpoint, unblocking each dependent solution
            while self.resolve_scope_roots(&roots, FallbackStage::Bounded)? {}

            if !self.apply_defaults(&roots)? {
                break;
            }
        }

        Ok(())
    }

    /// Commit the widened union of one variable's literal lower bounds, so a contextual slot
    /// read before its call settles reads a widened type.
    pub(in crate::sema) fn fix_literal_candidates(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let Some(root) = self.root_variable(ty)? else {
            return Ok(());
        };

        // a const or scalar-bounded parameter keeps its literals
        if let VariableRole::Instantiation { parameter } = self.infer.variable_role(root)? {
            let origin = self.infer.origin(self.infer.variable(root)?.origin);
            if self.parameter_keeps_literals(origin, parameter)? {
                return Ok(());
            }
        }
        let lower = self
            .infer
            .variables
            .side_bounds(root, BoundSide::Lower)?
            .collect::<SmallVec<[TypeBound; 2]>>();
        if lower.is_empty() {
            return Ok(());
        }
        let mut widened = SmallVec::<[dir::GlobalTypeId; 2]>::new();
        for bound in lower {
            let ty = self.shallow_resolve(bound.ty)?;
            if !matches!(self.ty(ty)?, dir::Type::Literal(_)) {
                return Ok(());
            }
            widened.push(self.widen_type(ty)?);
        }
        let solution = self.normalized_union_type(widened)?;

        self.commit_solution(root, solution)
    }

    /// Apply one declared default to the first open root that carries one.
    pub(in crate::sema) fn apply_defaults(
        &mut self,
        roots: &[dir::TypeVariableId],
    ) -> CompilerResult<bool> {
        for root in roots {
            if self.infer.variable(*root)?.state.is_open()
                && let Some(default) = self.variable_default(*root)?
                && self.resolve_variable(*root, &[default], FallbackStage::Bounded)?
            {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Apply one declared default to one of a scope's open variables.
    pub(in crate::sema) fn apply_scope_default(
        &mut self,
        scope: InferenceScope,
    ) -> CompilerResult<bool> {
        let roots = self.open_scope_variables(scope)?;

        self.apply_defaults(&roots)
    }

    /// Collect the distinct component roots of one variable list.
    fn variable_roots(
        &mut self,
        variables: &[dir::TypeVariableId],
    ) -> CompilerResult<SmallVec<[dir::TypeVariableId; 4]>> {
        let mut roots = SmallVec::<[dir::TypeVariableId; 4]>::new();
        for variable in variables {
            let root = self.infer.alias_root(*variable)?;
            if !roots.contains(&root) {
                roots.push(root);
            }
        }

        Ok(roots)
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

    /// Return whether one generic parameter keeps literal candidates: a const parameter, or one
    /// bounded by a scalar family.
    pub(in crate::sema) fn parameter_keeps_literals(
        &mut self,
        origin: Origin,
        parameter: GenericParameterId,
    ) -> CompilerResult<bool> {
        if self.require_generic_parameter(parameter)?.is_const {
            return Ok(true);
        }
        for bound in self.declared_parameter_bounds(parameter)? {
            let bound = self.normalize(origin, bound)?;
            if self.scalar_families(origin, bound)?.is_some() {
                return Ok(true);
            }
        }

        Ok(false)
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
        let mut numeric_lower = SmallVec::<[TypeBound; 2]>::new();
        let mut has_recursive_bound = false;
        for bound in &lower {
            // a bound aliased onto this variable itself carries no information
            if self.root_variable(bound.ty)? == Some(variable) {
                continue;
            }
            if self.type_contains_variable(bound.ty, variable)? {
                has_recursive_bound = true;
                continue;
            }
            // a numeric variable follows another variable's solution
            if self.numeric_root(bound.ty)?.is_some() {
                numeric_lower.push(*bound);
                continue;
            }
            closed_lower.push(*bound);
        }

        // split the upper bounds, keeping the equation and the directed expectations
        let upper = self
            .infer
            .variables
            .side_bounds(variable, BoundSide::Upper)?
            .collect::<SmallVec<[TypeBound; 2]>>();
        let mut closed_upper = SmallVec::<[TypeBound; 4]>::new();
        let mut contextual_types = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        let mut equation = None;
        for bound in &upper {
            if self.root_variable(bound.ty)? == Some(variable) {
                continue;
            }
            if self.type_contains_variable(bound.ty, variable)? {
                has_recursive_bound = true;
                continue;
            }
            closed_upper.push(*bound);
            if bound.relation == Relation::Equal {
                equation = Some(bound.ty);
            }
            if matches!(
                bound.relation,
                Relation::Assignable | Relation::Widens | Relation::Subtype | Relation::Extends
            ) {
                contextual_types.push(bound.ty);
            }
        }

        // admit the lower candidates, a numeric variable taking its family from typed bounds
        let state = *self.infer.variable(variable)?;
        let origin = self.infer.origin(state.origin);
        let has_equation = closed_lower
            .iter()
            .any(|bound| bound.relation == Relation::Equal);
        let mut lower_types = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        for bound in &closed_lower {
            if has_equation && bound.relation != Relation::Equal {
                continue;
            }
            if !self.admits_candidate(origin, state.kind, bound.ty)? {
                continue;
            }
            lower_types.push(bound.ty);
        }
        let mut typed = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        for contextual in contextual_types {
            if self.admits_candidate(origin, state.kind, contextual)? {
                typed.push(contextual);
            }
        }
        contextual_types = typed;

        // a variable bounded below by numeric variables alone is that numeric variable
        if lower_types.is_empty()
            && contextual_types.is_empty()
            && equation.is_none()
            && let Some(bound) = numeric_lower.first()
            && let Some(numeric) = self.root_variable(bound.ty)?
        {
            self.alias_variable(variable, numeric)?;

            return Ok(true);
        }

        // form the preferred lower and contextual solutions once
        let is_unconstrained_recursion = has_recursive_bound
            && state.kind == VariableKind::Type
            && lower_types.is_empty()
            && contextual_types.is_empty()
            && numeric_lower.is_empty();
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
        }
        // take an infer binder's covariant candidates over its contravariant bounds
        else if let Some(lower_solution) = lower_solution
            && self.infer.variable_role(variable)? == VariableRole::Binder
        {
            Some(lower_solution)
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
                // adopt a candidate closed at the final settle, or open only in numeric variables
                Verdict::Ambiguous
                    if (stage == FallbackStage::Final
                        && !self.type_flags(lower_solution)?.has_variable())
                        || self.is_open_only_numerically(lower_solution)? =>
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
        }
        // a numeric variable nothing decided falls back to its kind's default at the end
        else if stage == FallbackStage::Final
            && let Some(fallback) = state.kind.fallback()
        {
            Some(self.intern_type(fallback)?)
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

    /// Return whether one type resolves to a scalar literal.
    fn is_open_only_numerically(&mut self, ty: dir::GlobalTypeId) -> CompilerResult<bool> {
        let variables = self.type_variables(ty)?;
        if variables.is_empty() {
            return Ok(false);
        }
        for variable in variables {
            if self.root_kind(variable)? == VariableKind::Type {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Return whether one bound may solve a variable of the given kind.
    ///
    /// A numeric variable takes the typed members of its family, an integer one adapting to
    /// floats as well.
    fn admits_candidate(
        &mut self,
        origin: Origin,
        kind: VariableKind,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let domains: &[dir::ScalarDomain] = match kind {
            VariableKind::Type => return Ok(true),
            VariableKind::Integer => &[dir::ScalarDomain::Integer, dir::ScalarDomain::Float],
            VariableKind::Float => &[dir::ScalarDomain::Float],
        };
        let ty = self.shallow_resolve(ty)?;
        if matches!(self.ty(ty)?, dir::Type::Literal(_)) {
            return Ok(false);
        }

        Ok(self.scalar_families(origin, ty)?.is_some_and(|families| {
            domains
                .iter()
                .any(|domain| families.contains(dir::ScalarFamily::Domain(*domain)))
        }))
    }

    /// Return whether a live check may still push a new bound onto one variable.
    fn variable_may_grow(&self, variable: dir::TypeVariableId) -> CompilerResult<bool> {
        let Some(producers) = self.fulfill.producers.get(&variable) else {
            return Ok(false);
        };
        let waiters = self.fulfill.waiting.get(&Wake::Variable(variable));

        // find one live producer standing outside the variable's own waiters
        for id in producers {
            let is_live = self.fulfill.checks.state(*id) != WorkState::Done;
            let is_waiting_on_self = waiters.is_some_and(|waiters| waiters.contains(id));
            if is_live && !is_waiting_on_self {
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

        // forward the aliased variable onto the root, the component taking the narrower kind
        let kind = self
            .infer
            .variable(root)?
            .kind
            .join(self.infer.variable(aliased)?.kind);
        self.infer.variable_mut(root)?.kind = kind;
        self.infer.variable_mut(aliased)?.state = VariableState::Alias(root);
        self.fulfill.wake(Wake::Variable(aliased));
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
        self.fulfill.wake(Wake::Variable(variable));

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

        // numeric variables bounding one variable from below join into one family variable
        if side == BoundSide::Lower
            && let Some(root) = self.numeric_root(bound)?
        {
            let lower = self
                .infer
                .variables
                .side_bounds(self.infer.alias_root(variable)?, BoundSide::Lower)?
                .collect::<SmallVec<[TypeBound; 2]>>();
            for known in lower {
                if let Some(other) = self.numeric_root(known.ty)?
                    && other != root
                {
                    self.alias_variable(root, other)?;
                }
            }
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
