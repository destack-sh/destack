use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::{IndexMap, IndexSet};
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::CheckState;

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

    /// Return inference variables referenced by this substitution's arguments.
    pub(in crate::check) fn variables(
        &self,
        check: &CheckState<'_>,
    ) -> CompilerResult<SmallVec<[dir::TypeVariableId; 4]>> {
        let mut variables = SmallVec::new();

        // collect variables in argument order from most recently inferred first
        for argument in self.arguments.iter().rev().copied() {
            for variable in check.type_variables(argument)? {
                if !variables.contains(&variable) {
                    variables.push(variable);
                }
            }
        }

        Ok(variables)
    }
}

/// Rule applied to matching leaves of one type graph.
#[derive(Debug, Clone, Copy)]
enum TypeRewrite<'a> {
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
    /// Replace solved variables with their solutions, keep open variables.
    ResolveVariables,
    /// Remove inference barriers after candidate inference has closed.
    EraseNoInfer,
}

impl TypeRewrite<'_> {
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
            Self::Replace { .. }
            | Self::SubstituteInfer { .. }
            | Self::ResolveVariables
            | Self::EraseNoInfer => None,
        }
    }

    /// Return the receiver replacing `this` references.
    fn receiver(&self) -> Option<dir::GlobalTypeId> {
        match self {
            Self::Substitute { receiver, .. } => *receiver,
            Self::Replace { .. }
            | Self::SubstituteInfer { .. }
            | Self::ResolveVariables
            | Self::EraseNoInfer => None,
        }
    }

    /// Return the captured type for one conditional-infer symbol.
    fn infer_capture(&self, symbol: dir::GlobalSymbolId) -> Option<dir::GlobalTypeId> {
        match self {
            Self::SubstituteInfer { captures } => captures
                .iter()
                .find(|capture| capture.symbol == symbol)
                .map(|capture| capture.ty),
            Self::Substitute { .. }
            | Self::Replace { .. }
            | Self::ResolveVariables
            | Self::EraseNoInfer => None,
        }
    }
}

