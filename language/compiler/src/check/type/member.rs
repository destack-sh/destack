use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexSet;
use smallvec::SmallVec;

use crate::check::{
    Answer, CheckState, CheckWarning, Dependency, GenericTemplateId, Origin, Relation,
    Substitution, Task,
};
use crate::{CheckError, CompilerResult, DiagnosticAnchor};

/// Result of looking up one member on a receiver type.
#[derive(Debug, Clone)]
pub(in crate::check) enum MemberLookup {
    /// Lookup is waiting on unresolved dependencies.
    Pending(SmallVec<[Dependency; 2]>),
    /// No member exists.
    Missing,
    /// One structural field exists.
    Field(dir::GlobalTypeId),
    /// One or more declaration-backed members exist.
    Found(Vec<MemberCandidate>),
}

impl MemberLookup {
    /// Return a lookup from collected candidates.
    fn from_candidates(candidates: Vec<MemberCandidate>) -> Self {
        if candidates.is_empty() {
            Self::Missing
        } else {
            Self::Found(candidates)
        }
    }
}

/// One declaration-backed member candidate.
#[derive(Debug, Clone)]
pub(in crate::check) struct MemberCandidate {
    /// The declaring member symbol.
    pub(in crate::check) symbol: Option<dir::GlobalSymbolId>,
    /// The substituted member type.
    pub(in crate::check) ty: dir::GlobalTypeId,
    /// The committed member value when the member carries one.
    pub(in crate::check) value: Option<dir::GlobalStaticId>,
    /// The substituted static value of the member, when it has one.
    pub(in crate::check) value_type: Option<dir::GlobalTypeId>,
}

