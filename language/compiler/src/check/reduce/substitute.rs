use destack_core::{FxIndexMap, FxIndexSet, ensure_sufficient_stack};
use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Cause, CauseKind, CheckState, GenericParameterId, GenericTemplateId, Origin, VariableRole,
    Widening,
};

/// One positional generic substitution.
#[derive(Debug, Clone, Default)]
pub(in crate::check) struct TypeSubstitution {
    /// The declared parameters in declaration order.
    pub(in crate::check) parameters: SmallVec<[dir::GlobalGenericParameterId; 4]>,
    /// The applied arguments in declaration order.
    pub(in crate::check) arguments: SmallVec<[dir::GlobalTypeId; 4]>,
    /// The qualified receiver replacing `this` references.
    pub(in crate::check) receiver: Option<dir::GlobalTypeId>,
}

/// One conditional-infer branch substitution.
#[derive(Debug, Clone, Copy)]
pub(in crate::check) struct InferSubstitution {
    /// The inferred binder symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The captured type.
    pub(in crate::check) ty: dir::GlobalTypeId,
}

impl TypeSubstitution {
    /// Return whether this substitution replaces nothing.
    pub(in crate::check) fn is_empty(&self) -> bool {
        self.parameters.is_empty() && self.receiver.is_none()
    }

    /// Return this substitution with a qualified receiver.
    pub(in crate::check) fn with_receiver(mut self, receiver: dir::GlobalTypeId) -> Self {
        self.receiver = Some(receiver);
        self
    }

    /// Return this substitution extended by carried outer bindings.
    pub(in crate::check) fn with_carried(&self, carried: &[dir::GenericArgumentBinding]) -> Self {
        let mut composed = self.clone();
        for binding in carried {
            if !composed.parameters.contains(&binding.parameter) {
                composed.parameters.push(binding.parameter);
                composed.arguments.push(binding.argument);
            }
        }

        composed
    }
}

impl CheckState<'_> {
    /// Substitute written annotation arguments into one template.
    pub(in crate::check) fn substitute_annotation_arguments(
        &mut self,
        origin: Origin,
        template: GenericTemplateId,
        written: &[dir::GlobalTypeId],
    ) -> CompilerResult<Option<TypeSubstitution>> {
        let parameters = self.generic_template_parameters(template);

        self.substitute_parameter_arguments(
            origin,
            &parameters,
            written,
            TypeSubstitution::default(),
        )
    }

    /// Substitute written annotation arguments into one parameter list.
    pub(in crate::check) fn substitute_parameter_arguments(
        &mut self,
        origin: Origin,
        parameters: &[GenericParameterId],
        written: &[dir::GlobalTypeId],
        mut substitution: TypeSubstitution,
    ) -> CompilerResult<Option<TypeSubstitution>> {
        if written.len() > self.written_parameter_count(parameters) {
            return Ok(None);
        }

        // bind explicit parameters and fill omitted defaults
        let mut cursor = 0;
        for parameter in parameters.iter().copied() {
            let Some(binding) = self.generic_parameter(parameter).copied() else {
                return Ok(None);
            };
            let is_explicit = matches!(binding.origin, dir::GenericParameterOrigin::Explicit);
            if is_explicit && cursor < written.len() {
                substitution.parameters.push(parameter);
                substitution.arguments.push(written[cursor]);
                cursor += 1;

                continue;
            }

            // evaluate defaults against the application built so far
            let default = binding
                .default
                .map(|default| self.substitute_type(origin.module(), default, &substitution))
                .transpose()?;
            if let Some(default) = default {
                substitution.parameters.push(parameter);
                substitution.arguments.push(default);

                continue;
            }

            if is_explicit {
                return Ok(None);
            }
        }

        Ok(Some(substitution))
    }

    /// Instantiate one parameter list, opening omitted explicit parameters.
    pub(in crate::check) fn instantiate_parameter_arguments(
        &mut self,
        origin: Origin,
        parameters: &[GenericParameterId],
        written: &[dir::GlobalTypeId],
        mut substitution: TypeSubstitution,
    ) -> CompilerResult<Option<TypeSubstitution>> {
        if written.len() > self.written_parameter_count(parameters) {
            return Ok(None);
        }

        // bind written parameters and open omitted inference parameters
        let mut cursor = 0;
        for parameter in parameters.iter().copied() {
            let Some(binding) = self.generic_parameter(parameter).copied() else {
                return Ok(None);
            };
            let is_explicit = matches!(binding.origin, dir::GenericParameterOrigin::Explicit);
            if is_explicit && cursor < written.len() {
                substitution.parameters.push(parameter);
                substitution.arguments.push(written[cursor]);
                cursor += 1;

                continue;
            }

            // open every omitted parameter while only explicit parameters consume source arguments
            let primitive_constraint = match binding.constraint {
                Some(constraint) => matches!(
                    self.ty(self.settled_root(constraint)?)?,
                    dir::Type::Primitive(_)
                ),
                None => false,
            };
            let widening = match binding.is_const
                || primitive_constraint
                || binding.memory_parameter().is_some()
            {
                true => Widening::Never,
                false => Widening::WhenWritten,
            };
            let variable =
                self.allocate_variable(origin, widening, VariableRole::Instantiation { parameter });

            // declared defaults complete the parameter when inference stays dry
            if let Some(default) = binding.default {
                let default = self.substitute_type(origin.module(), default, &substitution)?;
                self.set_variable_default(variable, default)?;
            }

            let argument = self.variable_type(variable)?;
            substitution.parameters.push(parameter);
            substitution.arguments.push(argument);

            // declared bounds substitute self-references and discharge on solutions
            if binding.memory_parameter().is_none()
                && let Some(bound) = binding.constraint
            {
                let bound = self.substitute_type(origin.module(), bound, &substitution)?;
                let cause = self.intern_cause(Cause::root(origin, CauseKind::Bound { parameter }));
                self.set_variable_parameter_bound(variable, bound, cause)?;
            }
        }

        Ok(Some(substitution))
    }

    /// Return how many parameters accept written arguments.
    pub(in crate::check) fn written_parameter_count(
        &self,
        parameters: &[GenericParameterId],
    ) -> usize {
        parameters
            .iter()
            .filter(|parameter| {
                self.generic_parameter(**parameter).is_some_and(|binding| {
                    matches!(binding.origin, dir::GenericParameterOrigin::Explicit)
                })
            })
            .count()
    }
}

