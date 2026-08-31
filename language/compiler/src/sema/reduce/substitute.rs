use std::hash::{Hash, Hasher};

use destack_core::{FxIndexMap, FxIndexSet, ensure_sufficient_stack};
use destack_dir as dir;
use destack_source::ModuleId;
use rustc_hash::FxHasher;
use smallvec::SmallVec;

use crate::sema::{CheckState, Origin};
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
                message: format!("a generic parameter {parameter:?} bound more than once"),
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

    /// Return whether this substitution is empty.
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
    /// Replace conditional-infer binders with captured types.
    SubstituteInfer {
        /// The captured types keyed by binder symbol.
        captures: &'a [InferSubstitution],
        /// The origin closed rebuilt entries normalize under.
        ///
        /// A branch that tails into another conditional evaluates in place and leaves this open.
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

        // rebuild the requested root under every normalize pass
        if matches!(rule, SubstitutionRule::Normalize { .. }) {
            affected.insert(id, true);
        }

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

        // reuse one decided substitution of closed inputs
        let mut hasher = FxHasher::default();
        substitution.bindings.hash(&mut hasher);
        let key = (id, substitution.receiver, hasher.finish());
        let is_closed = !flags.has_variable() && self.is_closed_substitution(substitution)?;
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

    /// Return whether one substitution binds closed types only.
    fn is_closed_substitution(&self, substitution: &TypeSubstitution) -> CompilerResult<bool> {
        if let Some(receiver) = substitution.receiver
            && self.type_flags(receiver)?.has_variable()
        {
            return Ok(false);
        }
        for binding in &substitution.bindings {
            if self.type_flags(binding.argument)?.has_variable() {
                return Ok(false);
            }
        }

        Ok(true)
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
                message: format!(
                    "an interface implementation {implementation:?} without an application"
                ),
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

        // rebuild each recorded member subject over normalized heads
        let subjects: Vec<_> = self.module.iter_member_subjects().collect();
        for (site, mut subject, key) in subjects {
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
                subject.key_source,
                SubstitutionRule::Normalize { origin },
            )?;

            // commit the subject only when a head actually moved
            if receiver != subject.receiver
                || target != subject.target
                || key_type != subject.key_source
            {
                subject.receiver = receiver;
                subject.target = target;
                subject.key_source = key_type;
                self.module.members_tail.commit_subject(site, subject, key);
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

            // re-point the definition's roots and store it back
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
        // translate the types each definition holds
        match definition {
            // translate an alias's value
            dir::Definition::TypeAlias(alias) => {
                self.translate_root(module, origin, &mut alias.value)?;
            }
            // translate a struct's conformances and members
            dir::Definition::Struct(nominal) => {
                for conformance in &mut nominal.implements {
                    self.translate_root(module, origin, &mut conformance.interface)?;
                }

                self.translate_members(module, origin, &mut nominal.members)?;
            }
            // translate a class's base, conformances, constructors, and members
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
            // translate an interface's bases and members
            dir::Definition::Interface(nominal) => {
                for heritage in &mut nominal.extends {
                    self.translate_root(module, origin, &mut heritage.ty)?;
                }

                self.translate_members(module, origin, &mut nominal.members)?;
            }
            // translate an enum's conformances and members
            dir::Definition::Enum(nominal) => {
                for conformance in &mut nominal.implements {
                    self.translate_root(module, origin, &mut conformance.interface)?;
                }

                self.translate_members(module, origin, &mut nominal.members)?;
            }
            // translate a newtype's backing, constructors, and members
            dir::Definition::Newtype(nominal) => {
                self.translate_root(module, origin, &mut nominal.backing)?;

                for constructor in &mut nominal.constructors {
                    self.translate_root(module, origin, &mut constructor.backing)?;
                    self.translate_root(module, origin, &mut constructor.ty)?;
                }

                self.translate_members(module, origin, &mut nominal.members)?;
            }
            // translate an extension's target, conformances, and members
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
                // skip the members whose types live on their own symbols
                dir::DefinitionMember::Field(_)
                | dir::DefinitionMember::Method(_)
                | dir::DefinitionMember::AssociatedConst(_)
                | dir::DefinitionMember::EnumVariant(_) => {}
                // translate an associated type's constraint and value
                dir::DefinitionMember::AssociatedType(member) => {
                    if let Some(constraint) = &mut member.constraint {
                        self.translate_root(module, origin, constraint)?;
                    }

                    if let Some(value) = &mut member.value {
                        self.translate_root(module, origin, value)?;
                    }
                }
                // translate a call or construct signature's type
                dir::DefinitionMember::CallSignature(member)
                | dir::DefinitionMember::ConstructSignature(member) => {
                    self.translate_root(module, origin, &mut member.ty)?;
                }
                // translate an index signature's key and value types
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

        // require a NoInfer application
        if self.language_item(application.symbol)? != Some(dir::LanguageItem::NoInfer) {
            return Ok(None);
        }

        // require the barrier to carry exactly one argument
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

    /// Return whether one type applies the replaced declaration with equal arguments.
    fn replaces_application(
        &self,
        from: dir::GlobalTypeId,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // match distinct interned forms of one application
        let dir::Type::Application(application) = self.ty_raw(id)? else {
            return Ok(false);
        };
        let dir::Type::Application(from_application) = self.ty_raw(from)? else {
            return Ok(false);
        };
        if from_application.symbol != application.symbol {
            return Ok(false);
        }

        // compare the arguments elementwise
        let from_arguments = self
            .type_ids(from.module_id, from_application.arguments)?
            .to_vec();
        let arguments = self.type_ids(id.module_id, application.arguments)?.to_vec();
        if from_arguments.len() != arguments.len() {
            return Ok(false);
        }
        for (from_argument, argument) in from_arguments.iter().zip(&arguments) {
            if from_argument == argument {
                continue;
            }
            let matched = match (self.ty_raw(*from_argument)?, self.ty_raw(*argument)?) {
                (dir::Type::Parameter(left), dir::Type::Parameter(right)) => left == right,
                _ => false,
            };
            if !matched {
                return Ok(false);
            }
        }

        Ok(true)
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
            // hit every member or elided application a normalize pass rebuilds
            _ if matches!(rule, SubstitutionRule::Normalize { .. }) => match ty {
                // follow a variable to its solution
                dir::Type::Variable(variable) => match self.infer.solution(variable)? {
                    Some(solution) => self.substitution_affects(solution, rule, affected)?,
                    None => false,
                },
                ty => {
                    let mut hit = match &ty {
                        dir::Type::Member(_) => true,
                        dir::Type::Application(application) => {
                            self.language_item(application.symbol)?
                                .is_some_and(crate::sema::language::is_type_computation)
                                || self.is_partial_application(id.module_id, application)?
                        }
                        _ => false,
                    };
                    let mut children = SmallVec::<[dir::GlobalTypeId; 8]>::new();
                    self.for_each_type_child(id.module_id, &ty, |child| children.push(child))?;
                    for child in children {
                        hit |= self.substitution_affects(child, rule, affected)?;
                    }

                    hit
                }
            },
            // hit a replace pass's own source id or an equal application
            _ if matches!(rule, SubstitutionRule::Replace { from, .. }
                if from == id || self.replaces_application(from, id)?) =>
            {
                true
            }
            // hit a bare reference to a conditional-infer binder
            (dir::Type::Application(instance), _)
                if instance.arguments.is_empty()
                    && rule.infer_capture(instance.symbol).is_some() =>
            {
                true
            }
            // hit a direct conditional-infer binder
            (dir::Type::Operation(operation), _)
                if let dir::TypeOperation::Infer(dir::InferType {
                    symbol: Some(symbol),
                    ..
                }) = self.type_operation(id.module_id, operation)?
                    && rule.infer_capture(symbol).is_some() =>
            {
                true
            }
            // hit a bound generic parameter
            (dir::Type::Parameter(parameter), SubstitutionRule::Substitute { .. }) => {
                rule.substituted(parameter).is_some()
            }
            // hit a receiver reference under a receiver rewrite
            (dir::Type::This, SubstitutionRule::Substitute { .. }) => rule.receiver().is_some(),
            // follow a variable to its solution
            (dir::Type::Variable(variable), _) => match self.infer.solution(variable)? {
                Some(solution) => self.substitution_affects(solution, rule, affected)?,
                None => false,
            },
            // hit an inference barrier under an erasing pass
            (_, SubstitutionRule::EraseNoInfer) if self.no_infer_target(id)?.is_some() => true,
            // inherit every other head from its children
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

        // hold this id on the active path across the rebuild
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
        // replace one matched type id or an equal application
        if let SubstitutionRule::Replace { from, to } = rule
            && (id == from || self.replaces_application(from, id)?)
        {
            return Ok(to);
        }

        // resolve one solved variable through its root
        let variable = match self.ty_raw(id)? {
            dir::Type::Variable(variable) => Some(variable),
            _ => None,
        };

        // substitute through a solved variable
        if let Some(variable) = variable {
            return match self.infer.solution(variable)? {
                // substitute through the solution, which flags apart from its variable entry
                Some(solution) => {
                    let mut affected = FxIndexMap::default();
                    self.substitution_affects(solution, rule, &mut affected)?;

                    self.substitute_guarded(target, solution, rule, &affected, substituting)
                }
                // keep an open root as it stands
                None => Ok(id),
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

            // leave an unbound parameter in place under a substitution
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

        // read type records from their owner and intern the result in the target module
        let ty = self.ty(id)?;
        let substituted =
            self.substitute_children(id.module_id, target, ty, rule, affected, substituting)?;

        // rebuild set constructors through their normalizing builders
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

        // normalize the rebuilt entry under a normalizing rule
        if let SubstitutionRule::Normalize { origin }
        | SubstitutionRule::SubstituteInfer {
            origin: Some(origin),
            ..
        } = rule
        {
            // complete an elided application even under rigid arguments
            if let dir::Type::Application(application) = self.ty(rebuilt)?
                && let Some(filled) =
                    self.fill_elided_application(rebuilt.module_id, &application)?
            {
                return Ok(filled);
            }

            // normalize once every parameter, this, and variable the entry holds is closed
            let flags = self.type_flags(rebuilt)?;
            if !flags.has_this()
                && !flags.has_variable()
                && (!flags.has_parameter() || !self.has_open_parameter(rebuilt)?)
            {
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
        self.map_type_children(source, ty, &mut |state, child| {
            state.substitute_guarded(target, child, rule, affected, substituting)
        })
    }
}
