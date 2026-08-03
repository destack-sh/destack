use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    Answer, BodyState, CheckState, InterfaceMember, MemberCandidate, MemberLookup, ObligationCheck,
    ObligationFailure, Origin, Relation, TypeSubstitution, answer,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Check and record one declaration's interface conformance.
    pub(in crate::check) fn check_interface_conformance(
        &mut self,
        origin: Origin,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Answer<ObligationCheck>> {
        let Some(definition) = self.definition(symbol)? else {
            return Err(CompilerError::Internal {
                message: format!("implementation obligation has no definition: {symbol:?}"),
            });
        };
        let members = definition.members().to_vec();
        let implementations = definition.implementations().to_vec();
        if implementations.is_empty() {
            return Ok(Answer::Ready(ObligationCheck::holds()));
        }
        let target = match definition {
            dir::Definition::Extension(extension) => extension.target.r#type(),
            dir::Definition::Struct(_) | dir::Definition::Class(_) | dir::Definition::Enum(_) => {
                let instance = self.declaration_instance(symbol)?;

                self.intern_type(dir::Type::Application(instance))?
            }
            dir::Definition::TypeAlias(_)
            | dir::Definition::Interface(_)
            | dir::Definition::Newtype(_) => {
                return Err(CompilerError::Internal {
                    message: format!("definition {symbol:?} cannot implement interfaces"),
                });
            }
        };
        let mut selected = Vec::new();
        let mut failures = Vec::new();

        // select every declared implementation before mutating DIR
        for declared in implementations {
            let heritage = declared.interface;
            let (interface_module, interface) = self.require_nominal_application(heritage.ty)?;

            // record the declarations behind each requirement
            let conforms = answer!(self.conform_declared_implementation(
                origin,
                heritage.ty,
                target,
                &members,
                interface_module,
                &interface,
            )?);
            if !conforms {
                failures.push(ObligationFailure::InterfaceNotImplemented {
                    source: heritage.source,
                    ty: target,
                    interface: heritage.ty,
                });
                continue;
            }

            let instantiation = TypeSubstitution::default().with_receiver(target);
            let implementation = answer!(self.select_interface_implementation(
                origin,
                symbol,
                heritage.source,
                heritage.ty,
                target,
                &members,
                &instantiation,
                interface_module,
                &interface,
            )?);
            match implementation {
                Ok(implementation) => selected.push(implementation),
                Err(failure) => failures.push(failure),
            }
        }
        self.commit_interface_implementations(symbol, selected)?;

        Ok(Answer::Ready(ObligationCheck::from_failures(failures)))
    }

    /// Check one extension's implementation coherence.
    pub(in crate::check) fn check_implementation_coherence(
        &mut self,
        origin: Origin,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Answer<ObligationCheck>> {
        let source = self.origin_source(origin)?;
        let Some(dir::Definition::Extension(extension)) = self.definition(symbol)? else {
            return Err(CompilerError::Internal {
                message: format!("extension obligation has no extension definition: {symbol:?}"),
            });
        };
        let target = extension.target;
        let form = extension.form;
        let implements = self
            .declared_implementations(symbol)?
            .into_iter()
            .collect::<SmallVec<[_; 2]>>();

        // reject anonymous exported extensions on nonlocal targets
        let module = source.module_id;
        let mut failures = Vec::new();
        if self.is_unnamed_exported_nonlocal_extension(module, symbol, form, target) {
            failures.push(ObligationFailure::UnnamedExportedNonlocalExtension {
                source,
                target: target.r#type(),
            });
        }

        if implements.is_empty() {
            let check = ObligationCheck::from_failures(failures);

            return Ok(Answer::Ready(check));
        }
        let package = module.package_id;

        match target {
            // reject extension implementation pairs outside both packages
            dir::ExtensionTarget::Rooted { root, ty } => {
                let foreign_target = root.module_id.package_id != package;
                for implementation in &implements {
                    let (_, interface) =
                        self.require_nominal_application(implementation.interface.ty)?;
                    let interface = interface.symbol;
                    if foreign_target && interface.module_id.package_id != package {
                        failures.push(ObligationFailure::NonLocalImplementation {
                            source,
                            interface,
                            ty: root,
                        });
                    }
                }

                let conflicts = answer!(self.check_conflicting_implementations(
                    origin,
                    module,
                    source,
                    symbol,
                    root,
                    ty,
                    &implements,
                )?);
                failures.extend(conflicts);
            }
            _ => {
                // require open implementations beside their interface
                for implementation in &implements {
                    let (_, interface) =
                        self.require_nominal_application(implementation.interface.ty)?;
                    let interface = interface.symbol;
                    if interface.module_id.package_id != package {
                        failures.push(ObligationFailure::ForeignBlanketImplementation {
                            source,
                            interface,
                        });
                    }
                }
            }
        }

        let check = ObligationCheck::from_failures(failures);

        Ok(Answer::Ready(check))
    }

    /// Return whether an exported extension needs a source-level name.
    fn is_unnamed_exported_nonlocal_extension(
        &self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        form: dir::ExtensionForm,
        target: dir::ExtensionTarget,
    ) -> bool {
        if form != dir::ExtensionForm::Exported {
            return false;
        }

        // accept a blanket extension, its bound interface names it
        if matches!(self.ty(target.r#type()), Ok(dir::Type::Parameter(_))) {
            return false;
        }

        let target_is_local = target.root().is_some_and(|root| root.module_id == module);
        if target_is_local {
            return false;
        }

        self.binding_table(symbol.module_id)
            .get_symbol(symbol.local_id)
            .key
            .is_none()
    }

    /// Decide whether declared members conform to one applied interface.
    fn conform_declared_implementation(
        &mut self,
        origin: Origin,
        implementation: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        members: &[dir::DefinitionMember],
        interface_module: ModuleId,
        interface: &dir::GenericApplication,
    ) -> CompilerResult<Answer<bool>> {
        let Some(definition @ dir::Definition::Interface(_)) = self.definition(interface.symbol)?
        else {
            return Err(CompilerError::Internal {
                message: format!(
                    "implementation target has no interface definition: {:?}",
                    interface.symbol,
                ),
            });
        };
        let interface_members = definition.members().to_vec();

        // resolve this projections through the declared associated types
        let instantiation = TypeSubstitution::default().with_receiver(target);
        let Some((_, receiver)) = answer!(self.instantiate_implemented_interface(
            origin,
            implementation,
            target,
            members,
            &instantiation,
            interface_module,
            interface,
            &interface_members,
        )?) else {
            return Ok(Answer::Ready(false));
        };
        let interface_substitution =
            self.qualified_instance_substitution(interface_module, interface, receiver)?;
        let requirements = answer!(self.interface_requirements_with_substitution(
            origin,
            interface.symbol,
            &interface_substitution,
        )?);
        let substitution = TypeSubstitution::default().with_receiver(receiver);

        // any declared overload may satisfy each named requirement
        for requirement in requirements.members {
            let mut candidates = SmallVec::<[_; 2]>::new();
            for member in members {
                let member = match self.body().declared_member(member)? {
                    Answer::Ready(Some(member)) => member,
                    Answer::Ready(None) => continue,
                    Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
                };
                if member.matches(requirement.space, requirement.key) {
                    candidates.push(member);
                }
            }
            if candidates.is_empty() {
                if requirement.has_default || requirement.is_optional {
                    continue;
                }

                // satisfy undeclared requirements from the target's inherent members
                let inherent =
                    answer!(self.inherent_member_candidates(origin, target, &requirement)?);
                let Some(required) = requirement.ty else {
                    if inherent.is_empty() {
                        return Ok(Answer::Ready(false));
                    }

                    continue;
                };

                let mut satisfied = false;
                for candidate in &inherent {
                    let found = candidate.callable.unwrap_or(candidate.access_type);
                    let decision = self.decide_member_relation(
                        origin,
                        Relation::Assignable,
                        requirement.role,
                        found,
                        required,
                        substitution.receiver,
                    )?;
                    if answer!(decision) {
                        satisfied = true;
                        break;
                    }
                }
                if !satisfied {
                    return Ok(Answer::Ready(false));
                }

                continue;
            }

            // accept an abstract associated requirement by presence
            let Some(required) = requirement.ty else {
                continue;
            };

            let mut satisfied = false;
            for candidate in candidates {
                let Some(found) = candidate.ty else {
                    continue;
                };
                let found = self.substitute_type(found, &substitution)?;
                let decision = self.decide_member_relation(
                    origin,
                    Relation::Assignable,
                    requirement.role,
                    found,
                    required,
                    substitution.receiver,
                )?;
                if answer!(decision) {
                    satisfied = true;
                    break;
                }
            }
            if !satisfied {
                return Ok(Answer::Ready(false));
            }
        }

        // inherited interfaces conform through the same declared members
        for application in requirements.inherited {
            let (application_module, application_instance) =
                self.require_nominal_application(application.ty)?;
            let inherited = answer!(self.conform_declared_implementation(
                origin,
                application.ty,
                target,
                members,
                application_module,
                &application_instance,
            )?);
            if !inherited {
                return Ok(Answer::Ready(false));
            }
        }

        Ok(Answer::Ready(true))
    }

    /// Select one declaration's implementation of an applied interface.
    pub(in crate::check) fn select_interface_implementation(
        &mut self,
        origin: Origin,
        implementer: dir::GlobalSymbolId,
        source: dir::GlobalNodeIdAny,
        declared_interface: dir::GlobalTypeId,
        implementer_type: dir::GlobalTypeId,
        members: &[dir::DefinitionMember],
        instantiation: &TypeSubstitution,
        interface_module: ModuleId,
        interface: &dir::GenericApplication,
    ) -> CompilerResult<Answer<Result<dir::InterfaceImplementation, ObligationFailure>>> {
        let Some(definition @ dir::Definition::Interface(_)) = self.definition(interface.symbol)?
        else {
            return Err(CompilerError::Internal {
                message: format!(
                    "implementation target has no interface definition: {:?}",
                    interface.symbol,
                ),
            });
        };
        let interface_members = definition.members().to_vec();

        // instantiate the implementation's associated types
        let Some((implementation, receiver)) = answer!(self.instantiate_implemented_interface(
            origin,
            declared_interface,
            implementer_type,
            members,
            instantiation,
            interface_module,
            interface,
            &interface_members,
        )?) else {
            let failure = ObligationFailure::InterfaceNotImplemented {
                source,
                ty: implementer_type,
                interface: declared_interface,
            };

            return Ok(Answer::Ready(Err(failure)));
        };

        let interface_substitution =
            self.qualified_instance_substitution(interface_module, interface, receiver)?;
        let requirements = answer!(self.interface_requirements_with_substitution(
            origin,
            interface.symbol,
            &interface_substitution,
        )?);
        let mut selected = Vec::new();

        // select exact declarations for every named requirement
        for interface_member in requirements.members {
            let candidates = answer!(self.implementation_member_candidates(
                origin,
                implementer,
                implementer_type,
                members,
                &interface_member,
            )?);
            if candidates.is_empty() {
                // accept a defaulted member as its own implementation
                if interface_member.has_default {
                    selected.push(dir::InterfaceMemberImplementation {
                        requirement: interface_member.source,
                        declarations: vec![interface_member.source],
                    });
                    continue;
                }
                if interface_member.is_optional {
                    continue;
                }

                let failure = ObligationFailure::InterfaceNotImplemented {
                    source,
                    ty: implementer_type,
                    interface: declared_interface,
                };

                return Ok(Answer::Ready(Err(failure)));
            }

            // overloads select in declaration order, first conforming wins
            let mut chosen = None;
            if candidates.len() > 1
                && let Some(required) = interface_member.ty
            {
                for candidate in &candidates {
                    let found = candidate.callable.unwrap_or(candidate.access_type);
                    let conforms = self.decide_member_relation(
                        origin,
                        Relation::Assignable,
                        interface_member.role,
                        found,
                        required,
                        instantiation.receiver,
                    )?;
                    if answer!(conforms) {
                        chosen = Some(candidate.symbol);
                        break;
                    }
                }
            }

            let mut declarations = Vec::with_capacity(candidates.len());
            match chosen {
                Some(symbol) => declarations.push(self.symbol_source(symbol)?),
                None => {
                    for candidate in &candidates {
                        declarations.push(self.symbol_source(candidate.symbol)?);
                    }
                }
            }
            selected.push(dir::InterfaceMemberImplementation {
                requirement: interface_member.source,
                declarations,
            });
        }

        // check inherited interfaces against the same extension members
        for application in requirements.inherited {
            let (application_module, application_instance) =
                self.require_nominal_application(application.ty)?;
            let inherited = answer!(self.select_interface_implementation(
                origin,
                implementer,
                source,
                declared_interface,
                implementer_type,
                members,
                instantiation,
                application_module,
                &application_instance,
            )?);
            match inherited {
                Ok(inherited) => selected.extend(inherited.members),
                Err(failure) => return Ok(Answer::Ready(Err(failure))),
            }
        }

        Ok(Answer::Ready(Ok(dir::InterfaceImplementation {
            interface: dir::NominalHeritage {
                source,
                ty: implementation,
            },
            members: selected,
        })))
    }

    /// Return declarations visible to one explicit interface implementation.
    fn implementation_member_candidates(
        &mut self,
        origin: Origin,
        implementer: dir::GlobalSymbolId,
        target: dir::GlobalTypeId,
        members: &[dir::DefinitionMember],
        requirement: &InterfaceMember,
    ) -> CompilerResult<Answer<Vec<MemberCandidate>>> {
        match self.symbol_kind(implementer)? {
            dir::SymbolKind::Extension => {
                let mut declared = SmallVec::<[_; 2]>::new();
                for member in members {
                    let member = match self.body().declared_member(member)? {
                        Answer::Ready(Some(member)) => member,
                        Answer::Ready(None) => continue,
                        Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
                    };
                    if member.matches(requirement.space, requirement.key) {
                        declared.push(member);
                    }
                }
                let substitution = TypeSubstitution::default().with_receiver(target);
                let candidates = answer!(self.body().extension_member_candidates(
                    origin,
                    implementer,
                    &substitution,
                    &declared,
                )?);

                // satisfy undeclared requirements from the target's inherent members
                if candidates.is_empty() {
                    return self.inherent_member_candidates(origin, target, requirement);
                }

                Ok(Answer::Ready(candidates))
            }
            dir::SymbolKind::Struct | dir::SymbolKind::Class | dir::SymbolKind::Enum => {
                self.inherent_member_candidates(origin, target, requirement)
            }
            kind => Err(CompilerError::Internal {
                message: format!("{kind:?} symbol {implementer:?} cannot implement interfaces"),
            }),
        }
    }

    /// Return the target's inherent members matching one interface requirement.
    fn inherent_member_candidates(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
        requirement: &InterfaceMember,
    ) -> CompilerResult<Answer<Vec<MemberCandidate>>> {
        let lookup = answer!(self.body().lookup_inherent_member(
            origin,
            origin.module(),
            target,
            requirement.space,
            requirement.key,
        )?);
        let candidates = match lookup {
            MemberLookup::Missing => Vec::new(),
            MemberLookup::Found(candidates) => candidates,
            lookup @ MemberLookup::Intersection(_) => {
                lookup
                    .into_candidates()
                    .ok_or_else(|| CompilerError::Internal {
                        message: "nominal implementation has a structural member".into(),
                    })?
            }
            MemberLookup::Field(_) | MemberLookup::Union(_) => {
                return Err(CompilerError::Internal {
                    message: "nominal implementation has a structural member".into(),
                });
            }
        };

        Ok(Answer::Ready(candidates))
    }

    /// Instantiate one extension interface with its concrete associated types.
    fn instantiate_implemented_interface(
        &mut self,
        origin: Origin,
        implementation: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        members: &[dir::DefinitionMember],
        instantiation: &TypeSubstitution,
        interface_module: ModuleId,
        interface: &dir::GenericApplication,
        interface_members: &[dir::DefinitionMember],
    ) -> CompilerResult<Answer<Option<(dir::GlobalTypeId, dir::GlobalTypeId)>>> {
        let (interface_base, mut bindings) = self.refinement_bindings(implementation)?;
        let interface_substitution = self.instance_substitution(interface_module, interface)?;
        let receiver_substitution = TypeSubstitution::default().with_receiver(target);

        // select declarations and defaults for every associated type
        for member in interface_members {
            let dir::DefinitionMember::AssociatedType(associated) = member else {
                continue;
            };
            let written = bindings
                .iter()
                .find(|(key, _)| *key == associated.key)
                .map(|(_, value)| *value);
            let declared = members.iter().find_map(|member| match member {
                dir::DefinitionMember::AssociatedType(candidate)
                    if candidate.key == associated.key =>
                {
                    candidate.value
                }
                _ => None,
            });
            let declared = declared
                .map(|value| self.substitute_type(value, instantiation))
                .transpose()?
                .map(|value| self.substitute_type(value, &receiver_substitution))
                .transpose()?;

            // require both authored bindings to name one reduced type
            if let (Some(written), Some(declared)) = (written, declared)
                && let Answer::Ready(written) = self.reduce_type(origin, written)?
                && let Answer::Ready(reduced) = self.reduce_type(origin, declared)?
                && let Answer::Ready(equal) = self.decide_equal(origin, written, reduced)?
                && !equal
            {
                return Ok(Answer::Ready(None));
            }

            // explicit bindings override an interface default
            let value = match written.or(declared).or(associated.value) {
                Some(value) => value,
                None => continue,
            };
            let value = if written.is_none() && declared.is_none() {
                self.substitute_type(value, &interface_substitution)?
            } else {
                value
            };
            if let Some((_, current)) = bindings.iter_mut().find(|(key, _)| *key == associated.key)
            {
                *current = value;
            } else {
                bindings.push((associated.key, value));
            }
        }

        // resolve receiver-relative binding values through the complete receiver
        let receiver = self.intern_refinements(origin.module(), target, &bindings)?;
        let receiver_substitution = TypeSubstitution::default().with_receiver(receiver);
        for (_, value) in &mut bindings {
            *value = self.substitute_type(*value, &receiver_substitution)?;
        }
        let receiver = self.intern_refinements(origin.module(), target, &bindings)?;
        let implementation = self.intern_refinements(origin.module(), interface_base, &bindings)?;

        // require every concrete binding to satisfy its declared bound
        let interface_substitution =
            self.qualified_instance_substitution(interface_module, interface, receiver)?;
        for member in interface_members {
            let dir::DefinitionMember::AssociatedType(associated) = member else {
                continue;
            };
            let Some(constraint) = associated.constraint else {
                continue;
            };
            let Some((_, value)) = bindings.iter().find(|(key, _)| *key == associated.key) else {
                continue;
            };
            let constraint = self.substitute_type(constraint, &interface_substitution)?;
            if !answer!(self.decide_relation(origin, Relation::Satisfies, *value, constraint,)?) {
                return Ok(Answer::Ready(None));
            }
        }

        Ok(Answer::Ready(Some((implementation, receiver))))
    }

    /// Commit one declaration's selected interface implementations.
    fn commit_interface_implementations(
        &mut self,
        symbol: dir::GlobalSymbolId,
        selected: Vec<dir::InterfaceImplementation>,
    ) -> CompilerResult<()> {
        let Some(definition) = self.definition_mut(symbol) else {
            return Err(CompilerError::Internal {
                message: format!("implementer {symbol:?} lost its definition"),
            });
        };

        // replace each successful selection by its authored source
        for selected in selected {
            let source = selected.interface.source;
            let Some(implementation) = definition
                .implementations_mut()
                .iter_mut()
                .find(|implementation| implementation.interface.source == source)
            else {
                return Err(CompilerError::Internal {
                    message: format!("implementer {symbol:?} lost implementation {source:?}"),
                });
            };
            *implementation = selected;
        }

        Ok(())
    }

    /// Return one symbol's interface implementations as written.
    fn declared_implementations(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Vec<dir::InterfaceImplementation>> {
        // prefer the own module's still-unrefined declared row over a foreign definition
        if let Some(module) = self.module_maybe(symbol.module_id)
            && let Some(declared) = &module.declared
            && let Some(definition) = declared.definitions.definition(symbol)
        {
            return Ok(definition.implementations().to_vec());
        }

        Ok(self
            .definition(symbol)?
            .map(|definition| definition.implementations().to_vec())
            .unwrap_or_default())
    }

    /// Check visible implementations conflicting with one new extension.
    fn check_conflicting_implementations(
        &mut self,
        origin: Origin,
        module: ModuleId,
        source: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
        root: dir::GlobalSymbolId,
        ty: dir::GlobalTypeId,
        implementations: &[dir::InterfaceImplementation],
    ) -> CompilerResult<Answer<Vec<ObligationFailure>>> {
        let mut failures = Vec::new();

        // collect comparable implementations before overlap checks
        let mut candidates = SmallVec::<
            [(
                dir::GlobalSymbolId,
                dir::GlobalTypeId,
                dir::NominalHeritage,
                dir::NominalHeritage,
            ); 2],
        >::new();
        for other in self.body().visible_extensions(module, root)? {
            if other == symbol {
                continue;
            }
            if !self.is_later_definition(source, other) {
                continue;
            }
            let Some(dir::Definition::Extension(extension)) = self.definition(other)? else {
                continue;
            };
            let extension = extension.clone();
            let dir::ExtensionTarget::Rooted {
                root: other_root,
                ty: other_ty,
            } = extension.target
            else {
                continue;
            };
            if other_root != root {
                continue;
            }
            let other_implementations = self.declared_implementations(other)?;
            for other_implementation in &other_implementations {
                let (_, other_interface) =
                    self.require_nominal_application(other_implementation.interface.ty)?;
                let other_symbol = self.resolve_symbol_alias(other_interface.symbol)?;
                for implementation in implementations {
                    let (_, interface) =
                        self.require_nominal_application(implementation.interface.ty)?;
                    let symbol = self.resolve_symbol_alias(interface.symbol)?;
                    if symbol == other_symbol {
                        candidates.push((
                            other,
                            other_ty,
                            implementation.interface.clone(),
                            other_implementation.interface.clone(),
                        ));
                        break;
                    }
                }
            }
        }

        // reject overlapping receivers under one unifiable interface instantiation
        for (other, other_ty, heritage, other_heritage) in candidates {
            if !answer!(self.types_may_overlap(origin, ty, other_ty)?) {
                continue;
            }
            let (heritage_module, heritage_interface) =
                self.require_nominal_application(heritage.ty)?;
            let (other_module, other_interface) =
                self.require_nominal_application(other_heritage.ty)?;
            let heritage_arguments =
                self.filled_application_arguments(heritage_module, &heritage_interface)?;
            let other_arguments =
                self.filled_application_arguments(other_module, &other_interface)?;
            if heritage_arguments.len() == other_arguments.len() {
                let mut distinct = false;
                for (left, right) in heritage_arguments
                    .iter()
                    .copied()
                    .zip(other_arguments.iter().copied())
                {
                    if !answer!(self.types_may_overlap(origin, left, right)?) {
                        distinct = true;
                        break;
                    }
                }
                if distinct {
                    continue;
                }
            }

            failures.push(ObligationFailure::ConflictingImplementation {
                source,
                conflict: other,
                interface: heritage_interface.symbol,
                ty,
            });
        }

        Ok(Answer::Ready(failures))
    }

    /// Return whether `source` is later than one other local definition.
    fn is_later_definition(
        &self,
        source: dir::GlobalNodeIdAny,
        other: dir::GlobalSymbolId,
    ) -> bool {
        let Some(state) = self.module_maybe(other.module_id) else {
            return true;
        };
        let Some(other_source) = state.definitions.definition_source_maybe(other) else {
            return true;
        };
        if other_source.module_id != source.module_id {
            return true;
        }

        source.local_id.id > other_source.local_id.id
    }
}

impl BodyState<'_, '_> {
    /// Reject duplicate property slots across visible extensions.
    pub(in crate::check) fn check_duplicate_extension_members(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<Answer<ObligationCheck>> {
        // collect the extensions this module declares
        let declared = {
            let state = self.check.module(module);
            let Some(declared) = &state.declared else {
                return Ok(Answer::Ready(ObligationCheck::holds()));
            };
            declared
                .definitions
                .iter_definitions()
                .filter_map(|(symbol, definition)| match definition {
                    dir::Definition::Extension(extension) => Some((symbol, extension.clone())),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };

        for (extension_symbol, extension) in declared {
            let origin = Origin::Symbol(extension_symbol);

            let slots = answer!(self.property_slots(origin, &extension.members)?);
            if slots.is_empty() {
                continue;
            }
            let target = self.format_type(extension.target.r#type());

            // gather competitors sharing the target root or ground head
            let root = extension.target.root();
            let competitors = match root {
                Some(root) => self.visible_extensions(module, root)?,
                None => self.visible_blanket_extensions(module)?,
            };
            let ground = match root {
                Some(_) => None,
                None => match self.ty(extension.target.r#type())? {
                    dir::Type::Primitive(primitive) => Some(primitive),
                    // skip parameterized blankets, use sites judge their overlap
                    _ => continue,
                },
            };

            for competitor_symbol in competitors {
                // leave same-module collisions to source order
                if competitor_symbol == extension_symbol || competitor_symbol.module_id == module {
                    continue;
                }
                let Some(dir::Definition::Extension(competitor)) =
                    self.definition(competitor_symbol)?
                else {
                    continue;
                };
                let competes = match root {
                    Some(root) => competitor.target.root() == Some(root),
                    None => competitor.target.is_blanket(),
                };
                if !competes {
                    continue;
                }
                let competitor_target = competitor.target.r#type();
                let members = competitor.members.clone();
                if ground.is_some()
                    && !matches!(
                        self.ty(competitor_target)?,
                        dir::Type::Primitive(primitive) if Some(primitive) == ground
                    )
                {
                    continue;
                }

                let other = answer!(self.property_slots(origin, &members)?);
                for slot in &slots {
                    let duplicated = other.iter().any(|candidate| {
                        candidate.key == slot.key
                            && candidate.space == slot.space
                            && candidate.form == slot.form
                            && ((candidate.reads && slot.reads)
                                || (candidate.writes && slot.writes))
                    });
                    if duplicated {
                        self.check.report_duplicate_extension_member(
                            slot.source,
                            &slot.key,
                            target.clone(),
                        );
                    }
                }
            }
        }

        Ok(Answer::Ready(ObligationCheck::holds()))
    }

    /// Collect the property slot surface of one extension declaration.
    fn property_slots(
        &mut self,
        origin: Origin,
        members: &[dir::DefinitionMember],
    ) -> CompilerResult<Answer<Vec<PropertySlot>>> {
        let mut slots = Vec::new();
        for member in members {
            let (reads, writes) = match member {
                dir::DefinitionMember::Field(field) => (true, !field.is_readonly),
                dir::DefinitionMember::Method(method) => match method.role {
                    Some(dir::FunctionRole::Getter) => (true, false),
                    Some(dir::FunctionRole::Setter) => (false, true),
                    // methods union as overloads and never collide
                    _ => continue,
                },
                _ => continue,
            };
            let Some(key) = member.key() else {
                continue;
            };
            let Some(ty) = answer!(self.definition_member_type(member)?) else {
                continue;
            };
            let this = self
                .signature_head(ty)?
                .and_then(|signature| signature.this_parameter);
            let Some(form) = answer!(self.slot_receiver(origin, this)?) else {
                continue;
            };

            slots.push(PropertySlot {
                key,
                space: member.space(),
                reads,
                writes,
                form,
                source: member.source(),
            });
        }

        Ok(Answer::Ready(slots))
    }

    /// Return the comparable declared receiver of one property slot.
    fn slot_receiver(
        &mut self,
        origin: Origin,
        this: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<Option<SlotReceiver>>> {
        let Some(this) = this else {
            return Ok(Answer::Ready(Some(SlotReceiver::Default)));
        };
        let head = answer!(self.reduce_type_head(origin, this)?);
        let form = match self.ty(head)? {
            dir::Type::Form(form) => match form.form {
                // borrows compare by their access value
                dir::Form::Borrowed(borrow) => {
                    let borrow = self.check.type_borrow(head.module_id, borrow)?;
                    let access = answer!(self.reduce_type_head(origin, borrow.access)?);
                    let access = match self.ty(access)? {
                        dir::Type::Literal(dir::ScalarLiteral::String(name)) => Some(name),
                        _ => None,
                    };

                    Some(SlotReceiver::Borrowed(access))
                }
                dir::Form::Owned => Some(SlotReceiver::Owned),
                dir::Form::Raw => Some(SlotReceiver::Raw),
                _ => Some(SlotReceiver::Default),
            },
            // conversion receivers never collide with plain slots
            dir::Type::Application(_) => None,
            _ => Some(SlotReceiver::Default),
        };

        Ok(Answer::Ready(form))
    }
}

/// One single-slot property surface entry compared for duplicates.
struct PropertySlot {
    /// The member key.
    key: dir::StaticKey,
    /// The member space declaring the slot.
    space: dir::MemberSpace,
    /// Whether the slot serves reads.
    reads: bool,
    /// Whether the slot serves writes.
    writes: bool,
    /// The comparable declared receiver form.
    form: SlotReceiver,
    /// The declaring member source node.
    source: dir::GlobalNodeIdAny,
}

/// The comparable declared receiver of one property slot.
#[derive(PartialEq)]
enum SlotReceiver {
    /// The family-default managed receiver.
    Default,
    /// A borrowed receiver compared by access.
    Borrowed(Option<dir::StringId>),
    /// An owned receiver.
    Owned,
    /// A raw pointer receiver.
    Raw,
}