/// Rule applied to matching leaves of one type graph.
#[derive(Debug, Clone, Copy)]
enum SubstitutionRule<'a> {
    /// Replace generic parameter and receiver references by position.
    Substitute {
        /// The declared parameters in declaration order.
        parameters: &'a [dir::GlobalGenericParameterId],
        /// The applied arguments in declaration order.
        arguments: &'a [dir::GlobalTypeId],
        /// The qualified receiver replacing `this` references.
        receiver: Option<dir::GlobalTypeId>,
    },
    /// Replace one type id wherever it occurs.
    Replace {
        /// The replaced type id.
        from: dir::GlobalTypeId,
        /// The replacement type id.
        to: dir::GlobalTypeId,
    },
    /// Replace conditional-infer binders with captured types.
    SubstituteInfer {
        /// The captured types keyed by binder symbol.
        captures: &'a [InferSubstitution],
    },
    /// Remove inference barriers after candidate inference has closed.
    EraseNoInfer,
}

impl SubstitutionRule<'_> {
    /// Return the substituted argument for one parameter.
    fn substituted(&self, parameter: dir::GlobalGenericParameterId) -> Option<dir::GlobalTypeId> {
        match self {
            Self::Substitute {
                parameters,
                arguments,
                ..
            } => parameters
                .iter()
                .position(|candidate| *candidate == parameter)
                .and_then(|position| arguments.get(position))
                .copied(),
            Self::Replace { .. } | Self::SubstituteInfer { .. } | Self::EraseNoInfer => None,
        }
    }

    /// Return the receiver replacing `this` references.
    fn receiver(&self) -> Option<dir::GlobalTypeId> {
        match self {
            Self::Substitute { receiver, .. } => *receiver,
            Self::Replace { .. } | Self::SubstituteInfer { .. } | Self::EraseNoInfer => None,
        }
    }

    /// Return the captured type for one conditional-infer symbol.
    fn infer_capture(&self, symbol: dir::GlobalSymbolId) -> Option<dir::GlobalTypeId> {
        match self {
            Self::SubstituteInfer { captures } => captures
                .iter()
                .find(|capture| capture.symbol == symbol)
                .map(|capture| capture.ty),
            Self::Substitute { .. } | Self::Replace { .. } | Self::EraseNoInfer => None,
        }
    }
}

