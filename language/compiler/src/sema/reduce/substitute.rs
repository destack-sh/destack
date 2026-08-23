use std::hash::{Hash, Hasher};

use destack_core::{FxIndexMap, FxIndexSet, ensure_sufficient_stack};
use destack_dir as dir;
use destack_source::ModuleId;
use rustc_hash::FxHasher;
use smallvec::SmallVec;

use crate::sema::{CheckState, GenericParameterId, Origin};
use crate::{CompilerError, CompilerResult};

/// One generic type substitution.
#[derive(Debug, Clone, Default)]
pub(in crate::sema) struct TypeSubstitution {
    /// The applied generic arguments in declaration order.
    pub(in crate::sema) bindings: SmallVec<[dir::GenericArgumentBinding; 4]>,
    /// The qualified receiver replacing `this` references.
    pub(in crate::sema) receiver: Option<dir::GlobalTypeId>,
}

/// One conditional-infer branch substitution.
#[derive(Debug, Clone, Copy)]
pub(in crate::sema) struct InferSubstitution {
    /// The inferred binder symbol.
    pub(in crate::sema) symbol: dir::GlobalSymbolId,
    /// The captured type.
    pub(in crate::sema) ty: dir::GlobalTypeId,
}

impl TypeSubstitution {
    /// Return the argument bound to one parameter.
    pub(in crate::sema) fn argument(
        &self,
        parameter: dir::GlobalGenericParameterId,
    ) -> Option<dir::GlobalTypeId> {
        self.bindings
            .iter()
            .find(|binding| binding.parameter == parameter)
            .map(|binding| binding.argument)
    }

    /// Bind one parameter to one argument.
    pub(in crate::sema) fn bind(
        &mut self,
        parameter: dir::GlobalGenericParameterId,
        argument: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        // refuse duplicate parameter bindings
        if self
            .bindings
            .iter()
            .any(|binding| binding.parameter == parameter)
        {
            return Err(CompilerError::Internal {
                message: format!("generic parameter {parameter:?} was bound more than once"),
            });
        }

        // append the binding in declaration order
        self.bindings
            .push(dir::GenericArgumentBinding::new(parameter, argument));

        Ok(())
    }

    /// Return the applied arguments in binding order.
    pub(in crate::sema) fn arguments(&self) -> impl Iterator<Item = dir::GlobalTypeId> + '_ {
        self.bindings.iter().map(|binding| binding.argument)
    }

    /// Return whether this substitution replaces nothing.
    pub(in crate::sema) fn is_empty(&self) -> bool {
        self.bindings.is_empty() && self.receiver.is_none()
    }

    /// Return this substitution with a qualified receiver.
    pub(in crate::sema) fn with_receiver(mut self, receiver: dir::GlobalTypeId) -> Self {
        self.receiver = Some(receiver);
        self
    }

    /// Return this substitution extended by carried outer bindings.
    pub(in crate::sema) fn with_carried(
        mut self,
        carried: &[dir::GenericArgumentBinding],
    ) -> CompilerResult<Self> {
        // merge carried bindings without changing an existing selection
        for binding in carried {
            match self.argument(binding.parameter) {
                Some(argument) if argument == binding.argument => {}
                Some(argument) => {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "generic parameter {:?} has conflicting arguments {argument:?} and {:?}",
                            binding.parameter, binding.argument,
                        ),
                    });
                }
                None => self.bindings.push(*binding),
            }
        }

        Ok(self)
    }
}

