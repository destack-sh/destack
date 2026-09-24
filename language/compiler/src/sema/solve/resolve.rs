use destack_core::FxIndexSet;
use destack_dir as dir;
use smallvec::SmallVec;

use crate::sema::{
    Bound, BoundSide, CauseId, CheckEvent, CheckOutcome, CheckState, GenericParameterId, Origin,
    Relation, RelationCheck, Settle, TypeSubstitution, VariableBounds, VariableKind, VariableState,
    Verdict, Wake, WorkState,
};
use crate::{CompilerError, CompilerResult};

/// How the unbound memory parameters of one substitution ground.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) enum MemoryGrounding {
    /// Open each parameter for inference at the site.
    Open,
    /// Bind each parameter at its elided default.
    Elided,
}

impl CheckState<'_> {
    /// Allocate one inference variable.
    pub(in crate::sema) fn open_variable(&mut self, origin: Origin) -> dir::TypeVariableId {
        self.open_variable_of(origin, VariableKind::Type)
    }

    /// Allocate one inference variable instantiating a declared generic parameter.
    pub(in crate::sema) fn open_instantiation(
        &mut self,
        origin: Origin,
        parameter: dir::GlobalGenericParameterId,
        kind: VariableKind,
    ) -> CompilerResult<dir::TypeVariableId> {
        let variable = self.open_variable_of(origin, kind);
        self.infer.variables.get_mut(variable)?.parameter = Some(parameter);

        Ok(variable)
    }

    /// Open one omitted generic parameter at its kind, eliding an undeclared memory default.
    pub(in crate::sema) fn open_omitted_parameter(
        &mut self,
        origin: Origin,
        parameter: dir::GlobalGenericParameterId,
    ) -> CompilerResult<dir::TypeVariableId> {
        let binding = self.require_generic_parameter(parameter)?.clone();
        let memory = match (binding.memory_parameter(), binding.constraint) {
            (Some(kind), _) => Some(kind),
            (None, Some(constraint)) => self.memory_kind(constraint)?,
            (None, None) => None,
        };
        let kind = memory.map_or(VariableKind::Type, VariableKind::Memory);
        let variable = self.open_instantiation(origin, parameter, kind)?;

        // elide a memory parameter without a declared default at its kind's value
        if binding.default.is_none()
            && let Some(memory) = memory
        {
            let default = self.elided_memory_default(memory)?;
            self.infer.set_variable_default(variable, default)?;
        }

        Ok(variable)
    }

    /// Ground the unbound memory parameters of one substitution.
    pub(in crate::sema) fn ground_memory_parameters(
        &mut self,
        origin: Origin,
        parameters: &[dir::GlobalGenericParameterId],
        substitution: &mut TypeSubstitution,
        grounding: MemoryGrounding,
    ) -> CompilerResult<()> {
        // open every unbound parameter, or bind each memory parameter at its elided default
        match grounding {
            MemoryGrounding::Open => self.open_unbound_parameters(origin, parameters, substitution),
            MemoryGrounding::Elided => {
                for parameter in parameters.iter().copied() {
                    if substitution.argument(parameter).is_some() {
                        continue;
                    }
                    let binding = self.generic_parameter(parameter)?.ok_or_else(|| {
                        CompilerError::Internal {
                            message: format!(
                                "a generic parameter {parameter:?} without its binding"
                            ),
                        }
                    })?;
                    let Some(kind) = binding.memory_parameter() else {
                        continue;
                    };
                    let witness = self.elided_memory_default(kind)?;
                    substitution.bind(parameter, witness)?;
                }

                Ok(())
            }
        }
    }

    /// Open every unbound parameter of one substitution for inference.
    pub(in crate::sema) fn open_unbound_parameters(
        &mut self,
        origin: Origin,
        parameters: &[dir::GlobalGenericParameterId],
        substitution: &mut TypeSubstitution,
    ) -> CompilerResult<()> {
        for parameter in parameters.iter().copied() {
            if substitution.argument(parameter).is_some() {
                continue;
            }
            let variable = self.open_omitted_parameter(origin, parameter)?;
            let ty = self.intern_type(dir::Type::Variable(variable))?;
            substitution.bind(parameter, ty)?;
        }

        Ok(())
    }

    /// Allocate one memory-kinded inference variable as a type term.
    pub(in crate::sema) fn open_memory_type(
        &mut self,
        origin: Origin,
        kind: dir::MemoryParameter,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // open a memory variable carrying its elided default
        let variable = self.open_variable_of(origin, VariableKind::Memory(kind));
        let default = self.elided_memory_default(kind)?;
        self.infer.set_variable_default(variable, default)?;

        self.variable_type(variable)
    }

    /// Allocate one inference variable of the given kind.
    pub(in crate::sema) fn open_variable_of(
        &mut self,
        origin: Origin,
        kind: VariableKind,
    ) -> dir::TypeVariableId {
        let variable = self.infer.open_variable(origin, kind);
        self.record_event(CheckEvent::VariableAllocated { variable, kind });

        variable
    }

    /// Open one join slot, which collects the union of the values it receives.
    pub(in crate::sema) fn open_join_variable(
        &mut self,
        origin: Origin,
    ) -> CompilerResult<dir::TypeVariableId> {
        let variable = self.open_variable(origin);
        self.infer.variable_mut(variable)?.is_join = true;

        Ok(variable)
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

        Ok(self.root_kind(root)?.is_numeric().then_some(root))
    }

    /// Set the declared default completing one variable when inference stays dry.
    pub(in crate::sema) fn set_variable_default(
        &mut self,
        variable: dir::TypeVariableId,
        default: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        self.infer.set_variable_default(variable, default)
    }

    /// Mark the variables one function body reads, fixing them to their candidates so far.
    pub(in crate::sema) fn fix_variables(
        &mut self,
        variables: &[dir::TypeVariableId],
    ) -> CompilerResult<()> {
        // fix each variable's component root
        for variable in variables {
            let root = self.infer.alias_root(*variable)?;
            self.infer.variable_mut(root)?.is_fixed = true;
        }

        Ok(())
    }

    /// Return one variable's alias root, or none once the variable is solved.
    pub(in crate::sema) fn open_root(
        &self,
        variable: dir::TypeVariableId,
    ) -> CompilerResult<Option<dir::TypeVariableId>> {
        let variable = self.infer.alias_root(variable)?;
        let state = self.infer.variable(variable)?;

        Ok(state.state.is_open().then_some(variable))
    }

    /// Return the memory parameter kind one variable ranges over, however it arose.
    pub(in crate::sema) fn variable_memory_parameter(
        &self,
        variable: dir::TypeVariableId,
    ) -> CompilerResult<Option<dir::MemoryParameter>> {
        // read the kind from the variable itself or from its declared parameter
        let variable = self.infer.variable(variable)?;
        let kind = match (variable.kind, variable.parameter) {
            (VariableKind::Memory(kind), _) => Some(kind),
            (_, Some(parameter)) => self
                .generic_parameter(parameter)?
                .and_then(|binding| binding.memory_parameter()),
            (_, None) => None,
        };

        Ok(kind)
    }

    /// Resolve variables at one stage to a fixpoint, each solution unblocking the next.
    pub(in crate::sema) fn settle_variables(
        &mut self,
        variables: &[dir::TypeVariableId],
        stage: Settle,
    ) -> CompilerResult<()> {
        let roots = self.variable_roots(variables)?;
        while self.resolve_roots(&roots, stage)? {}

        Ok(())
    }

    /// Collect the distinct component roots of one variable list.
    fn variable_roots(
        &mut self,
        variables: &[dir::TypeVariableId],
    ) -> CompilerResult<SmallVec<[dir::TypeVariableId; 4]>> {
        // collect each distinct component root once
        let mut roots = SmallVec::<[dir::TypeVariableId; 4]>::new();
        for variable in variables {
            let root = self.infer.alias_root(*variable)?;
            if !roots.contains(&root) {
                roots.push(root);
            }
        }

        Ok(roots)
    }

    /// Resolve every open variable this scope allocated once, in allocation order.
    pub(in crate::sema) fn resolve_scope(
        &mut self,
        scope: usize,
        stage: Settle,
    ) -> CompilerResult<bool> {
        let roots = if stage.settles() {
            self.settleable_scope_variables(scope)?
        } else {
            self.open_scope_variables(scope)?
        };

        self.resolve_roots(&roots, stage)
    }

    /// Return the open variables one scope settles, leaving those a live check still binds.
    fn settleable_scope_variables(
        &mut self,
        scope: usize,
    ) -> CompilerResult<SmallVec<[dir::TypeVariableId; 2]>> {
        let mut settleable = SmallVec::new();
        for root in self.open_scope_variables(scope)? {
            let is_bound = self
                .fulfill
                .binders
                .get(&root)
                .is_some_and(|binder| self.fulfill.checks.state(*binder) != WorkState::Done);
            if !is_bound {
                settleable.push(root);
            }
        }

        Ok(settleable)
    }

    /// Resolve every still open root once, in order.
    fn resolve_roots(
        &mut self,
        roots: &[dir::TypeVariableId],
        stage: Settle,
    ) -> CompilerResult<bool> {
        // resolve every root that is still open
        let mut resolved = false;
        for root in roots {
            let root = self.infer.alias_root(*root)?;
            if self.infer.variable(root)?.state.is_open() {
                resolved |= self.resolve_variable(root, stage)?;
            }
        }

        Ok(resolved)
    }

    /// Return the open variables owned by one inference scope.
    pub(in crate::sema) fn open_scope_variables(
        &self,
        scope: usize,
    ) -> CompilerResult<SmallVec<[dir::TypeVariableId; 2]>> {
        // collect the scope's still-open variables in allocation order
        let mut open = SmallVec::new();
        for index in scope..self.infer.variable_count() {
            let variable = dir::TypeVariableId(index as u32);
            let state = self.infer.variable(variable)?;
            if state.state.is_open() && !state.is_dead {
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
        // require every variable the declared default references to be solved already
        let Some(default) = self.infer.variable(variable)?.default else {
            return Ok(None);
        };
        for dependency in self.type_variables(default)? {
            if dependency == variable || self.infer.solution(dependency)?.is_none() {
                return Ok(None);
            }
        }

        Ok(Some(default))
    }

    /// Return whether one generic parameter keeps a literal candidate.
    pub(in crate::sema) fn parameter_keeps_literals(
        &mut self,
        origin: Origin,
        parameter: GenericParameterId,
        literal: dir::GlobalTypeId,
        is_fixed: bool,
    ) -> CompilerResult<bool> {
        // keep literals for a const parameter
        let binding = self.require_generic_parameter(parameter)?.clone();
        if binding.is_const {
            return Ok(true);
        }

        // keep literals for a parameter whose declared bounds name the literal's domain
        for bound in self.declared_parameter_bounds(parameter)? {
            if self.type_keeps_literal(origin, literal, bound, true)? {
                return Ok(true);
            }
        }

        // widen numeric literals, and widen every literal a fixed parameter holds
        let literal = self.shallow_resolve(literal)?;
        if is_fixed || self.numeric_literal_kind(literal)?.is_some() {
            return Ok(false);
        }

        // keep literals the owner's return type exposes through the parameter
        let template = dir::GlobalGenericTemplateId::new(parameter.module_id, binding.template);
        let Some(owner) = self
            .generic_template(template)?
            .and_then(|template| template.symbol)
        else {
            return Ok(false);
        };
        let Some(ty) = self.adopt_symbol_type_maybe(owner)? else {
            return Ok(false);
        };
        let Some(head) = self.signature_head(ty)? else {
            return Ok(false);
        };
        let Some(return_type) = head.return_type else {
            return Ok(false);
        };

        self.has_exposed_type(return_type, binding.ty)
    }

    /// Return whether one type names literals of a literal's domain.
    pub(in crate::sema) fn type_keeps_literal(
        &mut self,
        origin: Origin,
        literal: dir::GlobalTypeId,
        ty: dir::GlobalTypeId,
        is_bound: bool,
    ) -> CompilerResult<bool> {
        // read the literal's scalar domain
        let literal = self.shallow_resolve(literal)?;
        let domain = match self.ty(literal)? {
            dir::Type::Union(union) => self.scalar_literal_union_domain(literal, union)?,
            shape => shape.scalar_domain(),
        };

        // read the arms the named type offers
        let bound = self.normalize(origin, ty)?;
        let arms = match self.ty(bound)? {
            dir::Type::Union(union) => self.type_ids(bound.module_id, union.elements)?,
            _ => std::slice::from_ref(&bound),
        };

        // accept any arm naming the same domain
        for arm in arms {
            let arm = self.shallow_resolve(*arm)?;
            let keeps = match self.ty(arm)? {
                dir::Type::Literal(_) => self.ty(arm)?.scalar_domain() == domain,
                dir::Type::Primitive(_) => is_bound && self.ty(arm)?.scalar_domain() == domain,
                dir::Type::Key(dir::StaticKey::Name(_)) => {
                    domain == Some(dir::ScalarDomain::String)
                }
                dir::Type::Operation(operation) => {
                    domain == Some(dir::ScalarDomain::String)
                        && matches!(
                            self.type_operation(arm.module_id, operation)?,
                            dir::TypeOperation::TemplateLiteral(_)
                        )
                }
                dir::Type::Union(_) | dir::Type::Reference(_) | dir::Type::Application(_) => {
                    arm != bound && self.type_keeps_literal(origin, literal, arm, is_bound)?
                }
                _ => false,
            };
            if keeps {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Return whether a type exposes another type through transparent alternatives.
    pub(in crate::sema) fn has_exposed_type(
        &self,
        ty: dir::GlobalTypeId,
        exposed: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // seed the walk at the named type
        let mut pending = SmallVec::<[dir::GlobalTypeId; 4]>::from_slice(&[ty]);
        let mut visited = SmallVec::<[dir::GlobalTypeId; 8]>::new();

        // follow forms, unions, intersections, and conditional alternatives
        while let Some(ty) = pending.pop() {
            if ty == exposed {
                return Ok(true);
            }
            if visited.contains(&ty) {
                continue;
            }
            visited.push(ty);

            match self.ty(ty)? {
                dir::Type::Form(form) => pending.push(form.value),
                dir::Type::Union(union) => {
                    pending.extend(self.type_ids(ty.module_id, union.elements)?.iter().copied());
                }
                dir::Type::Intersection(intersection) => {
                    pending.extend(
                        self.type_ids(ty.module_id, intersection.elements)?
                            .iter()
                            .copied(),
                    );
                }
                dir::Type::Operation(_) => {
                    if let Some(dir::TypeOperation::Conditional(conditional)) =
                        self.operation_head(ty)?
                    {
                        pending.push(conditional.then_type);
                        pending.push(conditional.else_type);
                    }
                }
                _ => {}
            }
        }

        Ok(false)
    }

    /// Resolve one open variable to its one candidate, committing it.
    pub(in crate::sema) fn resolve_variable(
        &mut self,
        variable: dir::TypeVariableId,
        stage: Settle,
    ) -> CompilerResult<bool> {
        // read the variable's state and its origin
        let state = *self.infer.variable(variable)?;
        let origin = self.infer.origin(state.origin);

        // split the lower bounds into closed candidates and open numeric variables flowing in
        let lower = self
            .infer
            .variables
            .side_bounds(variable, BoundSide::Lower)?
            .collect::<SmallVec<[Bound; 2]>>();
        let mut lower_types = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        let mut numeric_lower = SmallVec::<[dir::TypeVariableId; 2]>::new();
        let mut has_recursive_bound = false;
        let has_equation = lower.iter().any(|bound| bound.relation == Relation::Equal);
        for bound in &lower {
            if self.root_variable(bound.ty)? == Some(variable) {
                continue;
            }
            if self.type_contains_variable(bound.ty, variable)? {
                has_recursive_bound = true;
            } else if let Some(root) = self.numeric_root(bound.ty)? {
                numeric_lower.push(root);
            } else if (!has_equation || bound.relation == Relation::Equal)
                && self.is_candidate_kind(state.kind, bound.ty)?
            {
                lower_types.push(bound.ty);
            }
        }

        // split the upper bounds into the equation, the storage context, and the declared bounds
        let upper = self
            .infer
            .variables
            .side_bounds(variable, BoundSide::Upper)?
            .collect::<SmallVec<[Bound; 2]>>();
        let mut contextual_types = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        let mut bound_types = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        let mut equation = None;
        let is_access = state.kind == VariableKind::Memory(dir::MemoryParameter::Access);
        for bound in &upper {
            if self.root_variable(bound.ty)?.is_some() {
                continue;
            }
            if self.type_contains_variable(bound.ty, variable)? {
                has_recursive_bound = true;
                continue;
            }
            let candidate = self.kind_candidate(origin, state.kind, bound.ty)?;
            match (bound.relation, candidate) {
                (Relation::Equal, _) => equation = Some(bound.ty),
                // leave an access grant out of the solution arms
                (Relation::Storable, Some(_)) if is_access => {}
                (Relation::Storable, Some(candidate)) if !contextual_types.contains(&candidate) => {
                    contextual_types.push(candidate);
                }
                (Relation::Subtype, Some(candidate)) if !bound_types.contains(&candidate) => {
                    bound_types.push(candidate);
                }
                _ => {}
            }
        }

        // join a variable bounded below by numeric variables alone with them
        if lower_types.is_empty()
            && equation.is_none()
            && let Some(first) = numeric_lower.first().copied()
            && (upper.is_empty() || stage != Settle::Possible || state.is_fixed)
        {
            for other in &numeric_lower[1..] {
                self.alias_variable(first, *other)?;
            }
            self.alias_variable(variable, first)?;

            return Ok(!self.infer.variable(variable)?.state.is_open());
        }

        // keep an open variable open until the final stage
        if stage == Settle::Possible && equation.is_none() && !state.is_fixed {
            return Ok(false);
        }

        // choose the one lower candidate, joining the candidates a memory hole collects
        let lower_solution = match (lower_types.as_slice(), state.kind) {
            ([], _) => None,
            ([_, _, ..], VariableKind::Memory(kind)) => {
                match self.join_memory_candidates(kind, &lower_types)? {
                    Some(joined) => Some(joined),
                    None => Some(self.best_common(variable, &lower_types)?),
                }
            }
            _ => Some(self.best_common(variable, &lower_types)?),
        };

        // widen the literals a fixed parameter lets go
        let mut is_widened = false;
        let lower_solution = match (lower_solution, state.parameter) {
            (Some(lower), Some(parameter))
                if state.is_fixed
                    && self.is_literal_shape(lower)?
                    && !self.parameter_keeps_literals(origin, parameter, lower, true)? =>
            {
                is_widened = true;

                Some(self.widen_fresh(origin, lower)?)
            }
            (lower, _) => lower,
        };

        // meet the storage context bounds
        let contextual = match contextual_types.as_slice() {
            [] => None,
            [single] => Some(*single),
            _ => Some(self.normalized_intersection_type(contextual_types.iter().copied())?),
        };

        // take the lower solution its upper bounds admit
        let (lower_solution, contextual) = match (lower_solution, contextual) {
            (Some(lower), Some(context)) => {
                let mut admitted = Verdict::Holds;
                for bound in &upper {
                    if self.root_variable(bound.ty)?.is_some() {
                        continue;
                    }
                    let verdict = self.decide_relation(origin, bound.relation, lower, bound.ty)?;
                    admitted = admitted.and(verdict);
                }
                match admitted {
                    Verdict::Ambiguous if stage == Settle::Possible => return Ok(false),
                    // take the context over a lower candidate its bounds refuse
                    Verdict::Fails => (None, Some(context)),
                    // a widened literal takes the context that includes it
                    _ if is_widened
                        && self.decide_relation(origin, Relation::Subtype, lower, context)?
                            == Verdict::Holds =>
                    {
                        (None, Some(context))
                    }
                    _ => (Some(lower), Some(context)),
                }
            }
            other => other,
        };

        // settle a variable by its declared bound once the body's inference ends
        let settles_bound = match state.kind {
            VariableKind::Memory(_) => false,
            _ => stage.settles(),
        };
        let bound = match (settles_bound, bound_types.as_slice()) {
            (true, [single]) => Some(*single),
            (true, [_, ..]) => {
                Some(self.normalized_intersection_type(bound_types.iter().copied())?)
            }
            _ => None,
        };

        // read the default and the fallback this stage allows
        let (default, fallback) = match stage {
            Settle::Final if !self.infer.is_deciding() => {
                (self.variable_default(variable)?, state.kind.fallback())
            }
            Settle::Final | Settle::All => (self.variable_default(variable)?, None),
            Settle::Possible => (None, None),
        };

        // choose the solution in candidate order
        let solution = match (
            equation,
            lower_solution,
            contextual,
            default.or(bound),
            fallback,
        ) {
            (Some(equation), ..) => equation,
            (None, Some(lower), ..) => lower,
            (None, None, Some(contextual), ..) => contextual,
            (None, None, None, Some(default), _) => default,
            (None, None, None, None, Some(fallback)) => self.intern_type(fallback)?,
            // report a structural cycle with no candidate as an infinite type
            (None, None, None, None, None)
                if has_recursive_bound && stage.settles() && state.kind.fallback().is_none() =>
            {
                let error = self.circular_type_error(origin)?;
                self.report(origin.module(), error);

                self.intern_type(dir::Type::Error)?
            }
            (None, None, None, None, None) => return Ok(false),
        };

        // commit the candidate's reduced head
        let solution = self.shallow_resolve(solution)?;
        let solution = self.reduce_redundant_forms(origin, solution)?;
        self.commit_solution(variable, solution)?;

        Ok(!self.infer.variable(variable)?.state.is_open())
    }

    /// Return whether some primitive of a numeric variable's kind can satisfy one upper bound.
    fn numeric_bound_admits(
        &mut self,
        origin: Origin,
        variable: dir::TypeVariableId,
        bound: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // admit a bound some primitive of the kind inhabits
        let kind = self.infer.variable(variable)?.kind;
        let bound = self.normalize(origin, bound)?;
        let admits = match self.ty(bound)? {
            dir::Type::Intersection(intersection) => {
                let members = self.type_ids(bound.module_id, intersection.elements)?;
                let mut admits = true;
                for member in members {
                    admits &= self.numeric_bound_admits(origin, variable, *member)?;
                }
                admits
            }
            dir::Type::Union(union) => {
                let arms = self.type_ids(bound.module_id, union.elements)?;
                let mut admits = false;
                for arm in arms {
                    admits |= self.numeric_bound_admits(origin, variable, *arm)?;
                }
                admits
            }
            dir::Type::Primitive(_) => self.is_candidate_kind(kind, bound)?,
            dir::Type::Function(_)
            | dir::Type::FunctionSignature(_)
            | dir::Type::FunctionPointer(_)
            | dir::Type::Object(_)
            | dir::Type::Tuple(_)
            | dir::Type::Slice(_)
            | dir::Type::FixedArray(_)
            | dir::Type::Null
            | dir::Type::Undefined
            | dir::Type::Void
            | dir::Type::Never => false,
            // refuse a rigid parameter for a numeric variable
            dir::Type::Parameter(_) => false,
            // admit an interface some primitive of the kind conforms to
            dir::Type::Application(_) => {
                self.is_conformance_target(bound)?
                    && self.kind_primitive_conforms(origin, variable, bound)?
            }
            dir::Type::Literal(_) | dir::Type::Range(_) | dir::Type::Key(_) => {
                let widened = self.widen_type(bound)?;

                self.is_candidate_kind(kind, widened)?
            }
            dir::Type::Form(form) => self.numeric_bound_admits(origin, variable, form.value)?,
            dir::Type::Unknown
            | dir::Type::Error
            | dir::Type::Variable(_)
            | dir::Type::Erased(_)
            | dir::Type::Member(_)
            | dir::Type::Operation(_)
            | dir::Type::Refined(_)
            | dir::Type::This => true,
            _ => false,
        };

        Ok(admits)
    }

    /// Return whether some primitive of a numeric variable's kind conforms to one interface bound.
    fn kind_primitive_conforms(
        &mut self,
        origin: Origin,
        variable: dir::TypeVariableId,
        bound: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let kind = self.infer.variable(variable)?.kind;
        let variable = self.variable_type(variable)?;
        for domain in kind.domains() {
            for primitive in domain.primitives() {
                let primitive = self.intern_type(dir::Type::Primitive(*primitive))?;
                let bound = self.replace_type(bound, variable, primitive)?;
                if self.decide_relation(origin, Relation::Subtype, primitive, bound)?
                    != Verdict::Fails
                {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Return whether one variable's bounds on one side lead to another variable.
    fn bounds_reach(
        &mut self,
        variable: dir::TypeVariableId,
        side: BoundSide,
        target: dir::TypeVariableId,
    ) -> CompilerResult<bool> {
        let known: SmallVec<[_; 4]> = self
            .infer
            .variables
            .side_bounds(variable, side)?
            .map(|known| known.ty)
            .collect();
        for ty in known {
            if self.root_variable(ty)? == Some(target) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Return the arms of one bound a variable kind can take, none when no arm fits.
    fn kind_candidate(
        &mut self,
        origin: Origin,
        kind: VariableKind,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // read the arms a memory or numeric kind distributes over
        let ty = self.structurally_normalize(origin, ty)?;
        let arms = match self.ty(ty)? {
            dir::Type::Union(union) if kind != VariableKind::Type => {
                self.type_ids(ty.module_id, union.elements)?
            }
            _ => std::slice::from_ref(&ty),
        };

        // keep the arms the kind can take
        let mut candidates = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        for arm in arms {
            if self.is_candidate_kind(kind, *arm)? {
                candidates.push(*arm);
            }
        }

        // join what the arms leave
        match candidates.as_slice() {
            [] => Ok(None),
            [single] => Ok(Some(*single)),
            _ => Ok(Some(
                self.normalized_union_type(candidates.iter().copied())?,
            )),
        }
    }

    /// Return whether one bound type may solve a variable of the given kind.
    fn is_candidate_kind(
        &mut self,
        kind: VariableKind,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // admit the bound by the variable kind
        let ty = self.shallow_resolve(ty)?;
        match kind {
            // a type variable takes every bound
            VariableKind::Type => Ok(true),

            // solve a memory slot to a term of its own kind
            VariableKind::Memory(memory) => Ok(self.memory_kind(ty)? == Some(memory)),

            // solve a numeric variable to a scalar of its domain
            VariableKind::Integer | VariableKind::Float => Ok(self
                .ty(ty)?
                .scalar_domain()
                .is_some_and(|domain| kind.domains().contains(&domain))),
        }
    }

    /// Return whether one type names constant values, alone or as a union of them.
    pub(in crate::sema) fn is_literal_shape(&self, ty: dir::GlobalTypeId) -> CompilerResult<bool> {
        // read whether the head names constants
        let ty = self.shallow_resolve(ty)?;
        Ok(match self.ty_raw(ty)? {
            dir::Type::Operation(operation) => matches!(
                self.type_operation(ty.module_id, operation)?,
                dir::TypeOperation::TemplateLiteral(_)
            ),
            dir::Type::Union(union) => {
                let mut is_literal = true;
                for arm in self.type_ids(ty.module_id, union.elements)? {
                    is_literal &= self.ty_raw(*arm)?.is_constant();
                }

                is_literal
            }
            shape => shape.is_constant(),
        })
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
            // skip closed subtrees without variables
            if !self.type_flags(id)?.has_variable() {
                continue;
            }

            let ty = self.ty_raw(id)?;

            // follow solved variables and collect open variables once
            if let dir::Type::Variable(variable) = ty {
                if let Some(solution) = self.infer.solution(variable)? {
                    pending.push(solution);
                } else if let Some(variable) = self.open_root(variable)?
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

    /// Return every type the graphs of some root types name, the roots included.
    pub(in crate::sema) fn mentioned_types(
        &self,
        roots: impl IntoIterator<Item = dir::GlobalTypeId>,
    ) -> CompilerResult<FxIndexSet<dir::GlobalTypeId>> {
        // walk each graph once, visiting every type a single time
        let mut pending = roots.into_iter().collect::<SmallVec<[_; 8]>>();
        let mut seen = FxIndexSet::default();
        while let Some(id) = pending.pop() {
            if !seen.insert(id) {
                continue;
            }
            let ty = self.ty_raw(id)?;
            self.for_each_type_child(id.module_id, &ty, |child| pending.push(child))?;
        }

        Ok(seen)
    }

    /// Return whether one type contains a variable root.
    pub(in crate::sema) fn type_contains_variable(
        &self,
        ty: dir::GlobalTypeId,
        variable: dir::TypeVariableId,
    ) -> CompilerResult<bool> {
        // scan the type's variables for the root
        for dependency in self.type_variables(ty)? {
            if dependency == variable {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Commit one variable solution, a bound its aliased kind refuses failing it.
    pub(in crate::sema) fn commit_solution(
        &mut self,
        variable: dir::TypeVariableId,
        solution: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        // alias a solution that is itself an open variable
        let variable = self.infer.alias_root(variable)?;
        let solution = self.shallow_resolve(solution)?;
        if let Some(other) = self.root_variable(solution)? {
            return self.alias_variable(variable, other);
        }

        // splice the closed rests of a tuple solution
        let origin_id = self.infer.variable(variable)?.origin;
        let origin = self.infer.origin(origin_id);
        let solution = match self.ty(solution)? {
            dir::Type::Tuple(tuple) => {
                let spliced = self.reduce_tuple_rest_splice(origin, solution, tuple)?;

                spliced.unwrap_or(solution)
            }
            _ => solution,
        };

        // complete circular solutions with the error type
        if self.type_contains_variable(solution, variable)? {
            let origin_id = self.infer.variable(variable)?.origin;
            let origin = self.infer.origin(origin_id);
            let error = self.circular_type_error(origin)?;
            self.report(origin.module(), error);
            let error = self.intern_type(dir::Type::Error)?;

            self.commit_error_solution(variable, error)?;

            return Ok(Verdict::Holds);
        }
        self.commit_variable_solution(variable, VariableState::Resolved(solution))?;

        Ok(Verdict::Holds)
    }

    /// Widen a numeric join variable that meets a candidate outside its domain.
    fn settle_numeric_kind(
        &mut self,
        variable: dir::TypeVariableId,
        origin: Origin,
        cause: CauseId,
    ) -> CompilerResult<()> {
        // require a numeric join slot
        let root = self.infer.alias_root(variable)?;
        let kind = self.root_kind(root)?;
        if !kind.is_numeric() || !self.infer.variable(root)?.is_join {
            return Ok(());
        }

        // look for a closed lower bound outside the numeric domain
        let lower = self
            .infer
            .variables
            .side_bounds(root, BoundSide::Lower)?
            .collect::<SmallVec<[Bound; 2]>>();
        let mut joins = false;
        for bound in &lower {
            if self.root_variable(bound.ty)?.is_none()
                && self.kind_candidate(origin, kind, bound.ty)?.is_none()
            {
                joins = true;
            }
        }
        if !joins {
            return Ok(());
        }

        // widen the slot and join the family default beside that candidate
        self.widen_numeric_slot(root, origin, cause)
    }

    /// Widen one numeric join slot and seed its family fallback candidate.
    pub(in crate::sema) fn widen_numeric_slot(
        &mut self,
        root: dir::TypeVariableId,
        origin: Origin,
        cause: CauseId,
    ) -> CompilerResult<()> {
        let kind = self.root_kind(root)?;
        self.infer.variable_mut(root)?.kind = VariableKind::Type;
        let fallback = kind.fallback().ok_or_else(|| CompilerError::Internal {
            message: format!("numeric kind {kind:?} has no fallback"),
        })?;
        let fallback = self.intern_type(fallback)?;
        let bound = Bound::new(origin, fallback, Relation::Subtype, cause);
        self.infer.push_bound(root, BoundSide::Lower, bound)?;

        Ok(())
    }

    /// Alias one open variable onto an equal variable's root.
    pub(in crate::sema) fn alias_variable(
        &mut self,
        first: dir::TypeVariableId,
        second: dir::TypeVariableId,
    ) -> CompilerResult<Verdict> {
        // read both component roots
        let first = self.infer.alias_root(first)?;
        let second = self.infer.alias_root(second)?;
        if first == second {
            return Ok(Verdict::Holds);
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
            .collect::<SmallVec<[Bound; 2]>>();
        let upper = self
            .infer
            .variables
            .side_bounds(aliased, BoundSide::Upper)?
            .collect::<SmallVec<[Bound; 2]>>();
        let aliased_state = *self.infer.variable(aliased)?;

        // forward the aliased variable onto the root, joining what each declares
        let root_kind = self.infer.variable(root)?.kind;
        let root_state = self.infer.variable_mut(root)?;
        root_state.kind = root_kind.join(aliased_state.kind);
        root_state.is_join |= aliased_state.is_join;
        root_state.parameter = root_state.parameter.or(aliased_state.parameter);
        root_state.is_fixed |= aliased_state.is_fixed;
        self.infer.variable_mut(aliased)?.state = VariableState::Alias(root);
        self.fulfill.wake(Wake::Variable(aliased));

        // re-admit the upper bounds a root joined into a numeric kind already holds
        let mut verdict = Verdict::Holds;
        let joined = self.infer.variable(root)?.kind;
        if joined.is_numeric() && !root_kind.is_numeric() {
            let origin = self.infer.origin(self.infer.variable(root)?.origin);
            let held = self
                .infer
                .variables
                .side_bounds(root, BoundSide::Upper)?
                .map(|bound| bound.ty)
                .collect::<SmallVec<[_; 2]>>();
            for bound in held {
                if self.root_variable(bound)?.is_none()
                    && !self.numeric_bound_admits(origin, root, bound)?
                {
                    verdict = Verdict::Fails;
                }
            }
        }

        // migrate the collected bounds onto the root
        let sides = lower
            .into_iter()
            .map(|bound| (BoundSide::Lower, bound))
            .chain(upper.into_iter().map(|bound| (BoundSide::Upper, bound)));
        for (side, bound) in sides {
            let pushed = self.push_variable_bound(
                root,
                side,
                bound.origin,
                bound.cause,
                bound.ty,
                bound.relation,
            )?;
            if pushed == Verdict::Fails {
                verdict = Verdict::Fails;
            }
        }

        // copy the declared default to a root without one
        if let Some(default) = aliased_state.default
            && self.infer.variable(root)?.default.is_none()
        {
            self.infer.set_variable_default(root, default)?;
        }

        Ok(verdict)
    }

    /// Commit one failed variable solution.
    pub(in crate::sema) fn commit_error_solution(
        &mut self,
        variable: dir::TypeVariableId,
        error: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        self.commit_variable_solution(variable, VariableState::Error(error))
    }

    /// Commit one completed variable state.
    fn commit_variable_solution(
        &mut self,
        variable: dir::TypeVariableId,
        state: VariableState,
    ) -> CompilerResult<()> {
        // move the collected bounds into the completed state
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

        // require the completed state to hold a type
        let ty = state.ty().ok_or_else(|| CompilerError::Internal {
            message: format!("cannot commit open check variable {variable:?}"),
        })?;

        // require exactly one state transition per variable
        let previous = self.infer.variable(variable)?.state;
        if previous == state {
            return Ok(());
        }

        // refuse a second completion
        if !previous.is_open() {
            return Err(CompilerError::Internal {
                message: format!("check variable {variable:?} completed twice"),
            });
        }

        // complete the variable and wake the work watching it
        self.infer.variable_mut(variable)?.state = state;
        self.fulfill.wake(Wake::Variable(variable));
        self.record_event(CheckEvent::VariableSolved {
            variable,
            bounds: Box::new(bounds.clone()),
            solution: ty,
        });

        // discharge the accumulated bounds against the completed type
        for bound in &bounds.lower {
            self.discharge_bound(BoundSide::Lower, bound, ty)?;
        }
        for bound in &bounds.upper {
            self.discharge_bound(BoundSide::Upper, bound, ty)?;
        }

        Ok(())
    }

    /// Join the lower candidates of one memory parameter, a region by extent and space.
    fn join_memory_candidates(
        &mut self,
        kind: dir::MemoryParameter,
        candidates: &[dir::GlobalTypeId],
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        match kind {
            dir::MemoryParameter::Region => {
                let mut extents = SmallVec::<[dir::GlobalTypeId; 4]>::new();
                let mut spaces = SmallVec::<[dir::GlobalTypeId; 4]>::new();
                for candidate in candidates {
                    match self.resolved_ty(*candidate)? {
                        dir::Type::Region(region) => {
                            extents.push(region.extent);
                            spaces.push(region.space);
                        }
                        _ => extents.push(*candidate),
                    }
                }

                // join bare extents as one union, region pairs by extent and space
                if spaces.is_empty() {
                    return Ok(Some(self.normalized_union_type(extents)?));
                }
                if spaces.len() != extents.len() {
                    return Ok(None);
                }
                let extent = self.normalized_union_type(extents)?;
                let space = self.normalized_union_type(spaces)?;

                Ok(Some(self.intern_region(extent, space)?))
            }
            _ => Ok(None),
        }
    }

    /// Discharge one accumulated bound against a committed solution.
    fn discharge_bound(
        &mut self,
        side: BoundSide,
        bound: &Bound,
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
    ) -> CompilerResult<Verdict> {
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
    ) -> CompilerResult<Verdict> {
        self.push_variable_bound(variable, BoundSide::Upper, origin, cause, bound, relation)
    }

    /// Push one bound onto a variable side and schedule it against its paired bounds.
    fn push_variable_bound(
        &mut self,
        variable: dir::TypeVariableId,
        side: BoundSide,
        origin: Origin,
        cause: CauseId,
        bound: dir::GlobalTypeId,
        relation: Relation,
    ) -> CompilerResult<Verdict> {
        // drop a bare self-reference bound
        let variable = self.infer.alias_root(variable)?;
        if self.root_variable(bound)? == Some(variable) {
            return Ok(Verdict::Ambiguous);
        }

        // discharge a late bound as a relation check, a settled region awaiting the region solve
        if let Some(solution) = self.infer.variable(variable)?.state.ty() {
            if self.variable_memory_parameter(variable)? != Some(dir::MemoryParameter::Region) {
                let late = Bound::new(origin, bound, relation, cause);
                self.discharge_bound(side, &late, solution)?;
            }

            return Ok(Verdict::Ambiguous);
        }

        // refuse an upper bound no primitive of a numeric kind inhabits
        let kind = self.root_kind(variable)?;
        if side == BoundSide::Upper
            && kind.is_numeric()
            && self.root_variable(bound)?.is_none()
            && !self.numeric_bound_admits(origin, variable, bound)?
        {
            return Ok(Verdict::Fails);
        }

        // solve a memory slot outright at the closed term it equals
        if relation == Relation::Equal
            && matches!(kind, VariableKind::Memory(_))
            && self.root_variable(bound)?.is_none()
            && !self.type_contains_variable(bound, variable)?
        {
            self.commit_solution(variable, bound)?;

            return Ok(Verdict::Holds);
        }

        // merge two variables that bound one another or that meet numerically or by access
        if let Some(other) = self.root_variable(bound)? {
            let kinds = (self.root_kind(variable)?, self.root_kind(other)?);
            let is_numeric = (kinds.0.is_numeric() || kinds.1.is_numeric())
                && !matches!(
                    kinds,
                    (VariableKind::Memory(_), _) | (_, VariableKind::Memory(_))
                );
            let is_access = kinds
                == (
                    VariableKind::Memory(dir::MemoryParameter::Access),
                    VariableKind::Memory(dir::MemoryParameter::Access),
                )
                || (relation == Relation::Equal
                    && matches!(kinds, (VariableKind::Memory(first), VariableKind::Memory(second)) if first == second));
            let is_cycle = self.bounds_reach(other, side, variable)?
                || self.bounds_reach(variable, side.opposite(), other)?;
            if is_numeric || is_access || is_cycle {
                if self.alias_variable(variable, other)? == Verdict::Fails {
                    return Ok(Verdict::Fails);
                }
                if is_numeric || is_cycle {
                    self.settle_numeric_kind(variable, origin, cause)?;
                }

                return Ok(Verdict::Ambiguous);
            }
        }

        // collect each distinct bound once, recording a bound between variables from both ends
        let bound = Bound::new(origin, bound, relation, cause);
        if !self.infer.push_bound(variable, side, bound)? {
            return Ok(Verdict::Ambiguous);
        }
        if let Some(other) = self.root_variable(bound.ty)? {
            let reverse = Bound::new(origin, self.variable_type(variable)?, relation, cause);
            self.infer.push_bound(other, side.opposite(), reverse)?;
        }

        // trace the pushed bound
        self.record_event(match side {
            BoundSide::Lower => CheckEvent::LowerBoundPushed { variable, bound },
            BoundSide::Upper => CheckEvent::UpperBoundPushed { variable, bound },
        });
        if side == BoundSide::Lower {
            self.settle_numeric_kind(variable, origin, cause)?;
        }

        // propagate the new bound through every bound on the opposite side
        let opposite = self
            .infer
            .variables
            .side_bounds(variable, side.opposite())?
            .collect::<SmallVec<[_; 2]>>();
        let is_memory = self.variable_memory_parameter(variable)?.is_some();
        let mut verdict = Verdict::Ambiguous;
        for paired in opposite {
            let (lower, upper) = match side {
                BoundSide::Lower => (bound, paired),
                BoundSide::Upper => (paired, bound),
            };
            let relation = lower.relation.join(upper.relation);
            let id = self.push_relation(RelationCheck::new(
                upper.origin,
                relation,
                lower.ty,
                upper.ty,
                upper.cause,
            ))?;

            // report a failed declared bound separately
            if let Some(CheckOutcome::Fails(_)) = self.fulfill.checks.result(id)?
                && !is_memory
                && upper.relation != Relation::Subtype
            {
                verdict = Verdict::Fails;
            }
        }

        Ok(verdict)
    }
}