impl CheckState<'_> {
    /// Substitute one type graph by replacing matching leaves.
    fn substitute_graph(
        &mut self,
        target: ModuleId,
        id: dir::GlobalTypeId,
        rule: SubstitutionRule<'_>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // mark every affected path once, then rebuild along the marks
        let mut marks = FxIndexMap::default();
        self.mark_substitutions(id, rule, &mut marks)?;
        let mut substituting = FxIndexSet::default();

        self.substitute_guarded(target, id, rule, &marks, &mut substituting)
    }

    /// Substitute generic parameters and `this` in one type.
    pub(in crate::check) fn substitute_type(
        &mut self,
        target: ModuleId,
        id: dir::GlobalTypeId,
        substitution: &TypeSubstitution,
    ) -> CompilerResult<dir::GlobalTypeId> {
        if substitution.is_empty() {
            return Ok(id);
        }

        self.substitute_graph(
            target,
            id,
            SubstitutionRule::Substitute {
                parameters: &substitution.parameters,
                arguments: &substitution.arguments,
                receiver: substitution.receiver,
            },
        )
    }

    /// Remove inference barriers after candidate inference has closed.
    pub(in crate::check) fn erase_inference_barriers(
        &mut self,
        target: ModuleId,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.substitute_graph(target, id, SubstitutionRule::EraseNoInfer)
    }

    /// Return whether one type graph contains an inference barrier.
    pub(in crate::check) fn contains_inference_barrier(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let mut pending = SmallVec::<[dir::GlobalTypeId; 8]>::new();
        let mut visited = FxIndexSet::default();
        pending.push(id);

        // scan each reachable type once
        while let Some(id) = pending.pop() {
            let id = self.settled_root(id)?;
            if !visited.insert(id) {
                continue;
            }

            // stop when either operation or stdlib NoInfer appears
            let ty = self.ty(id)?;
            if matches!(
                self.operation_head(id)?,
                Some(dir::TypeOperation::NoInfer(_))
            ) {
                return Ok(true);
            }
            if let dir::Type::Instance(instance) = ty
                && self
                    .global
                    .language
                    .item(instance.symbol)
                    .is_some_and(|item| item == dir::LanguageItem::NoInfer)
            {
                return Ok(true);
            }

            self.for_each_type_child(id.module_id, &ty, |child| pending.push(child))?;
        }

        Ok(false)
    }

    /// Replace one type id inside another type graph.
    pub(in crate::check) fn replace_type(
        &mut self,
        target: ModuleId,
        id: dir::GlobalTypeId,
        from: dir::GlobalTypeId,
        to: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let rule = SubstitutionRule::Replace { from, to };

        self.substitute_graph(target, id, rule)
    }

    /// Substitute conditional-infer captures inside one branch type.
    pub(in crate::check) fn substitute_infer_captures(
        &mut self,
        target: ModuleId,
        id: dir::GlobalTypeId,
        captures: &[InferSubstitution],
    ) -> CompilerResult<dir::GlobalTypeId> {
        if captures.is_empty() {
            return Ok(id);
        }

        self.substitute_graph(target, id, SubstitutionRule::SubstituteInfer { captures })
    }

    /// Mark whether each reachable id contains an affected leaf.
    fn mark_substitutions(
        &self,
        id: dir::GlobalTypeId,
        rule: SubstitutionRule<'_>,
        marks: &mut FxIndexMap<dir::GlobalTypeId, bool>,
    ) -> CompilerResult<bool> {
        // replay marks and break cycles
        if let Some(known) = marks.get(&id) {
            return Ok(*known);
        }
        marks.insert(id, false);

        // leaves decide directly, composites inherit their children
        let ty = self.ty(id)?;
        let hit = match (ty, rule) {
            _ if matches!(rule, SubstitutionRule::Replace { from, .. } if from == id) => true,
            (dir::Type::Instance(instance), _)
                if instance.arguments.is_empty()
                    && rule.infer_capture(instance.symbol).is_some() =>
            {
                true
            }
            (dir::Type::Operation(operation), _)
                if let dir::TypeOperation::Infer(dir::InferType {
                    symbol: Some(symbol),
                    ..
                }) = self.type_operation(id.module_id, operation)?
                    && rule.infer_capture(symbol).is_some() =>
            {
                true
            }
            (dir::Type::Parameter(parameter), SubstitutionRule::Substitute { .. }) => {
                rule.substituted(parameter).is_some()
            }
            (dir::Type::This, SubstitutionRule::Substitute { .. }) => rule.receiver().is_some(),
            (dir::Type::Variable(variable), _) => match self.solver.solution(variable)? {
                Some(solution) => self.mark_substitutions(solution, rule, marks)?,
                None => false,
            },
            (dir::Type::Operation(operation), SubstitutionRule::EraseNoInfer)
                if matches!(
                    self.type_operation(id.module_id, operation)?,
                    dir::TypeOperation::NoInfer(_)
                ) =>
            {
                true
            }
            (dir::Type::Instance(instance), SubstitutionRule::EraseNoInfer)
                if self
                    .global
                    .language
                    .item(instance.symbol)
                    .is_some_and(|item| item == dir::LanguageItem::NoInfer) =>
            {
                true
            }
            _ => {
                let mut children = SmallVec::<[dir::GlobalTypeId; 8]>::new();
                self.for_each_type_child(id.module_id, &ty, |child| children.push(child))?;
                let mut hit = false;
                for child in children {
                    hit |= self.mark_substitutions(child, rule, marks)?;
                }

                hit
            }
        };
        marks.insert(id, hit);

        Ok(hit)
    }

    /// Substitute one type with the active path tracked.
    fn substitute_guarded(
        &mut self,
        target: ModuleId,
        id: dir::GlobalTypeId,
        rule: SubstitutionRule<'_>,
        marks: &FxIndexMap<dir::GlobalTypeId, bool>,
        substituting: &mut FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // break substitution cycles conservatively
        if !substituting.insert(id) {
            return Ok(id);
        }
        let substituted =
            ensure_sufficient_stack(|| self.substitute_id(target, id, rule, marks, substituting));
        substituting.swap_remove(&id);

        substituted
    }

    /// Substitute one type id after the cycle guard accepts it.
    fn substitute_id(
        &mut self,
        target: ModuleId,
        id: dir::GlobalTypeId,
        rule: SubstitutionRule<'_>,
        marks: &FxIndexMap<dir::GlobalTypeId, bool>,
        substituting: &mut FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // replace one matched type id
        if let SubstitutionRule::Replace { from, to } = rule
            && id == from
        {
            return Ok(to);
        }

        // substitute one conditional-infer binder reference
        if let dir::Type::Instance(instance) = self.ty(id)?
            && instance.arguments.is_empty()
            && let Some(replacement) = rule.infer_capture(instance.symbol)
        {
            return Ok(replacement);
        }

        // substitute one direct conditional-infer binder
        if let Some(dir::TypeOperation::Infer(dir::InferType {
            symbol: Some(symbol),
            ..
        })) = self.operation_head(id)?
            && let Some(replacement) = rule.infer_capture(symbol)
        {
            return Ok(replacement);
        }

        // substitute one generic parameter reference
        if let dir::Type::Parameter(parameter) = self.ty(id)? {
            if let Some(replacement) = rule.substituted(parameter) {
                return Ok(replacement);
            }
            if matches!(rule, SubstitutionRule::Substitute { .. }) {
                return Ok(id);
            }
        }

        // substitute one qualified receiver reference
        if matches!(self.ty(id)?, dir::Type::This)
            && let Some(receiver) = rule.receiver()
        {
            return Ok(receiver);
        }

        // erase one inference barrier after the owning signature closes inference
        if matches!(rule, SubstitutionRule::EraseNoInfer) {
            let unwrapped = match self.ty(id)? {
                dir::Type::Operation(operation)
                    if let dir::TypeOperation::NoInfer(unary) =
                        self.type_operation(id.module_id, operation)? =>
                {
                    Some(unary.target)
                }
                dir::Type::Instance(instance)
                    if self
                        .global
                        .language
                        .item(instance.symbol)
                        .is_some_and(|item| item == dir::LanguageItem::NoInfer) =>
                {
                    match self.type_ids(id.module_id, instance.arguments)? {
                        [unwrapped] => Some(*unwrapped),
                        _ => None,
                    }
                }
                _ => None,
            };
            if let Some(unwrapped) = unwrapped {
                return self.substitute_guarded(target, unwrapped, rule, marks, substituting);
            }
        }

        // resolve one solved variable through its representative
        let variable = match self.ty(id)? {
            dir::Type::Variable(variable) => Some(variable),
            _ => None,
        };
        if let Some(variable) = variable {
            return match self.solver.solution(variable)? {
                Some(solution) => {
                    // solutions mark separately from their variable entries
                    let mut marks = FxIndexMap::default();
                    self.mark_substitutions(solution, rule, &mut marks)?;

                    self.substitute_guarded(target, solution, rule, &marks, substituting)
                }
                None => Ok(id),
            };
        }

        // skip rebuilds for types without affected leaves
        if !marks.get(&id).copied().unwrap_or(false) {
            return Ok(id);
        }

        // read payloads where the type lives, intern the rebuild where we work
        let ty = self.ty(id)?;
        let substituted =
            self.substitute_children(id.module_id, target, ty, rule, marks, substituting)?;

        self.intern_type(target, substituted)
    }

    /// Rebuild one type with substituted children.
    fn substitute_children(
        &mut self,
        source: ModuleId,
        target: ModuleId,
        ty: dir::Type,
        rule: SubstitutionRule<'_>,
        marks: &FxIndexMap<dir::GlobalTypeId, bool>,
        substituting: &mut FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::Type> {
        self.map_type_children(source, target, ty, &mut |state, child| {
            state.substitute_guarded(target, child, rule, marks, substituting)
        })
    }
}
