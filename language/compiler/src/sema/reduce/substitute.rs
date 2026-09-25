use smallvec::SmallVec;
use tspp_core::{FxIndexSet, ensure_sufficient_stack};
use tspp_dir as dir;
use tspp_source::ModuleId;

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
    /// Rebuild the reducible nodes, normalizing each closed head.
    Normalize {
        /// The declaration whose entries normalize.
        origin: Origin,
    },
    /// Rebuild the reducible nodes, reducing over rigid parameters and expanding aliases.
    Evaluate {
        /// The declaration whose entries evaluate.
        origin: Origin,
    },
    /// Rebuild every node of a declared type so construction normalizes each head.
    Translate {
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
        origin: Option<Origin>,
    },
    /// Remove every inference barrier.
    EraseNoInfer,
    /// Read the projections one implementation binds on its target.
    BindProjections {
        /// The implementing type whose projections bind.
        owner: dir::GlobalTypeId,
        /// The bound associated members by key.
        bindings: &'a [(dir::StaticKey, dir::GlobalTypeId)],
    },
}

impl SubstitutionRule<'_> {
    /// Return the substituted argument for one parameter.
    fn substituted(&self, parameter: dir::GlobalGenericParameterId) -> Option<dir::GlobalTypeId> {
        match self {
            Self::Substitute { substitution, .. } => substitution
                .bindings
                .iter()
                .find(|binding| binding.parameter == parameter)
                .map(|binding| binding.argument),
            Self::Replace { .. }
            | Self::SubstituteInfer { .. }
            | Self::EraseNoInfer
            | Self::Normalize { .. }
            | Self::Evaluate { .. }
            | Self::Translate { .. }
            | Self::BindProjections { .. } => None,
        }
    }

    /// Return whether this rule expands written alias heads, reducing over rigid parameters.
    fn evaluates(&self) -> bool {
        matches!(self, Self::Evaluate { .. })
    }

    /// Return the origin one normalizing rule reduces under.
    fn normalizing_origin(&self) -> Option<Origin> {
        match self {
            Self::Normalize { origin } | Self::Evaluate { origin } | Self::Translate { origin } => {
                Some(*origin)
            }
            Self::SubstituteInfer {
                origin: Some(origin),
                ..
            } => Some(*origin),
            _ => None,
        }
    }

    /// Return this rule for the branches of a deferred conditional.
    fn suspended(self) -> Option<Self> {
        match self {
            Self::Normalize { .. } | Self::Translate { .. } => None,
            Self::SubstituteInfer { captures, .. } => Some(Self::SubstituteInfer {
                captures,
                origin: None,
            }),
            rule => Some(rule),
        }
    }

    /// Return the receiver replacing `this` references.
    fn receiver(&self) -> Option<dir::GlobalTypeId> {
        match self {
            Self::Substitute { substitution, .. } => substitution.receiver,
            Self::Replace { .. }
            | Self::SubstituteInfer { .. }
            | Self::EraseNoInfer
            | Self::Normalize { .. }
            | Self::Evaluate { .. }
            | Self::Translate { .. }
            | Self::BindProjections { .. } => None,
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
            | Self::Normalize { .. }
            | Self::Evaluate { .. }
            | Self::Translate { .. }
            | Self::BindProjections { .. } => None,
        }
    }
}