impl CheckState<'_> {
    /// Collect the unsolved variables one type transitively references.
    pub(in crate::check) fn type_variables(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<SmallVec<[dir::TypeVariableId; 2]>> {
        let mut variables = SmallVec::new();
        let mut pending = SmallVec::<[dir::GlobalTypeId; 8]>::new();
        let mut visited = indexmap::IndexSet::new();
        pending.push(id);

        // scan the type graph without following symbol references;
        // solutions may be cyclic, so every id visits exactly once
        while let Some(id) = pending.pop() {
            if !visited.insert(id) {
                continue;
            }
            let ty = self.ty(id)?;

            // record open variables through their representative
            if let dir::Type::Variable(variable) = ty {
                let representative = self.solver.representative(variable)?;
                let state = self.solver.variable(representative)?;

                if state.solution.is_none() && !variables.contains(&representative) {
                    variables.push(representative);
                } else if let Some(solution) = state.solution {
                    pending.push(solution);
                }

                continue;
            }

            self.for_each_type_child(id.module_id, &ty, |child| pending.push(child))?;
        }

        Ok(variables)
    }

    /// Rewrite one type by replacing its leaves.
    /// Returns the same id when nothing changed.
    /// Source payload lists resolve in each rewritten type's own module;
    /// rebuilt composites intern into the writable target module.
    fn rewrite_type(
        &mut self,
        target: ModuleId,
        id: dir::GlobalTypeId,
        rewrite: TypeRewrite<'_>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // mark every rewritten path once, then rebuild along the marks
        let mut rewrites = IndexMap::new();
        self.mark_rewrites(id, rewrite, &mut rewrites)?;
        let mut rewriting = IndexSet::new();

        self.rewrite_type_guarded(target, id, rewrite, &rewrites, &mut rewriting)
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

        self.rewrite_type(
            target,
            id,
            TypeRewrite::Substitute {
                parameters: &substitution.parameters,
                arguments: &substitution.arguments,
                receiver: substitution.receiver,
            },
        )
    }

    /// Replace solved variables in one type.
    pub(in crate::check) fn resolve_type_variables(
        &mut self,
        target: ModuleId,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.rewrite_type(target, id, TypeRewrite::ResolveVariables)
    }

    /// Remove inference barriers after candidate inference has closed.
    pub(in crate::check) fn erase_inference_barriers(
        &mut self,
        target: ModuleId,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.rewrite_type(target, id, TypeRewrite::EraseNoInfer)
    }

    /// Return whether one type graph contains an inference barrier.
    pub(in crate::check) fn contains_inference_barrier(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let mut pending = SmallVec::<[dir::GlobalTypeId; 8]>::new();
        let mut visited = IndexSet::new();
        pending.push(id);

        // scan each reachable type once
        while let Some(id) = pending.pop() {
            let id = self.settled_root(id)?;
            if !visited.insert(id) {
                continue;
            }

            // stop when either operation or stdlib NoInfer appears
            let ty = self.ty(id)?;
            if matches!(ty, dir::Type::Operation(dir::TypeOperation::NoInfer(_))) {
                return Ok(true);
            }
            if let dir::Type::Instance(instance) = ty
                && self
                    .environment
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
        let rewrite = TypeRewrite::Replace { from, to };

        self.rewrite_type(target, id, rewrite)
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

        self.rewrite_type(target, id, TypeRewrite::SubstituteInfer { captures })
    }

    /// Mark whether each reachable id contains one rewritten leaf.
    /// Cyclic graphs mark conservatively unchanged on re-entry.
    fn mark_rewrites(
        &self,
        id: dir::GlobalTypeId,
        rewrite: TypeRewrite<'_>,
        rewrites: &mut IndexMap<dir::GlobalTypeId, bool>,
    ) -> CompilerResult<bool> {
        // replay marks and break cycles
        if let Some(known) = rewrites.get(&id) {
            return Ok(*known);
        }
        rewrites.insert(id, false);

        // leaves decide directly, composites inherit their children
        let ty = self.ty(id)?;
        let hit = match (ty, rewrite) {
            _ if matches!(rewrite, TypeRewrite::Replace { from, .. } if from == id) => true,
            (dir::Type::Instance(instance), _)
                if instance.arguments.is_empty()
                    && rewrite.infer_capture(instance.symbol).is_some() =>
            {
                true
            }
            (
                dir::Type::Operation(dir::TypeOperation::Infer(dir::InferType {
                    symbol: Some(symbol),
                    ..
                })),
                _,
            ) if rewrite.infer_capture(symbol).is_some() => true,
            (dir::Type::Parameter(parameter), TypeRewrite::Substitute { .. }) => {
                rewrite.substituted(parameter).is_some()
            }
            (dir::Type::This, TypeRewrite::Substitute { .. }) => rewrite.receiver().is_some(),
            (dir::Type::Variable(variable), TypeRewrite::ResolveVariables) => {
                self.solver.solution(variable)?.is_some()
            }
            (dir::Type::Variable(variable), _) => match self.solver.solution(variable)? {
                Some(solution) => self.mark_rewrites(solution, rewrite, rewrites)?,
                None => false,
            },
            (dir::Type::Operation(dir::TypeOperation::NoInfer(_)), TypeRewrite::EraseNoInfer) => {
                true
            }
            (dir::Type::Instance(instance), TypeRewrite::EraseNoInfer)
                if self
                    .environment
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
                    hit |= self.mark_rewrites(child, rewrite, rewrites)?;
                }

                hit
            }
        };
        rewrites.insert(id, hit);

        Ok(hit)
    }

    /// Rewrite one type with the active rewrite path tracked.
    fn rewrite_type_guarded(
        &mut self,
        target: ModuleId,
        id: dir::GlobalTypeId,
        rewrite: TypeRewrite<'_>,
        rewrites: &IndexMap<dir::GlobalTypeId, bool>,
        rewriting: &mut IndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // break rewrite cycles conservatively
        if !rewriting.insert(id) {
            return Ok(id);
        }
        let rewritten = destack_core::ensure_sufficient_stack(|| {
            self.rewrite_type_id(target, id, rewrite, rewrites, rewriting)
        });
        rewriting.swap_remove(&id);

        rewritten
    }

    /// Rewrite one type id after the cycle guard accepts it.
    fn rewrite_type_id(
        &mut self,
        target: ModuleId,
        id: dir::GlobalTypeId,
        rewrite: TypeRewrite<'_>,
        rewrites: &IndexMap<dir::GlobalTypeId, bool>,
        rewriting: &mut IndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // replace one matched type id
        if let TypeRewrite::Replace { from, to } = rewrite
            && id == from
        {
            return Ok(to);
        }

        // substitute one conditional-infer binder reference
        if let dir::Type::Instance(instance) = self.ty(id)?
            && instance.arguments.is_empty()
            && let Some(replacement) = rewrite.infer_capture(instance.symbol)
        {
            return Ok(replacement);
        }

        // substitute one direct conditional-infer binder
        if let dir::Type::Operation(dir::TypeOperation::Infer(dir::InferType {
            symbol: Some(symbol),
            ..
        })) = self.ty(id)?
            && let Some(replacement) = rewrite.infer_capture(symbol)
        {
            return Ok(replacement);
        }

        // substitute one generic parameter reference
        if let dir::Type::Parameter(parameter) = self.ty(id)? {
            if let Some(replacement) = rewrite.substituted(parameter) {
                return Ok(replacement);
            }
            if matches!(rewrite, TypeRewrite::Substitute { .. }) {
                return Ok(id);
            }
        }

        // substitute one qualified receiver reference
        if matches!(self.ty(id)?, dir::Type::This)
            && let Some(receiver) = rewrite.receiver()
        {
            return Ok(receiver);
        }

        // erase one inference barrier after the owning signature closes inference
        if matches!(rewrite, TypeRewrite::EraseNoInfer) {
            let unwrapped = match self.ty(id)? {
                dir::Type::Operation(dir::TypeOperation::NoInfer(unary)) => Some(unary.target),
                dir::Type::Instance(instance)
                    if self
                        .environment
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
                return self.rewrite_type_guarded(target, unwrapped, rewrite, rewrites, rewriting);
            }
        }

        // resolve one solved variable through its representative
        let variable = match self.ty(id)? {
            dir::Type::Variable(variable) => Some(variable),
            _ => None,
        };
        if let Some(variable) = variable {
            let solution = self.solver.solution(variable)?;

            return match (solution, rewrite) {
                (Some(solution), _) => {
                    // solutions mark separately from their variable entries
                    let mut rewrites = IndexMap::new();
                    self.mark_rewrites(solution, rewrite, &mut rewrites)?;

                    self.rewrite_type_guarded(target, solution, rewrite, &rewrites, rewriting)
                }
                (None, _) => Ok(id),
            };
        }

        // skip rebuilds for types without rewritten leaves
        if !rewrites.get(&id).copied().unwrap_or(false) {
            return Ok(id);
        }

        // read payloads where the type lives, intern the rebuild where we work
        let ty = self.ty(id)?;
        let rewritten =
            self.rewrite_children(id.module_id, target, ty, rewrite, rewrites, rewriting)?;

        self.intern_type(target, rewritten)
    }

    /// Rebuild one type with rewritten children.
    /// The source module owns the value's payload lists; the target module
    /// owns the rebuilt type and its reinterned lists.
    fn rewrite_children(
        &mut self,
        source: ModuleId,
        target: ModuleId,
        ty: dir::Type,
        rewrite: TypeRewrite<'_>,
        rewrites: &IndexMap<dir::GlobalTypeId, bool>,
        rewriting: &mut IndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::Type> {
        self.map_type_children(source, target, ty, &mut |state, child| {
            state.rewrite_type_guarded(target, child, rewrite, rewrites, rewriting)
        })
    }
}