/// Rule applied to matching leaves of one type graph.
#[derive(Debug, Clone, Copy)]
enum SubstitutionRule<'a> {
    /// Replace generic parameter and receiver references by binding.
    Substitute {
        /// The generic arguments and qualified receiver.
        substitution: &'a TypeSubstitution,
    },
    /// Rebuild every node so construction normalizes each head.
    Normalize {
        /// The declaration whose entries normalize.
        origin: Origin,
    },
    /// Replace one type id wherever it occurs.
    Replace {
        /// The replaced type id.
        from: dir::GlobalTypeId,
        /// The replacement type id.
        to: dir::GlobalTypeId,
    },
    /// Replace open roots and generic parameters with numbered holes and rigid types.
    Canonicalize {
        /// The canonical hole per open root.
        holes: &'a FxIndexMap<dir::TypeVariableId, dir::HoleIndex>,
        /// The canonical rigid number per generic parameter.
        parameters: &'a FxIndexMap<GenericParameterId, dir::RigidIndex>,
    },
    /// Replace numbered canonical holes and rigid types with live types.
    Instantiate {
        /// The live variable per canonical hole, in hole order.
        holes: &'a [dir::TypeVariableId],
        /// The live parameter type per canonical rigid number.
        parameters: &'a [dir::GlobalTypeId],
    },
    /// Replace conditional-infer binders with captured types.
    SubstituteInfer {
        /// The captured types keyed by binder symbol.
        captures: &'a [InferSubstitution],
        /// The origin closed rebuilt entries normalize under, absent for a branch that tails
        /// into another conditional and evaluates in place.
        origin: Option<Origin>,
    },
    /// Remove every inference barrier.
    EraseNoInfer,
}

