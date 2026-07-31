use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    Answer, CheckState, InterfaceMember, MemberCandidate, MemberLookup, ObligationCheck,
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
        // skip conformance while declaring, checking validates every implements row
        if self.is_declaration() {
            return Ok(Answer::Ready(ObligationCheck::Holds));
        }

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
        let implements = extension
            .implements
            .iter()
            .cloned()
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

                return Ok(Answer::Ready(false));
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

                Ok(Answer::Ready(candidates))
            }
            dir::SymbolKind::Struct | dir::SymbolKind::Class | dir::SymbolKind::Enum => {
                let lookup = answer!(self.body().lookup_inherent_member(
                    origin,
                    origin.module(),
                    target,
                    requirement.space,
                    requirement.key,
                )?);
                let candidates =
                    match lookup {
                        MemberLookup::Missing => Vec::new(),
                        MemberLookup::Found(candidates) => candidates,
                        lookup @ MemberLookup::Intersection(_) => lookup
                            .into_candidates()
                            .ok_or_else(|| CompilerError::Internal {
                                message: "nominal implementation has a structural member".into(),
                            })?,
                        MemberLookup::Field(_) | MemberLookup::Union(_) => {
                            return Err(CompilerError::Internal {
                                message: "nominal implementation has a structural member".into(),
                            });
                        }
                    };

                Ok(Answer::Ready(candidates))
            }
            kind => Err(CompilerError::Internal {
                message: format!("{kind:?} symbol {implementer:?} cannot implement interfaces"),
            }),
        }
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
        let state = self.module_mut(symbol.module_id);
        let Some(definition) = state.definitions.definition_mut(symbol) else {
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
            for other_implementation in &extension.implements {
                let (_, other_interface) =
                    self.require_nominal_application(other_implementation.interface.ty)?;
                for implementation in implementations {
                    let (_, interface) =
                        self.require_nominal_application(implementation.interface.ty)?;
                    if interface.symbol == other_interface.symbol {
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
            let heritage_arguments = self
                .type_ids(heritage_module, heritage_interface.arguments)?
                .to_vec();
            let other_arguments = self
                .type_ids(other_module, other_interface.arguments)?
                .to_vec();
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
        let other_source = state.definitions.definition_source(other);
        if other_source.module_id != source.module_id {
            return true;
        }

        source.local_id.id > other_source.local_id.id
    }
}