impl CheckState<'_> {
    /// Substitute one type graph by replacing matching leaves.
    fn substitute_graph(
        &mut self,
        id: dir::GlobalTypeId,
        rule: SubstitutionRule<'_>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let mut substituting = FxIndexSet::default();

        self.substitute_guarded(id, rule, &mut substituting)
    }

    /// Return whether one head applies a written alias, which rows keep as written.
    fn is_written_alias_head(&mut self, id: dir::GlobalTypeId) -> CompilerResult<bool> {
        let symbol = match self.ty(id)? {
            dir::Type::Application(application) => application.symbol,
            dir::Type::Reference(reference) => reference.symbol,
            _ => return Ok(false),
        };
        let declared = self.definition(symbol)?;
        let Some(dir::Definition::TypeAlias(alias)) = declared.as_deref() else {
            return Ok(false);
        };

        Ok(!matches!(self.ty(alias.value)?, dir::Type::Intrinsic))
    }

    /// Return whether one type graph holds a leaf the rule rewrites, read off its flags.
    fn rule_applies(
        &self,
        id: dir::GlobalTypeId,
        rule: SubstitutionRule<'_>,
    ) -> CompilerResult<bool> {
        let flags = self.type_flags(id)?;
        Ok(match rule {
            SubstitutionRule::Substitute { .. } => {
                flags.has_parameter() || flags.has_this() || flags.has_variable()
            }
            SubstitutionRule::Normalize { .. } => {
                flags.has_member()
                    || flags.has_operation()
                    || flags.has_reference()
                    || flags.has_variable()
            }
            SubstitutionRule::Evaluate { .. } => {
                flags.has_parameter()
                    || flags.has_this()
                    || flags.has_variable()
                    || flags.has_member()
                    || flags.has_operation()
            }
            SubstitutionRule::BindProjections { .. } => flags.has_member(),
            SubstitutionRule::Translate { .. }
            | SubstitutionRule::Replace { .. }
            | SubstitutionRule::SubstituteInfer { .. }
            | SubstitutionRule::EraseNoInfer => true,
        })
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

        // reuse the memoized result of a substitution of closed inputs
        let is_closed = !flags.has_variable() && self.is_closed_substitution(substitution)?;
        let key = is_closed.then(|| {
            let identity = (substitution.receiver, substitution.bindings.clone());

            self.substitution_keys.insert_full(identity).0 as u32
        });
        if let Some(key) = key
            && let Some(known) = self.substituted.get(&(id, key))
        {
            return Ok(*known);
        }
        let substituted =
            self.substitute_graph(id, SubstitutionRule::Substitute { substitution })?;
        if let Some(key) = key {
            self.substituted.insert((id, key), substituted);
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

        // replace every module's copy of the implemented application by its refinement
        let mut id = self.substitute_type(id, &substitution)?;
        if implementation != base {
            let base_arguments: SmallVec<[_; 8]> =
                self.type_ids(module, application.arguments)?.into();
            for occurrence in self.plain_applications_of(id, application.symbol, &base_arguments)? {
                id = self.replace_type(id, occurrence, implementation)?;
            }
        }

        Ok(id)
    }

    /// Normalize one type graph, reducing every closed head and keeping aliases.
    pub(in crate::sema) fn normalize_type(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.substitute_graph(id, SubstitutionRule::Normalize { origin })
    }

    /// Evaluate one type graph to the form lowering reads, expanding aliases.
    pub(in crate::sema) fn evaluate_type(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.substitute_graph(id, SubstitutionRule::Evaluate { origin })
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
            let normal = self.substitute_graph(ty, SubstitutionRule::Translate { origin })?;

            // normalize an alias to the type it names
            let normal = match self.symbol_kind(symbol)? {
                dir::SymbolKind::TypeAlias => self.normalize(origin, normal)?,
                _ => normal,
            };
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
            let normal = self.substitute_graph(ty, SubstitutionRule::Translate { origin })?;
            if normal != ty {
                self.module.types_tail.set_node_type(node, normal);
            }
        }

        // rebuild each recorded member subject over normalized heads
        let subjects: Vec<_> = self.module.iter_member_subjects().collect();
        for (site, mut subject, key) in subjects {
            let origin = Origin::Node(site.node(), subject.scope);
            let receiver =
                self.substitute_graph(subject.receiver, SubstitutionRule::Translate { origin })?;
            let target =
                self.substitute_graph(subject.target, SubstitutionRule::Translate { origin })?;
            let key_type =
                self.substitute_graph(subject.key_source, SubstitutionRule::Translate { origin })?;

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

        Ok(())
    }

    /// Remove inference barriers after candidate inference has closed.
    pub(in crate::sema) fn erase_inference_barriers(
        &mut self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.substitute_graph(id, SubstitutionRule::EraseNoInfer)
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

            return self.erase_inference_barriers(id);
        }

        // reuse the decided erasure of a closed graph
        if let Some(erased) = self.erasures.get(&id) {
            return Ok(*erased);
        }

        // decide the erasure once
        let erased = self.erase_inference_barriers(id)?;
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
        let from_arguments = self.type_ids(from.module_id, from_application.arguments)?;
        let arguments = self.type_ids(id.module_id, application.arguments)?;
        if from_arguments.len() != arguments.len() {
            return Ok(false);
        }
        for (from_argument, argument) in from_arguments.iter().zip(arguments) {
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
        id: dir::GlobalTypeId,
        from: dir::GlobalTypeId,
        to: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let rule = SubstitutionRule::Replace { from, to };

        self.substitute_graph(id, rule)
    }

    /// Bind the projections one implementation fixes on its target inside one type.
    pub(in crate::sema) fn bind_projections(
        &mut self,
        id: dir::GlobalTypeId,
        owner: dir::GlobalTypeId,
        bindings: &[(dir::StaticKey, dir::GlobalTypeId)],
    ) -> CompilerResult<dir::GlobalTypeId> {
        let owner = self.shallow_resolve(owner)?;

        self.substitute_graph(id, SubstitutionRule::BindProjections { owner, bindings })
    }

    /// Substitute conditional-infer captures inside one branch type.
    pub(in crate::sema) fn substitute_infer_captures(
        &mut self,
        origin: Option<Origin>,
        id: dir::GlobalTypeId,
        captures: &[InferSubstitution],
    ) -> CompilerResult<dir::GlobalTypeId> {
        if captures.is_empty() {
            return Ok(id);
        }

        self.substitute_graph(id, SubstitutionRule::SubstituteInfer { captures, origin })
    }

    /// Substitute one type with the active path tracked.
    fn substitute_guarded(
        &mut self,
        id: dir::GlobalTypeId,
        rule: SubstitutionRule<'_>,
        substituting: &mut FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // break a substitution cycle at its repeated head
        if !substituting.insert(id) {
            return Ok(id);
        }

        // hold this id on the active path across the rebuild
        let substituted = ensure_sufficient_stack(|| self.substitute_id(id, rule, substituting));
        substituting.swap_remove(&id);

        substituted
    }

    /// Substitute one type id once the cycle guard passes it.
    fn substitute_id(
        &mut self,
        id: dir::GlobalTypeId,
        rule: SubstitutionRule<'_>,
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
                // substitute through the solution
                Some(solution) => self.substitute_guarded(solution, rule, substituting),
                // keep an open root as it stands
                None => Ok(id),
            };
        }

        // preserve every graph without a leaf the rule rewrites
        if !self.rule_applies(id, rule)? {
            return Ok(id);
        }

        // bind one projection on the implementing type to the implementation's associated member
        if let SubstitutionRule::BindProjections { owner, bindings } = rule
            && let dir::Type::Member(member) = self.ty(id)?
        {
            let member = self.type_member(id.module_id, member)?;
            if self.shallow_resolve(member.owner)? == owner
                && self.type_ids(id.module_id, member.arguments)?.is_empty()
                && let Some((_, value)) = bindings.iter().find(|(key, _)| *key == member.key)
            {
                return Ok(*value);
            }
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
        if let dir::Type::Parameter(parameter) = self.ty(id)?
            && let Some(replacement) = rule.substituted(parameter)
        {
            return Ok(replacement);
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
            return self.substitute_guarded(target_id, rule, substituting);
        }

        // read type records from their owner and intern the result in the checked module
        let ty = self.ty(id)?;
        let substituted = self.substitute_children(id.module_id, ty, rule, substituting)?;

        // rebuild set constructors through their normalizing builders
        let rebuilt = match substituted {
            dir::Type::Union(union) => {
                let elements: SmallVec<[_; 8]> =
                    self.type_ids(self.module_id, union.elements)?.into();

                self.normalized_union_type(elements)
            }
            dir::Type::Intersection(intersection) => {
                let elements: SmallVec<[_; 8]> =
                    self.type_ids(self.module_id, intersection.elements)?.into();

                self.normalized_intersection_type(elements)
            }
            substituted => self.intern_type(substituted),
        }?;

        // normalize the rebuilt entry under a normalizing rule
        if let Some(origin) = rule.normalizing_origin() {
            // complete an elided application even under rigid arguments
            if let dir::Type::Application(application) = self.ty(rebuilt)?
                && let Some(filled) =
                    self.fill_elided_application(rebuilt.module_id, &application)?
            {
                return Ok(filled);
            }

            // keep a written alias application as written under every rule but evaluation
            let evaluates = rule.evaluates();
            if !evaluates && self.is_written_alias_head(rebuilt)? {
                return Ok(rebuilt);
            }

            // normalize once every this and variable the entry holds is closed
            let flags = self.type_flags(rebuilt)?;
            let is_open = flags.has_this() || flags.has_type_parameter() && !evaluates;
            if !is_open && !flags.has_variable() {
                return self.normalize(origin, rebuilt);
            }
        }

        Ok(rebuilt)
    }

    /// Rebuild one type with substituted children.
    fn substitute_children(
        &mut self,
        source: ModuleId,
        ty: dir::Type,
        rule: SubstitutionRule<'_>,
        substituting: &mut FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::Type> {
        // substitute a conditional's checked types under the rule and its branches suspended
        if let dir::Type::Operation(operation) = ty
            && let dir::TypeOperation::Conditional(mut conditional) =
                self.type_operation(source, operation)?
        {
            conditional.left = self.substitute_guarded(conditional.left, rule, substituting)?;
            conditional.right = self.substitute_guarded(conditional.right, rule, substituting)?;
            if let Some(suspended) = rule.suspended() {
                conditional.then_type =
                    self.substitute_guarded(conditional.then_type, suspended, substituting)?;
                conditional.else_type =
                    self.substitute_guarded(conditional.else_type, suspended, substituting)?;
            }
            let operation = self
                .module
                .types_tail
                .intern_operation(dir::TypeOperation::Conditional(conditional));

            return Ok(dir::Type::Operation(operation));
        }

        self.map_type_children(source, ty, &mut |state, child| {
            state.substitute_guarded(child, rule, substituting)
        })
    }
}