impl SubstitutionRule<'_> {
    /// Return the substituted argument for one parameter.
    fn substituted(&self, parameter: dir::GlobalGenericParameterId) -> Option<dir::GlobalTypeId> {
        match self {
            Self::Substitute { substitution } => substitution
                .bindings
                .iter()
                .find(|binding| binding.parameter == parameter)
                .map(|binding| binding.argument),
            Self::Replace { .. }
            | Self::Canonicalize { .. }
            | Self::Instantiate { .. }
            | Self::SubstituteInfer { .. }
            | Self::EraseNoInfer
            | Self::Normalize { .. } => None,
        }
    }

    /// Return the receiver replacing `this` references.
    fn receiver(&self) -> Option<dir::GlobalTypeId> {
        match self {
            Self::Substitute { substitution } => substitution.receiver,
            Self::Replace { .. }
            | Self::Canonicalize { .. }
            | Self::Instantiate { .. }
            | Self::SubstituteInfer { .. }
            | Self::EraseNoInfer
            | Self::Normalize { .. } => None,
        }
    }

    /// Return the captured type for one conditional-infer symbol.
    fn infer_capture(&self, symbol: dir::GlobalSymbolId) -> Option<dir::GlobalTypeId> {
        match self {
            Self::SubstituteInfer { captures, .. } => captures
                .iter()
                .find(|capture| capture.symbol == symbol)
                .map(|capture| capture.ty),
            Self::Substitute { .. }
            | Self::Replace { .. }
            | Self::Canonicalize { .. }
            | Self::Instantiate { .. }
            | Self::EraseNoInfer
            | Self::Normalize { .. } => None,
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
        // flag every affected path once, then rebuild along the flags
        let mut affected = FxIndexMap::default();
        self.substitution_affects(id, rule, &mut affected)?;
        let mut substituting = FxIndexSet::default();

        self.substitute_guarded(target, id, rule, &affected, &mut substituting)
    }

    /// Return whether one substitution maps every parameter to itself.
    fn is_identity_substitution(
        &mut self,
        substitution: &TypeSubstitution,
    ) -> CompilerResult<bool> {
        for binding in &substitution.bindings {
            let argument = self.shallow_resolve(binding.argument)?;
            let is_self = matches!(
                self.ty(argument)?,
                dir::Type::Parameter(parameter) if parameter == binding.parameter
            );
            if !is_self {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Substitute generic parameters and `this` in one type.
    pub(in crate::sema) fn substitute_type(
        &mut self,
        id: dir::GlobalTypeId,
        substitution: &TypeSubstitution,
    ) -> CompilerResult<dir::GlobalTypeId> {
        if substitution.is_empty() {
            return Ok(id);
        }

        // skip closed types, which hold nothing a substitution rewrites
        let flags = self.type_flags(id)?;
        if !flags.has_parameter() && !flags.has_this() {
            return Ok(id);
        }

        // skip identity substitutions when no receiver rewrite applies
        if (substitution.receiver.is_none() || !flags.has_this())
            && self.is_identity_substitution(substitution)?
        {
            return Ok(id);
        }

        // reuse one decided substitution of a resolved graph, verifying the
        //  stored bindings behind the hashed key
        let mut hasher = FxHasher::default();
        substitution.bindings.hash(&mut hasher);
        let key = (id, substitution.receiver, hasher.finish());
        let is_closed = !flags.has_variable();
        if is_closed
            && let Some((bindings, substituted)) = self.substitutions.get(&key)
            && *bindings == substitution.bindings
        {
            return Ok(*substituted);
        }

        // substitute the graph and store what it decided
        let substituted = self.substitute_graph(
            self.module_id,
            id,
            SubstitutionRule::Substitute { substitution },
        )?;
        if is_closed {
            self.substitutions
                .insert(key, (substitution.bindings.clone(), substituted));
        }

        Ok(substituted)
    }

    /// Instantiate one interface type under a selected implementation.
    pub(in crate::sema) fn instantiate_interface_type(
        &mut self,
        id: dir::GlobalTypeId,
        implementation: dir::GlobalTypeId,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // derive the complete positional and receiver substitution
        let (base, _) = self.refinements(implementation)?;
        let dir::Type::Application(application) = self.ty(base)? else {
            return Err(CompilerError::Internal {
                message: format!("interface implementation {implementation:?} has no application"),
            });
        };
        let module = base.module_id;
        let substitution = self.qualified_instance_substitution(module, &application, receiver)?;

        // replace the implemented application by its refined implementation
        let id = self.substitute_type(id, &substitution)?;
        let id = if implementation == base {
            id
        } else {
            self.replace_type(self.module_id, id, base, implementation)?
        };

        Ok(id)
    }

    /// Evaluate one closed type graph, reducing every head that decides.
    pub(in crate::sema) fn evaluate_type(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.substitute_graph(self.module_id, id, SubstitutionRule::Normalize { origin })
    }

    /// Translate the declared types into semantic types, normal by construction.
    pub(in crate::sema) fn translate_declared_types(&mut self) -> CompilerResult<()> {
        // rebuild each declared symbol type over normalized heads
        let module = self.module_id;
        let symbol_types: Vec<_> = self
            .module
            .types
            .with_tail(&self.module.types_tail)
            .symbol_types()
            .filter(|(symbol, _)| symbol.module_id == module)
            .collect();
        for (symbol, ty) in symbol_types {
            let origin = Origin::Symbol(symbol);
            let normal =
                self.substitute_graph(module, ty, SubstitutionRule::Normalize { origin })?;
            if normal != ty {
                self.module.types_tail.set_symbol_type(symbol, normal);
            }
        }

        // rebuild each node's annotation over normalized heads
        let node_types: Vec<_> = self
            .module
            .types
            .with_tail(&self.module.types_tail)
            .node_types()
            .filter(|(node, _)| node.module_id == module)
            .collect();
        for (node, ty) in node_types {
            let origin = Origin::Node(node, None);
            let normal =
                self.substitute_graph(module, ty, SubstitutionRule::Normalize { origin })?;
            if normal != ty {
                self.module.types_tail.set_node_type(node, normal);
            }
        }

        // rebuild each node's expectation the same way
        let expected_types: Vec<_> = self
            .module
            .types_tail
            .expected_types()
            .filter(|(node, _)| node.module_id == module)
            .collect();
        for (node, ty) in expected_types {
            let origin = Origin::Node(node, None);
            let normal =
                self.substitute_graph(module, ty, SubstitutionRule::Normalize { origin })?;
            if normal != ty {
                self.module.types_tail.set_expected_type(node, normal);
            }
        }

        // rebuild each recorded member subject over normalized heads
        let subjects: Vec<_> = self.module.iter_member_subjects().collect();
        for (site, mut subject) in subjects {
            let origin = Origin::Node(site.node(), subject.scope);
            let receiver = self.substitute_graph(
                module,
                subject.receiver,
                SubstitutionRule::Normalize { origin },
            )?;
            let target = self.substitute_graph(
                module,
                subject.target,
                SubstitutionRule::Normalize { origin },
            )?;
            let key_type = self.substitute_graph(
                module,
                subject.key_type,
                SubstitutionRule::Normalize { origin },
            )?;
            if receiver != subject.receiver
                || target != subject.target
                || key_type != subject.key_type
            {
                subject.receiver = receiver;
                subject.target = target;
                subject.key_type = key_type;
                self.module.members_tail.commit_subject(site, subject);
            }
        }

        // rebuild each definition's stored roots the same way
        let symbols: Vec<_> = self
            .module
            .iter_definitions()
            .map(|(symbol, _)| symbol)
            .collect();
        for symbol in symbols {
            let Some(source) = self.module.definition_source_maybe(symbol) else {
                continue;
            };
            let Some(definition) = self.module.definition(symbol) else {
                continue;
            };
            let mut definition = definition.clone();
            self.translate_definition(module, Origin::Symbol(symbol), &mut definition)?;
            self.module
                .definitions_tail
                .insert_definition(symbol, source, definition);
        }

        Ok(())
    }

    /// Translate one declared definition's stored roots over normalized heads.
    fn translate_definition(
        &mut self,
        module: ModuleId,
        origin: Origin,
        definition: &mut dir::Definition,
    ) -> CompilerResult<()> {
        match definition {
            dir::Definition::TypeAlias(alias) => {
                self.translate_root(module, origin, &mut alias.value)?;
            }
            dir::Definition::Struct(nominal) => {
                for conformance in &mut nominal.implements {
                    self.translate_root(module, origin, &mut conformance.interface)?;
                }
                self.translate_members(module, origin, &mut nominal.members)?;
            }
            dir::Definition::Class(nominal) => {
                if let Some(heritage) = &mut nominal.extends {
                    self.translate_root(module, origin, &mut heritage.ty)?;
                }
                for conformance in &mut nominal.implements {
                    self.translate_root(module, origin, &mut conformance.interface)?;
                }
                for constructor in &mut nominal.constructors {
                    self.translate_root(module, origin, &mut constructor.ty)?;
                }
                self.translate_members(module, origin, &mut nominal.members)?;
            }
            dir::Definition::Interface(nominal) => {
                for heritage in &mut nominal.extends {
                    self.translate_root(module, origin, &mut heritage.ty)?;
                }
                self.translate_members(module, origin, &mut nominal.members)?;
            }
            dir::Definition::Enum(nominal) => {
                for conformance in &mut nominal.implements {
                    self.translate_root(module, origin, &mut conformance.interface)?;
                }
                self.translate_members(module, origin, &mut nominal.members)?;
            }
            dir::Definition::Newtype(nominal) => {
                self.translate_root(module, origin, &mut nominal.backing)?;
                for constructor in &mut nominal.constructors {
                    self.translate_root(module, origin, &mut constructor.backing)?;
                    self.translate_root(module, origin, &mut constructor.ty)?;
                }
                self.translate_members(module, origin, &mut nominal.members)?;
            }
            dir::Definition::Extension(extension) => {
                let (dir::ExtensionTarget::Rooted { ty, .. }
                | dir::ExtensionTarget::Blanket { ty, .. }) = &mut extension.target;
                self.translate_root(module, origin, ty)?;
                for conformance in &mut extension.implements {
                    self.translate_root(module, origin, &mut conformance.interface)?;
                }
                self.translate_members(module, origin, &mut extension.members)?;
            }
        }

        Ok(())
    }

    /// Translate the stored roots of one definition's members.
    fn translate_members(
        &mut self,
        module: ModuleId,
        origin: Origin,
        members: &mut [dir::DefinitionMember],
    ) -> CompilerResult<()> {
        for member in members {
            match member {
                dir::DefinitionMember::Field(_)
                | dir::DefinitionMember::Method(_)
                | dir::DefinitionMember::AssociatedConst(_)
                | dir::DefinitionMember::EnumVariant(_) => {}
                dir::DefinitionMember::AssociatedType(member) => {
                    if let Some(constraint) = &mut member.constraint {
                        self.translate_root(module, origin, constraint)?;
                    }
                    if let Some(value) = &mut member.value {
                        self.translate_root(module, origin, value)?;
                    }
                }
                dir::DefinitionMember::CallSignature(member)
                | dir::DefinitionMember::ConstructSignature(member) => {
                    self.translate_root(module, origin, &mut member.ty)?;
                }
                dir::DefinitionMember::IndexSignature(member) => {
                    self.translate_root(module, origin, &mut member.key_type)?;
                    self.translate_root(module, origin, &mut member.value_type)?;
                }
            }
        }

        Ok(())
    }

    /// Re-point one stored root at its normalized head.
    fn translate_root(
        &mut self,
        module: ModuleId,
        origin: Origin,
        id: &mut dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        *id = self.substitute_graph(module, *id, SubstitutionRule::Normalize { origin })?;

        Ok(())
    }

    /// Remove inference barriers after candidate inference has closed.
    pub(in crate::sema) fn erase_inference_barriers(
        &mut self,
        target: ModuleId,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.substitute_graph(target, id, SubstitutionRule::EraseNoInfer)
    }

    /// Remove the inference barriers of one contextual target whose variables closed.
    pub(in crate::sema) fn erase_inference_barriers_if_closed(
        &mut self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // erase a flagged graph only once its variables close
        if self.type_flags(id)?.has_variable() {
            if !self.type_variables(id)?.is_empty() {
                return Ok(id);
            }

            return self.erase_inference_barriers(id.module_id, id);
        }

        // reuse the decided erasure of a closed graph
        if let Some(erased) = self.erasures.get(&id) {
            return Ok(*erased);
        }

        // decide the erasure once
        let erased = self.erase_inference_barriers(id.module_id, id)?;
        self.erasures.insert(id, erased);

        Ok(erased)
    }

    /// Return the target directly wrapped by `NoInfer`.
    pub(in crate::sema) fn no_infer_target(
        &mut self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let id = self.shallow_resolve(id)?;
        if let Some(dir::TypeOperation::NoInfer(operation)) = self.operation_head(id)? {
            return Ok(Some(operation.target));
        }

        // recognize the stdlib application before it reduces to the operation
        let dir::Type::Application(application) = self.ty(id)? else {
            return Ok(None);
        };
        if self.language_item(application.symbol)? != Some(dir::LanguageItem::NoInfer) {
            return Ok(None);
        }
        let arguments = self.type_ids(id.module_id, application.arguments)?;
        let [target] = arguments else {
            return Err(CompilerError::Internal {
                message: format!(
                    "NoInfer application {:?} has {} arguments",
                    application.symbol,
                    arguments.len(),
                ),
            });
        };

        Ok(Some(*target))
    }

    /// Replace numbered canonical holes and rigid types with live types in one pass.
    pub(in crate::sema) fn instantiate_answer_type(
        &mut self,
        target: ModuleId,
        id: dir::GlobalTypeId,
        holes: &[dir::TypeVariableId],
        parameters: &[dir::GlobalTypeId],
    ) -> CompilerResult<dir::GlobalTypeId> {
        let rule = SubstitutionRule::Instantiate { holes, parameters };

        self.substitute_graph(target, id, rule)
    }

    /// Replace open variable roots with numbered canonical holes in one pass.
    pub(in crate::sema) fn canonicalize_type(
        &mut self,
        target: ModuleId,
        id: dir::GlobalTypeId,
        holes: &FxIndexMap<dir::TypeVariableId, dir::HoleIndex>,
        parameters: &FxIndexMap<GenericParameterId, dir::RigidIndex>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let rule = SubstitutionRule::Canonicalize { holes, parameters };

        self.substitute_graph(target, id, rule)
    }

    /// Replace one type id inside another type graph.
    pub(in crate::sema) fn replace_type(
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
    pub(in crate::sema) fn substitute_infer_captures(
        &mut self,
        origin: Option<Origin>,
        target: ModuleId,
        id: dir::GlobalTypeId,
        captures: &[InferSubstitution],
    ) -> CompilerResult<dir::GlobalTypeId> {
        if captures.is_empty() {
            return Ok(id);
        }

        self.substitute_graph(
            target,
            id,
            SubstitutionRule::SubstituteInfer { captures, origin },
        )
    }

    /// Return whether one reachable id contains a leaf the substitution replaces.
    fn substitution_affects(
        &mut self,
        id: dir::GlobalTypeId,
        rule: SubstitutionRule<'_>,
        affected: &mut FxIndexMap<dir::GlobalTypeId, bool>,
    ) -> CompilerResult<bool> {
        // reuse flagged ids and break cycles
        if let Some(known) = affected.get(&id) {
            return Ok(*known);
        }
        affected.insert(id, false);

        // decide leaves directly and inherit composites from their children
        let ty = self.ty_raw(id)?;
        let hit = match (ty, rule) {
            _ if matches!(rule, SubstitutionRule::Normalize { .. }) => true,
            _ if matches!(rule, SubstitutionRule::Replace { from, .. } if from == id) => true,
            (dir::Type::Application(instance), _)
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
            (dir::Type::Variable(variable), _) => match self.infer.solution(variable)? {
                Some(solution) => self.substitution_affects(solution, rule, affected)?,
                None => match rule {
                    SubstitutionRule::Canonicalize { holes, .. } => {
                        holes.contains_key(&self.infer.alias_root(variable)?)
                    }
                    _ => false,
                },
            },
            (
                dir::Type::Parameter(parameter),
                SubstitutionRule::Canonicalize { parameters, .. },
            ) => parameters.contains_key(&parameter),
            (dir::Type::Hole(hole), SubstitutionRule::Instantiate { holes, .. }) => {
                (hole.0 as usize) < holes.len()
            }
            (dir::Type::Rigid(rigid), SubstitutionRule::Instantiate { parameters, .. }) => {
                (rigid.0 as usize) < parameters.len()
            }
            (_, SubstitutionRule::EraseNoInfer) if self.no_infer_target(id)?.is_some() => true,
            _ => {
                let mut hit = false;
                let mut children = SmallVec::<[dir::GlobalTypeId; 8]>::new();
                self.for_each_type_child(id.module_id, &ty, |child| children.push(child))?;
                for child in children {
                    hit |= self.substitution_affects(child, rule, affected)?;
                }

                hit
            }
        };
        affected.insert(id, hit);

        Ok(hit)
    }

    /// Substitute one type with the active path tracked.
    fn substitute_guarded(
        &mut self,
        target: ModuleId,
        id: dir::GlobalTypeId,
        rule: SubstitutionRule<'_>,
        affected: &FxIndexMap<dir::GlobalTypeId, bool>,
        substituting: &mut FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // break substitution cycles conservatively
        if !substituting.insert(id) {
            return Ok(id);
        }
        let substituted = ensure_sufficient_stack(|| {
            self.substitute_id(target, id, rule, affected, substituting)
        });
        substituting.swap_remove(&id);

        substituted
    }

    /// Substitute one type id once the cycle guard passes it.
    fn substitute_id(
        &mut self,
        target: ModuleId,
        id: dir::GlobalTypeId,
        rule: SubstitutionRule<'_>,
        affected: &FxIndexMap<dir::GlobalTypeId, bool>,
        substituting: &mut FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // replace one matched type id
        if let SubstitutionRule::Replace { from, to } = rule
            && id == from
        {
            return Ok(to);
        }

        // resolve one solved variable through its root
        let variable = match self.ty_raw(id)? {
            dir::Type::Variable(variable) => Some(variable),
            _ => None,
        };
        if let Some(variable) = variable {
            return match self.infer.solution(variable)? {
                // substitute through the solution, which flags apart from its variable entry
                Some(solution) => {
                    let mut affected = FxIndexMap::default();
                    self.substitution_affects(solution, rule, &mut affected)?;

                    self.substitute_guarded(target, solution, rule, &affected, substituting)
                }
                // rename one open root to its numbered canonical hole
                None => match rule {
                    SubstitutionRule::Canonicalize { holes, .. } => {
                        match holes.get(&self.infer.alias_root(variable)?) {
                            Some(hole) => self.intern_type(dir::Type::Hole(*hole)),
                            None => Ok(id),
                        }
                    }
                    _ => Ok(id),
                },
            };
        }

        // substitute one conditional-infer binder reference
        if let dir::Type::Application(instance) = self.ty(id)?
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

        // erase one selected inference barrier
        if matches!(rule, SubstitutionRule::EraseNoInfer)
            && let Some(target_id) = self.no_infer_target(id)?
        {
            return self.substitute_guarded(target, target_id, rule, affected, substituting);
        }

        // preserve every graph without a requested substitution
        if !affected.get(&id).copied().unwrap_or(false) {
            return Ok(id);
        }

        // reopen one canonical hole or rigid type at its live counterpart,
        //  keeping rigid numbers past the live parameters raw
        if let SubstitutionRule::Instantiate { holes, parameters } = rule {
            match self.ty(id)? {
                dir::Type::Hole(hole) => {
                    let Some(variable) = holes.get(hole.0 as usize) else {
                        return Err(CompilerError::Internal {
                            message: format!("canonical hole {hole} has no live counterpart"),
                        });
                    };

                    return self.intern_type(dir::Type::Variable(*variable));
                }
                dir::Type::Rigid(rigid) => {
                    if let Some(parameter) = parameters.get(rigid.0 as usize) {
                        return Ok(*parameter);
                    }
                }
                _ => {}
            }
        }

        // rename one generic parameter to its numbered canonical rigid type
        if let dir::Type::Parameter(parameter) = self.ty(id)?
            && let SubstitutionRule::Canonicalize { parameters, .. } = rule
            && let Some(rigid) = parameters.get(&parameter)
        {
            return self.intern_type(dir::Type::Rigid(*rigid));
        }

        // read type records from their owner and intern the result in the target module
        let ty = self.ty(id)?;
        let substituted =
            self.substitute_children(id.module_id, target, ty, rule, affected, substituting)?;
        let rebuilt = match substituted {
            dir::Type::Union(union) => {
                let elements: SmallVec<[_; 8]> = self.type_ids(target, union.elements)?.into();

                self.normalized_union_type(elements)
            }
            dir::Type::Intersection(intersection) => {
                let elements: SmallVec<[_; 8]> =
                    self.type_ids(target, intersection.elements)?.into();

                self.normalized_intersection_type(elements)
            }
            substituted => self.intern_type(substituted),
        }?;

        // normalize rebuilt entries that hold no parameter, this, or variable
        if let SubstitutionRule::Normalize { origin }
        | SubstitutionRule::SubstituteInfer {
            origin: Some(origin),
            ..
        } = rule
        {
            let flags = self.type_flags(rebuilt)?;
            if !flags.has_parameter() && !flags.has_this() && !flags.has_variable() {
                return self.normalize(origin, rebuilt);
            }
        }

        Ok(rebuilt)
    }

    /// Rebuild one type with substituted children.
    fn substitute_children(
        &mut self,
        source: ModuleId,
        target: ModuleId,
        ty: dir::Type,
        rule: SubstitutionRule<'_>,
        affected: &FxIndexMap<dir::GlobalTypeId, bool>,
        substituting: &mut FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::Type> {
        self.map_type_children(source, target, ty, &mut |state, child| {
            state.substitute_guarded(target, child, rule, affected, substituting)
        })
    }
}