#[allow(clippy::too_many_arguments)]
impl CheckState<'_> {
    /// Project one type-level member access through its owner.
    /// Returns ready none when the projection must stay symbolic.
    pub(in crate::check) fn project_member(
        &mut self,
        origin: Origin,
        member: &dir::MemberType,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let module = origin.module();
        let lookup = self.lookup_member(
            origin,
            module,
            member.owner,
            dir::MemberSpace::Static,
            member.key,
        )?;

        match lookup {
            // single projections substitute member arguments
            MemberLookup::Field(ty) => Ok(Answer::Ready(Some(ty))),
            MemberLookup::Found(candidates) => match candidates.as_slice() {
                [candidate] => {
                    // comptime const projections yield their static values
                    if let Some(written) = candidate.value_type {
                        return Ok(Answer::Ready(Some(written)));
                    }
                    if let Some(value) = candidate.value {
                        let source = self.origin_source_node(origin)?;
                        let spelling = self.push_type(module, dir::Type::Static(value), source)?;

                        return Ok(Answer::Ready(Some(spelling)));
                    }

                    Ok(Answer::Ready(Some(candidate.ty)))
                }
                _ => Ok(Answer::Ready(None)),
            },
            MemberLookup::Missing => Ok(Answer::Ready(None)),
            MemberLookup::Pending(blockers) => Ok(Answer::Pending(blockers)),
        }
    }

    /// Look up one member on a receiver type.
    pub(in crate::check) fn lookup_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        // close the receiver root first
        let receiver = match self.evaluate_root(origin, receiver)? {
            Answer::Ready(receiver) => receiver,
            Answer::Pending(blockers) => return Ok(MemberLookup::Pending(blockers)),
        };

        match self.ty(receiver)? {
            // memory forms look through their payloads
            dir::Type::Form(form) => {
                let value = form.value;

                self.lookup_member(origin, module, value, space, key)
            }

            // declaration references search their definition members
            dir::Type::Reference(instance) => {
                // parameter symbols search through their constraints
                if instance.arguments.is_empty()
                    && let Some(parameter) = self.generics.parameter_by_symbol(instance.symbol)
                {
                    return self.lookup_constraint_member(origin, module, parameter, space, key);
                }

                let instance = instance.clone();

                self.lookup_symbol_member(origin, module, receiver, instance, space, key)
            }

            // generic parameters search through their constraints
            dir::Type::Parameter(parameter) => {
                let parameter = *parameter;

                self.lookup_constraint_member(origin, module, parameter, space, key)
            }

            // structural shapes expose their fields
            dir::Type::Shape(shape) => {
                let field = shape
                    .fields
                    .iter()
                    .find(|field| field.key == key)
                    .map(|field| field.ty);

                match field {
                    Some(ty) => Ok(MemberLookup::Field(ty)),
                    None => Ok(MemberLookup::Missing),
                }
            }

            // tuples expose their labeled elements
            dir::Type::Tuple(tuple) => {
                let element = tuple
                    .elements
                    .iter()
                    .find(|element| {
                        element
                            .label
                            .is_some_and(|label| key == dir::StaticKey::Name(label))
                    })
                    .map(|element| element.ty);

                match element {
                    Some(ty) => Ok(MemberLookup::Field(ty)),
                    None => Ok(MemberLookup::Missing),
                }
            }

            // unions join member lookups across their elements
            dir::Type::Union(union) => {
                let elements = union.elements.iter().copied().collect::<SmallVec<[_; 4]>>();

                self.lookup_union_member(origin, module, &elements, space, key)
            }

            // intersections expose every part's members
            dir::Type::Intersection(intersection) => {
                let elements = intersection
                    .elements
                    .iter()
                    .copied()
                    .collect::<SmallVec<[_; 4]>>();
                for element in elements {
                    let element = self.resolve_root(element)?;
                    match self.lookup_member(origin, module, element, space, key)? {
                        MemberLookup::Missing => continue,
                        lookup => return Ok(lookup),
                    }
                }

                Ok(MemberLookup::Missing)
            }

            // scalars search their language-item owners
            dir::Type::Literal(literal) => {
                let owner = literal.owner_item();

                self.lookup_builtin_member(origin, module, receiver, owner, space, key)
            }
            dir::Type::Primitive(primitive) => {
                let owner = primitive.owner_item();

                self.lookup_builtin_member(origin, module, receiver, owner, space, key)
            }
            // collection views search their owner declarations
            dir::Type::Array(_) => self.lookup_builtin_member(
                origin,
                module,
                receiver,
                Some(dir::LanguageItem::Array),
                space,
                key,
            ),
            dir::Type::Slice(_) => self.lookup_builtin_member(
                origin,
                module,
                receiver,
                Some(dir::LanguageItem::Slice),
                space,
                key,
            ),
            dir::Type::FixedArray(_) => self.lookup_builtin_member(
                origin,
                module,
                receiver,
                Some(dir::LanguageItem::FixedArray),
                space,
                key,
            ),

            _ => Ok(MemberLookup::Missing),
        }
    }

    /// Look up one member through a generic parameter's constraint.
    fn lookup_constraint_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        parameter: dir::GlobalGenericParameterId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        let Some(binding) = self.generic_parameter(parameter) else {
            return Ok(MemberLookup::Missing);
        };
        let Some(constraint) = binding.constraint else {
            return Ok(MemberLookup::Missing);
        };

        self.lookup_member(origin, module, constraint, space, key)
    }

    /// Look up one member on a builtin scalar through its language item owner.
    fn lookup_builtin_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        owner: Option<dir::LanguageItem>,
        space: dir::MemberSpace,
        key: dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        let Some(owner) = owner else {
            return Ok(MemberLookup::Missing);
        };
        let symbol = self.language_symbol(owner);
        let instance = dir::GenericInstance {
            symbol,
            arguments: Vec::new(),
        };

        self.lookup_symbol_member(origin, module, receiver, instance, space, key)
    }

    /// Join member lookups across union elements.
    fn lookup_union_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        elements: &[dir::GlobalTypeId],
        space: dir::MemberSpace,
        key: dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        let mut candidates = Vec::new();
        let mut fields = SmallVec::<[dir::GlobalTypeId; 4]>::new();

        // every element must expose the member
        for element in elements {
            match self.lookup_member(origin, module, *element, space, key)? {
                MemberLookup::Field(ty) => fields.push(ty),
                MemberLookup::Found(found) => candidates.extend(found),
                MemberLookup::Missing => return Ok(MemberLookup::Missing),
                MemberLookup::Pending(blockers) => return Ok(MemberLookup::Pending(blockers)),
            }
        }

        // pure field unions join into one field type
        if candidates.is_empty() {
            let joined = match fields.as_slice() {
                [single] => *single,
                _ => {
                    let source = self.origin_source_node(origin)?;
                    let union = dir::Type::Union(dir::UnionType {
                        elements: fields.into_iter().collect(),
                    });

                    self.push_type(origin.module(), union, source)?
                }
            };

            return Ok(MemberLookup::Field(joined));
        }

        Ok(MemberLookup::from_candidates(candidates))
    }

    /// Look up one member on a declaration reference.
    fn lookup_symbol_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        instance: dir::GenericInstance,
        space: dir::MemberSpace,
        key: dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        // load external definitions on first use
        let mut instance = instance;
        if !self.is_component_module(instance.symbol.module_id) {
            instance.symbol = self.resolve_external_alias(instance.symbol)?;
            self.import_external_module(instance.symbol.module_id)?;
        }

        // search inherent members before extensions
        let inherent =
            self.lookup_inherent_member(origin, module, receiver, &instance, space, key)?;
        match inherent {
            MemberLookup::Found(_) | MemberLookup::Field(_) | MemberLookup::Pending(_) => {
                return Ok(inherent);
            }
            MemberLookup::Missing => {}
        }

        self.lookup_extension_member(origin, module, receiver, &instance, space, key)
    }

    /// Look up one inherent member on a declaration, walking its heritage.
    fn lookup_inherent_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        instance: &dir::GenericInstance,
        space: dir::MemberSpace,
        key: dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        // collect own members and heritage applications
        let Some(definition) = self.definition(instance.symbol) else {
            return Ok(MemberLookup::Missing);
        };
        let members = definition
            .members_with_key(space, key)
            .map(|member| {
                // accessor reads project the getter's return type
                let is_getter = matches!(
                    member,
                    dir::DefinitionMember::Method(method)
                        if method.role == Some(dir::FunctionRole::Getter)
                );

                (
                    member.symbol(),
                    member.ty(),
                    member.value(),
                    member.condition(),
                    is_getter,
                )
            })
            .collect::<SmallVec<[_; 2]>>();
        let heritages = definition
            .heritages()
            .iter()
            .map(|heritage| (heritage.symbol, heritage.arguments.clone()))
            .collect::<SmallVec<[_; 2]>>();

        // substitute applied arguments and the qualified receiver
        let substitution = self
            .parameter_substitution(instance)?
            .with_receiver(receiver);
        let source = self.origin_source_node(origin)?;
        let mut candidates = Vec::new();
        for (symbol, ty, value, condition, is_getter) in members {
            let Some(ty) = ty else {
                continue;
            };

            // gate candidates on their substituted @if availability
            match self.decide_member_availability(origin, module, condition, &substitution)? {
                Answer::Ready(true) => {}
                Answer::Ready(false) => continue,
                Answer::Pending(blockers) => return Ok(MemberLookup::Pending(blockers)),
            }
            let ty = if substitution.is_empty() {
                ty
            } else {
                self.fold_type(module, source, ty, substitution.rewrite())?
            };

            // accessor reads project the getter's return type
            let ty = if is_getter {
                match self.ty(ty)? {
                    dir::Type::Function(function) => function.return_type.unwrap_or(ty),
                    _ => ty,
                }
            } else {
                ty
            };

            // carry substituted static value types for projections
            let written = match symbol.and_then(|symbol| self.inputs.symbol_value(symbol)) {
                Some(written) if !substitution.is_empty() => {
                    Some(self.fold_type(module, source, written, substitution.rewrite())?)
                }
                written => written,
            };

            candidates.push(MemberCandidate {
                symbol,
                ty,
                value,
                value_type: written,
            });
        }
        if !candidates.is_empty() {
            return Ok(MemberLookup::Found(candidates));
        }

        // search substituted heritage applications
        for (symbol, arguments) in heritages {
            // substitute applied arguments into the heritage arguments
            let mut arguments = arguments;
            for argument in &mut arguments {
                if !substitution.is_empty() {
                    *argument =
                        self.fold_type(module, source, *argument, substitution.rewrite())?;
                }
            }

            let heritage = dir::GenericInstance { symbol, arguments };
            let lookup =
                self.lookup_inherent_member(origin, module, receiver, &heritage, space, key)?;
            match lookup {
                MemberLookup::Missing => {}
                lookup => return Ok(lookup),
            }
        }

        Ok(MemberLookup::Missing)
    }

    /// Look up one extension member on a declaration reference.
    fn lookup_extension_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        instance: &dir::GenericInstance,
        space: dir::MemberSpace,
        key: dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        let extensions = self.visible_extensions(module, instance.symbol);

        // visit extension declarations in resolution order
        let mut candidates = Vec::new();
        let mut seen = IndexSet::new();
        for extension_symbol in extensions {
            let lookup =
                self.lookup_one_extension(origin, module, receiver, extension_symbol, space, key)?;

            match lookup {
                MemberLookup::Found(found) => {
                    // collect first declarations of each member symbol
                    for candidate in found {
                        if candidate.symbol.is_some_and(|symbol| !seen.insert(symbol)) {
                            continue;
                        }
                        candidates.push(candidate);
                    }
                }
                MemberLookup::Pending(blockers) => return Ok(MemberLookup::Pending(blockers)),
                MemberLookup::Missing | MemberLookup::Field(_) => {}
            }
        }

        Ok(MemberLookup::from_candidates(candidates))
    }

    /// Collect extension symbols visible from one module for one target.
    fn visible_extensions(
        &self,
        module: ModuleId,
        target: dir::GlobalSymbolId,
    ) -> SmallVec<[dir::GlobalSymbolId; 4]> {
        let mut symbols = SmallVec::new();

        // collect extensions declared beside the looking module
        if let Some(state) = self.modules.get(&module) {
            let working = &state.working.definitions;
            symbols.extend(working.target_extensions(target).iter().copied());
            symbols.extend(working.blanket_extensions().iter().copied());
        }

        // collect inherent extensions beside the target declaration
        if target.module_id != module {
            if let Some(state) = self.modules.get(&target.module_id) {
                let working = &state.working.definitions;
                symbols.extend(working.target_extensions(target).iter().copied());
                symbols.extend(working.blanket_extensions().iter().copied());
            }
            if let Some(external) = self.external_modules.get(&target.module_id) {
                symbols.extend(external.definitions.target_extensions(target));
                symbols.extend(external.definitions.blanket_extensions());
            }
        }

        // collect explicitly imported extension symbols
        for (_, symbol) in self.module(module).resolved.imports.symbol_targets() {
            if self
                .definition(symbol)
                .is_some_and(|definition| matches!(definition, dir::Definition::Extension(_)))
                && !symbols.contains(&symbol)
            {
                symbols.push(symbol);
            }
        }

        symbols
    }

    /// Check one freshly walked extension against the coherence rules:
    /// blanket implementations stay in the contract's package, duplicate
    /// implementations conflict, and fully foreign implementations warn.
    /// Each conflict reports once, at the later declaration.
    pub(in crate::check) fn check_extension_coherence(
        &mut self,
        module: ModuleId,
        source: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        let Some(dir::Definition::Extension(extension)) = self.definition(symbol) else {
            return Ok(());
        };
        let target = extension.target;
        let implements = extension
            .implements
            .iter()
            .map(|heritage| heritage.symbol)
            .collect::<SmallVec<[_; 2]>>();
        if implements.is_empty() {
            return Ok(());
        }
        let package = module.package_id;
        let (module, anchor) = self.source_anchor(source);

        match target {
            // blanket implementations must live beside their contract
            dir::ExtensionTarget::Nominal { root, ty } => {
                // a fully foreign implementation risks program-wide conflicts
                let foreign_target = root.module_id.package_id != package;
                for contract in implements.iter().copied() {
                    if foreign_target && contract.module_id.package_id != package {
                        let warning = CheckWarning::ForeignImplementation {
                            anchor: anchor.clone(),
                            module,
                            contract: self.format_symbol(contract),
                            ty: self.format_symbol(root),
                        };
                        self.module_mut(module).warnings.push(warning.into());
                    }
                }

                self.check_conflicting_implementations(
                    module,
                    source,
                    anchor,
                    symbol,
                    root,
                    ty,
                    &implements,
                )?;
            }
            _ => {
                for contract in implements.iter().copied() {
                    if contract.module_id.package_id != package {
                        let error = CheckError::ForeignBlanketImplementation {
                            anchor: anchor.clone(),
                            module,
                            contract: self.format_symbol(contract),
                        };
                        self.module_mut(module).diagnostics.push(error.into());
                    }
                }
            }
        }

        Ok(())
    }

    /// Report visible implementations conflicting with one new extension.
    fn check_conflicting_implementations(
        &mut self,
        module: ModuleId,
        source: dir::GlobalNodeIdAny,
        anchor: DiagnosticAnchor,
        symbol: dir::GlobalSymbolId,
        root: dir::GlobalSymbolId,
        ty: dir::GlobalTypeId,
        implements: &[dir::GlobalSymbolId],
    ) -> CompilerResult<()> {
        // generic implementations need instantiation overlap checking
        if self.generics.template_by_symbol(symbol).is_some() {
            return Ok(());
        }

        // collect comparable implementations in one pure pass
        let mut candidates = SmallVec::<[(dir::GlobalTypeId, dir::GlobalSymbolId); 2]>::new();
        for other in self.visible_extensions(module, root) {
            if other == symbol || self.generics.template_by_symbol(other).is_some() {
                continue;
            }
            let Some(dir::Definition::Extension(extension)) = self.definition(other) else {
                continue;
            };
            let dir::ExtensionTarget::Nominal {
                root: other_root,
                ty: other_ty,
            } = extension.target
            else {
                continue;
            };
            if other_root != root {
                continue;
            }
            let shared = extension
                .implements
                .iter()
                .map(|heritage| heritage.symbol)
                .find(|contract| implements.contains(contract));
            if let Some(contract) = shared {
                candidates.push((other_ty, contract));
            }
        }

        // distinct closed receivers do not conflict
        let origin = Origin::Node(source);
        for (other_ty, contract) in candidates {
            if self.decide_relation(origin, Relation::Equal, ty, other_ty)? != Answer::Ready(true) {
                continue;
            }

            let error = CheckError::ConflictingImplementation {
                anchor: anchor.clone(),
                module,
                contract: self.format_symbol(contract),
                ty: self.format_type(ty),
            };
            self.module_mut(module).diagnostics.push(error.into());
        }

        Ok(())
    }

    /// Look up matching members from one extension declaration.
    fn lookup_one_extension(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        extension_symbol: dir::GlobalSymbolId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        // gate the extension on its @if availability
        match self.decide_availability(extension_symbol)? {
            Answer::Ready(true) => {}
            Answer::Ready(false) => return Ok(MemberLookup::Missing),
            Answer::Pending(blockers) => return Ok(MemberLookup::Pending(blockers)),
        }

        // read the extension members
        let Some(dir::Definition::Extension(extension)) = self.definition(extension_symbol) else {
            return Ok(MemberLookup::Missing);
        };
        if !extension.is_visible_from(module) {
            return Ok(MemberLookup::Missing);
        }
        let target_type = extension.target.r#type();
        let matched = extension
            .members
            .iter()
            .filter(|member| member.space() == space && member.key() == Some(key))
            .map(|member| {
                // accessor reads project the getter's return type
                let is_getter = matches!(
                    member,
                    dir::DefinitionMember::Method(method)
                        if method.role == Some(dir::FunctionRole::Getter)
                );

                (
                    member.symbol(),
                    member.ty(),
                    member.value(),
                    member.condition(),
                    is_getter,
                )
            })
            .collect::<SmallVec<[_; 2]>>();
        if matched.is_empty() {
            return Ok(MemberLookup::Missing);
        }
        let members = matched;
        let where_clauses = match self.definition(extension_symbol) {
            Some(dir::Definition::Extension(extension)) => extension.where_clauses.clone(),
            _ => Vec::new(),
        };

        // hypothesize extension generics and match the receiver
        let template = self.symbol_template(extension_symbol);
        let probe = self.begin_probe();
        let result = self.match_extension(
            origin,
            module,
            receiver,
            template,
            target_type,
            &where_clauses,
            &members,
        );

        match result {
            // matched member types escape the probe, so their harvested
            // allocations must survive the rollback
            Ok(lookup @ (MemberLookup::Field(_) | MemberLookup::Found(_))) => {
                self.harvest_probe(probe)?;

                Ok(lookup)
            }
            Ok(MemberLookup::Missing) => {
                self.unwind_probe(probe)?;

                Ok(MemberLookup::Missing)
            }
            // blockers that died with the probe cannot wake this extension
            Ok(MemberLookup::Pending(blockers)) => {
                self.unwind_probe(probe)?;

                let blockers = self.surviving_blockers(blockers);
                if blockers.is_empty() {
                    Ok(MemberLookup::Missing)
                } else {
                    Ok(MemberLookup::Pending(blockers))
                }
            }
            Err(error) => {
                self.unwind_probe(probe)?;

                Err(error)
            }
        }
    }

    /// Match one extension target against a receiver under an active probe.
    fn match_extension(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        template: Option<GenericTemplateId>,
        target_type: dir::GlobalTypeId,
        where_clauses: &[dir::ExtensionWhereClause],
        members: &[(
            Option<dir::GlobalSymbolId>,
            Option<dir::GlobalTypeId>,
            Option<dir::GlobalStaticId>,
            Option<dir::GlobalTypeId>,
            bool,
        )],
    ) -> CompilerResult<MemberLookup> {
        let source = self.origin_source_node(origin)?;

        // hypothesize fresh variables for the extension's generic parameters
        let substitution = match template {
            Some(template) => self.instantiate_template(origin, template)?,
            None => Default::default(),
        };
        let substitution = substitution.with_receiver(receiver);
        let target_type = if substitution.is_empty() {
            target_type
        } else {
            self.fold_type(module, source, target_type, substitution.rewrite())?
        };

        // require the receiver to match the extension target
        match self.constrain(origin, Relation::Assignable, receiver, target_type)? {
            Answer::Ready(true) => {}
            Answer::Ready(false) => return Ok(MemberLookup::Missing),
            Answer::Pending(blockers) => return Ok(MemberLookup::Pending(blockers)),
        }

        // solve hypothesized parameters from the matched receiver bounds
        let floor = self.queue.solve_count();
        for argument in substitution.arguments.iter().copied() {
            if let Some(variable) = self.root_variable(argument)? {
                self.queue_task(Task::Solve(variable));
            }
        }
        // reject extensions whose inferred hypotheses violate their constraints
        if !self.drain_probe_tasks(floor)? {
            return Ok(MemberLookup::Missing);
        }
        self.close_hypotheses(module, source, &substitution)?;

        // require every where clause to hold
        for clause in where_clauses {
            let left = if substitution.is_empty() {
                clause.left
            } else {
                self.fold_type(module, source, clause.left, substitution.rewrite())?
            };
            let right = if substitution.is_empty() {
                clause.right
            } else {
                self.fold_type(module, source, clause.right, substitution.rewrite())?
            };

            match self.decide_relation(origin, Relation::Satisfies, left, right)? {
                Answer::Ready(true) => {}
                Answer::Ready(false) => return Ok(MemberLookup::Missing),
                Answer::Pending(blockers) => return Ok(MemberLookup::Pending(blockers)),
            }
        }

        // harvest matched members through resolved hypothesis variables
        let mut candidates = Vec::new();
        for (symbol, ty, value, condition, is_getter) in members {
            let Some(ty) = ty else {
                continue;
            };

            // gate members on their substituted @if availability
            match self.decide_member_availability(origin, module, *condition, &substitution)? {
                Answer::Ready(true) => {}
                Answer::Ready(false) => continue,
                Answer::Pending(blockers) => return Ok(MemberLookup::Pending(blockers)),
            }
            let ty = if substitution.is_empty() {
                *ty
            } else {
                self.fold_type(module, source, *ty, substitution.rewrite())?
            };
            let ty = self.harvest_type(module, source, ty)?;
            // accessor reads project the getter's return type
            let ty = if *is_getter {
                match self.ty(ty)? {
                    dir::Type::Function(function) => function.return_type.unwrap_or(ty),
                    _ => ty,
                }
            } else {
                ty
            };

            // carry substituted static value types for projections
            let written = match symbol.and_then(|symbol| self.inputs.symbol_value(symbol)) {
                Some(written) if !substitution.is_empty() => {
                    let folded = self.fold_type(module, source, written, substitution.rewrite())?;

                    Some(self.harvest_type(module, source, folded)?)
                }
                written => written,
            };

            candidates.push(MemberCandidate {
                symbol: *symbol,
                ty,
                value: *value,
                value_type: written,
            });
        }

        Ok(MemberLookup::from_candidates(candidates))
    }

    /// Decide one member's @if availability at a use site.
    ///
    /// Symbolic residues stay unavailable: a use of a conditionally
    /// available member must sit under a guard entailing its condition,
    /// which the active assumptions reduce to a literal.
    pub(in crate::check) fn decide_member_availability(
        &mut self,
        origin: Origin,
        module: ModuleId,
        condition: Option<dir::GlobalTypeId>,
        substitution: &Substitution,
    ) -> CompilerResult<Answer<bool>> {
        let Some(condition) = condition else {
            return Ok(Answer::Ready(true));
        };

        // substitute applied arguments into the declaration-context predicate
        let condition = if substitution.is_empty() {
            condition
        } else {
            let source = self.origin_source_node(origin)?;

            self.fold_type(module, source, condition, substitution.rewrite())?
        };

        // reduce under the use site's guard assumptions
        let reduced = match self.evaluate_root(origin, condition)? {
            Answer::Ready(reduced) => reduced,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };

        match self.ty(reduced)? {
            dir::Type::Literal(dir::ScalarLiteral::Boolean(holds)) => Ok(Answer::Ready(*holds)),
            _ => Ok(Answer::Ready(false)),
        }
    }
}
