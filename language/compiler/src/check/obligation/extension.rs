use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    Answer, BodyState, CheckState, ExtensionCoherenceObligation, InterfaceMember, MemberCandidate,
    MemberLookup, ObligationCheck, ObligationFailure, Origin, Relation, TypeSubstitution, answer,
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
        let mut failures = Vec::new();

        // prove each declared implementation against the interface requirements
        for heritage in implementations {
            let conforms = answer!(self.conform_declared_implementation(
                origin,
                heritage.ty,
                target,
                &members,
            )?);
            if !conforms {
                failures.push(ObligationFailure::InterfaceNotImplemented {
                    source: heritage.source,
                    ty: target,
                    interface: heritage.ty,
                });
            }
        }

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
                    let (_, interface) = self.nominal_application(implementation.ty)?;
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
                    let (_, interface) = self.nominal_application(implementation.ty)?;
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
    ) -> CompilerResult<Answer<bool>> {
        // resolve associated projections through the declared implementation
        let instantiation = TypeSubstitution::default().with_receiver(target);
        let Some(implementation) = answer!(self.instantiate_interface_implementation(
            origin,
            implementation,
            target,
            members,
            &instantiation,
        )?) else {
            return Ok(Answer::Ready(false));
        };
        // collect the interface requirements at this implementation
        let requirements = answer!(self.interface_requirements(origin, implementation, target)?);
        let substitution = TypeSubstitution::default().with_receiver(target);

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
            let inherited = answer!(self.conform_declared_implementation(
                origin,
                application.ty,
                target,
                members,
            )?);
            if !inherited {
                return Ok(Answer::Ready(false));
            }
        }

        Ok(Answer::Ready(true))
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

    /// Instantiate one interface implementation with its concrete associated types.
    pub(in crate::check) fn instantiate_interface_implementation(
        &mut self,
        origin: Origin,
        declared_interface: dir::GlobalTypeId,
        implementer: dir::GlobalTypeId,
        implementation_members: &[dir::DefinitionMember],
        instantiation: &TypeSubstitution,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        // read the declared interface and its refinements
        let (mut interface_base, mut bindings) = self.refinement_bindings(declared_interface)?;
        let dir::Type::Application(interface) = self.ty(interface_base)? else {
            return Err(CompilerError::Internal {
                message: format!("implemented interface {declared_interface:?} has no application"),
            });
        };

        // fill elided interface arguments
        if let Some(filled) = self.fill_elided_application(interface_base.module_id, &interface)? {
            interface_base = filled;
        }
        let dir::Type::Application(interface) = self.ty(interface_base)? else {
            return Err(CompilerError::Internal {
                message: format!("filled interface {interface_base:?} has no application"),
            });
        };

        // read the interface's declared members
        let Some(dir::Definition::Interface(definition)) = self.definition(interface.symbol)?
        else {
            return Err(CompilerError::Internal {
                message: format!(
                    "implementation target has no interface definition: {:?}",
                    interface.symbol,
                ),
            });
        };
        let interface_members = definition.members.clone();

        // select declarations and defaults for every associated type
        for member in &interface_members {
            let dir::DefinitionMember::AssociatedType(associated) = member else {
                continue;
            };
            let refined = bindings
                .iter()
                .find(|(key, _)| *key == associated.key)
                .map(|(_, value)| *value);
            let declared = implementation_members
                .iter()
                .find_map(|member| match member {
                    dir::DefinitionMember::AssociatedType(candidate)
                        if candidate.key == associated.key =>
                    {
                        candidate.value
                    }
                    _ => None,
                });
            let declared = declared
                .map(|value| self.substitute_type(value, instantiation))
                .transpose()?;

            // require both authored bindings to name one reduced type
            if let (Some(refined), Some(declared)) = (refined, declared) {
                let refined = answer!(self.reduce_type(origin, refined)?);
                let declared = answer!(self.reduce_type(origin, declared)?);
                if !answer!(self.decide_equal(origin, refined, declared)?) {
                    return Ok(Answer::Ready(None));
                }
            }

            // explicit bindings override an interface default
            let value = match refined.or(declared).or(associated.value) {
                Some(value) => value,
                None => continue,
            };
            if let Some((_, current)) = bindings.iter_mut().find(|(key, _)| *key == associated.key)
            {
                *current = value;
            } else {
                bindings.push((associated.key, value));
            }
        }

        // normalize every selected value through the complete implementation
        let implementation = self.intern_refinements(origin.module(), interface_base, &bindings)?;
        for (_, value) in &mut bindings {
            *value = answer!(self.instantiate_interface_type(
                origin,
                *value,
                implementation,
                implementer,
            )?);
        }
        let implementation = self.intern_refinements(origin.module(), interface_base, &bindings)?;

        // require every concrete binding to satisfy its declared bound
        for member in &interface_members {
            let dir::DefinitionMember::AssociatedType(associated) = member else {
                continue;
            };
            let Some(constraint) = associated.constraint else {
                continue;
            };
            let Some((_, value)) = bindings.iter().find(|(key, _)| *key == associated.key) else {
                continue;
            };
            let constraint = answer!(self.instantiate_interface_type(
                origin,
                constraint,
                implementation,
                implementer,
            )?);
            if !answer!(self.decide_relation(origin, Relation::Satisfies, *value, constraint,)?) {
                return Ok(Answer::Ready(None));
            }
        }

        Ok(Answer::Ready(Some(implementation)))
    }

    /// Return one symbol's interface implementations as written.
    fn declared_implementations(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Vec<dir::NominalHeritage>> {
        // prefer the own module's authored implementations over a foreign definition
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
        implementations: &[dir::NominalHeritage],
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
                let (_, other_interface) = self.nominal_application(other_implementation.ty)?;
                let other_symbol = self.resolve_symbol_alias(other_interface.symbol)?;
                for implementation in implementations {
                    let (_, interface) = self.nominal_application(implementation.ty)?;
                    let symbol = self.resolve_symbol_alias(interface.symbol)?;
                    if symbol == other_symbol {
                        candidates.push((
                            other,
                            other_ty,
                            implementation.clone(),
                            other_implementation.clone(),
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
            let (heritage_module, heritage_interface) = self.nominal_application(heritage.ty)?;
            let (other_module, other_interface) = self.nominal_application(other_heritage.ty)?;
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
    /// Check one extension against other visible extensions' properties.
    pub(in crate::check) fn check_extension_coherence(
        &mut self,
        obligation: &ExtensionCoherenceObligation,
    ) -> CompilerResult<Answer<ObligationCheck>> {
        let extension_symbol = obligation.symbol;
        let module = extension_symbol.module_id;
        let Some(dir::Definition::Extension(extension)) = self.definition(extension_symbol)? else {
            return Ok(Answer::Ready(ObligationCheck::holds()));
        };
        let extension = extension.clone();
        let mut failures = Vec::new();

        {
            let origin = Origin::Symbol(extension_symbol);

            let declared = answer!(self.property_members(origin, &extension.members)?);
            if declared.is_empty() {
                return Ok(Answer::Ready(ObligationCheck::holds()));
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
                    // leave parameterized blanket overlap to use sites
                    _ => return Ok(Answer::Ready(ObligationCheck::holds())),
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

                let other = answer!(self.property_members(origin, &members)?);
                for member in &declared {
                    let duplicated = other.iter().any(|candidate| {
                        candidate.key == member.key
                            && candidate.space == member.space
                            && candidate.form == member.form
                            && ((candidate.reads && member.reads)
                                || (candidate.writes && member.writes))
                    });
                    if duplicated {
                        failures.push(ObligationFailure::DuplicateExtensionMember {
                            source: member.source,
                            member: member.key,
                            target: target.clone(),
                        });
                    }
                }
            }
        }

        Ok(Answer::Ready(ObligationCheck::from_failures(failures)))
    }

    /// Collect the property members one extension declares.
    fn property_members(
        &mut self,
        origin: Origin,
        members: &[dir::DefinitionMember],
    ) -> CompilerResult<Answer<Vec<PropertyMember>>> {
        let mut properties = Vec::new();
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
            let Some(form) = answer!(self.property_receiver(origin, this)?) else {
                continue;
            };

            properties.push(PropertyMember {
                key,
                space: member.space(),
                reads,
                writes,
                form,
                source: member.source(),
            });
        }

        Ok(Answer::Ready(properties))
    }

    /// Return the comparable declared receiver of one property member.
    fn property_receiver(
        &mut self,
        origin: Origin,
        this: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<Option<PropertyReceiver>>> {
        let Some(this) = this else {
            return Ok(Answer::Ready(Some(PropertyReceiver::Default)));
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

                    Some(PropertyReceiver::Borrowed(access))
                }
                dir::Form::Owned => Some(PropertyReceiver::Owned),
                dir::Form::Raw => Some(PropertyReceiver::Raw),
                _ => Some(PropertyReceiver::Default),
            },
            // conversion receivers never collide with plain slots
            dir::Type::Application(_) => None,
            _ => Some(PropertyReceiver::Default),
        };

        Ok(Answer::Ready(form))
    }
}

/// One exclusive property member compared for duplicates.
struct PropertyMember {
    /// The member key.
    key: dir::StaticKey,
    /// The member space declaring the property.
    space: dir::MemberSpace,
    /// Whether the property serves reads.
    reads: bool,
    /// Whether the property serves writes.
    writes: bool,
    /// The comparable declared receiver form.
    form: PropertyReceiver,
    /// The declaring member source node.
    source: dir::GlobalNodeIdAny,
}

/// The comparable declared receiver of one property member.
#[derive(PartialEq)]
enum PropertyReceiver {
    /// The family-default managed receiver.
    Default,
    /// A borrowed receiver compared by access.
    Borrowed(Option<dir::StringId>),
    /// An owned receiver.
    Owned,
    /// A raw pointer receiver.
    Raw,
}
