use destack_dir as dir;
use destack_dir::MemberRole;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{
    BodyState, CandidateOutcome, ExtensionMatch, MemberLookup, OpenBounds, Origin, Relation,
    TypeSubstitution, UnboundParameters, Verdict,
};
use crate::{CompilerError, CompilerResult};

impl BodyState<'_, '_> {
    /// Return the use-site type projected by one declaration member.
    pub(in crate::sema) fn projected_member_type(
        &mut self,
        origin: Origin,
        receiver: Option<dir::GlobalTypeId>,
        role: MemberRole,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // project field types alone
        if role != MemberRole::Field {
            return Ok(ty);
        }

        // keep the declared type where the site names no receiver
        let Some(receiver) = receiver else {
            return Ok(ty);
        };

        // resolve the field in the receiver's place
        let ty = self.project_member_place(origin, receiver, ty)?;

        // readonly receivers project deep readonly views onto stored fields
        if !self.is_readonly_receiver_projection(receiver)? {
            return Ok(ty);
        }

        // read the projected field through its solution
        let ty = self.shallow_resolve(ty)?;
        if matches!(self.ty(ty)?, dir::Type::Form(form) if form.form == dir::Form::Readonly) {
            return Ok(ty);
        }
        if !self.is_readonly_type_projection(origin, ty)? {
            return Ok(ty);
        }

        // wrap the projected field in the readonly view
        let projected = self.intern_type(dir::Type::Form(dir::FormType {
            form: dir::Form::Readonly,
            value: ty,
        }))?;

        self.normalize(origin, projected)
    }

    /// Return whether one receiver projects stored fields as readonly.
    pub(in crate::sema) fn is_readonly_receiver_projection(
        &mut self,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let mut current = receiver;

        loop {
            let form = match self.ty(current)? {
                dir::Type::Form(form) => form,
                _ => return Ok(false),
            };

            // explicit readonly views make every stored field readonly
            if form.form == dir::Form::Readonly {
                return Ok(true);
            }

            // readonly borrows expose only readonly stored fields
            if let dir::Form::Borrowed(borrow) = form.form
                && let access = self.type_borrow(current.module_id, borrow)?.access
                && self.is_readonly_access(access)?
            {
                return Ok(true);
            }

            current = form.value;
        }
    }

    /// Project one type-level member access through its owner.
    pub(in crate::sema) fn project_member(
        &mut self,
        origin: Origin,
        member: &dir::MemberType,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // read the written refinement bindings of a qualified projection once
        let refinement = match member.qualifier {
            Some(qualifier) => Some(self.refinements(qualifier)?),
            None => None,
        };

        // bind the projection from written refinements
        if let Some((_, bindings)) = &refinement
            && let Some((_, value)) = bindings.iter().find(|(key, _)| *key == member.key)
        {
            return Ok(Some(*value));
        }

        // select the implementation for applied interface scopes
        if let Some((base, _)) = refinement
            && !self.is_rigid_projection_owner(member.owner)?
        {
            return match self.ty(base)? {
                // project the applied interface's selected implementation
                dir::Type::Application(_) => self.project_selected_member(origin, member, base),
                // project the lexical extension scope's own associated member
                dir::Type::Reference(reference) => {
                    self.project_scope_member(origin, member, reference.symbol)
                }
                // every other qualifier projects nothing
                _ => Ok(None),
            };
        }

        // resolve remaining projections through member lookup
        let module = self.module_id;
        let subject =
            self.member_subject(origin, member.owner, member.owner, dir::MemberSpace::Static)?;
        let lookup = self.lookup_member(origin, module, subject, member.key)?;
        let projected = match lookup {
            // fall back to the qualifying interface's declared default
            MemberLookup::Missing => self.project_default_member(origin, member)?,
            // project whatever the lookup found
            found => self.project_member_lookup(module, member, found)?,
        };

        Ok(projected)
    }

    /// Project one associated member through its selected interface implementation.
    fn project_selected_member(
        &mut self,
        origin: Origin,
        member: &dir::MemberType,
        qualifier: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // read the qualifying interface application
        let module = self.module_id;
        let (interface_module, interface) = self.nominal_application(qualifier)?;
        let owner = member.owner;

        // project a declaring class scope's own associated member
        let scope = interface.symbol;
        if let Some(definition) = self.definition(scope)?
            && !matches!(definition, dir::Definition::Interface(_))
        {
            let members = definition.members().to_vec();
            let substitution =
                self.qualified_instance_substitution(interface_module, &interface, owner)?;

            return self.project_declared_associated_member(
                origin,
                member,
                &members,
                &substitution,
            );
        }

        // enumerate candidate extensions by receiver family
        let apparent = self.intern_apparent_type(owner)?;
        let extensions =
            self.visible_implementation_extensions(origin, module, apparent, interface.symbol)?;
        for extension_symbol in extensions {
            let Some(dir::Definition::Extension(extension)) = self.definition(extension_symbol)?
            else {
                continue;
            };

            if !extension.is_visible_from(module) {
                continue;
            }

            // read the extension target, interfaces, and members
            let target_type = extension.target.r#type();
            let interfaces = extension
                .implements
                .iter()
                .map(|conformance| conformance.interface)
                .collect::<SmallVec<[_; 2]>>();
            let members = extension.members.clone();
            let template = self.symbol_template(extension_symbol)?;

            // classify the candidate's bounds in a probe before committing:
            //  a rejected match must roll its inference bindings back, so the
            //  probe runs first and only a holding candidate reruns to commit
            let verdict = self.probe_candidate(|state| {
                let matched = state.match_extension_implementation(
                    origin,
                    Relation::Assignable,
                    interface_module,
                    owner,
                    owner,
                    &interface,
                    template,
                    target_type,
                    &interfaces,
                    OpenBounds::Probe,
                )?;

                Ok(match matched {
                    ExtensionMatch::Matched(..) => CandidateOutcome::Accepted(()),
                    // unsatisfied bounds leave the candidate unselected here
                    ExtensionMatch::Unmatched | ExtensionMatch::Unproven => {
                        CandidateOutcome::Rejected(())
                    }
                })
            })?;

            if !matches!(verdict, Verdict::Holds) {
                continue;
            }

            // rerun the match to commit its substitution
            let matched = self.match_extension_implementation(
                origin,
                Relation::Assignable,
                interface_module,
                owner,
                owner,
                &interface,
                template,
                target_type,
                &interfaces,
                OpenBounds::Probe,
            )?;
            let ExtensionMatch::Matched(substitution, _) = matched else {
                continue;
            };

            // project the extension's own declared value, falling back to the interface default
            let projected =
                self.project_declared_associated_member(origin, member, &members, &substitution)?;
            if projected.is_some() {
                return Ok(projected);
            }

            return self.project_qualified_default(origin, member, qualifier);
        }

        // match the owner's own declared implementations
        if let Some((application_module, application)) = self.nominal_application_maybe(owner)?
            && let Some(definition) = self.definition(application.symbol)?
        {
            let interfaces = definition
                .implementations()
                .iter()
                .map(|conformance| conformance.interface)
                .collect::<SmallVec<[_; 2]>>();
            let members = definition.members().to_vec();

            if !interfaces.is_empty() {
                let mut substitution =
                    self.instance_substitution(application_module, &application)?;
                let matched = self.match_implemented_interface(
                    origin,
                    Relation::Assignable,
                    interface_module,
                    &[],
                    &mut substitution,
                    &interfaces,
                    &interface,
                )?;
                if matched.is_some() {
                    // project the owner's own declared value, falling back to the interface default
                    let projected = self.project_declared_associated_member(
                        origin,
                        member,
                        &members,
                        &substitution,
                    )?;
                    if projected.is_some() {
                        return Ok(projected);
                    }

                    return self.project_qualified_default(origin, member, qualifier);
                }
            }
        }

        Ok(None)
    }

    /// Project one associated member declared by a lexical extension scope.
    fn project_scope_member(
        &mut self,
        origin: Origin,
        member: &dir::MemberType,
        scope: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let Some(definition) = self.definition(scope)?.cloned() else {
            return Err(CompilerError::Internal {
                message: format!("associated type scope {scope:?} has no definition"),
            });
        };

        // bind an extension scope's parameters through its matched target
        let substitution = match &definition {
            dir::Definition::Extension(extension) => {
                let template = self.symbol_template(scope)?;
                let target = extension.target.r#type();
                let owner = self.strip_form(origin, member.owner)?;
                let matched = self.match_extension_subject(
                    origin,
                    owner,
                    owner,
                    template,
                    target,
                    UnboundParameters::Open,
                )?;
                let Some(matched) = matched else {
                    return Ok(None);
                };

                matched.with_receiver(owner)
            }
            _ => TypeSubstitution::default().with_receiver(member.owner),
        };

        // project the scope's own members against the written owner
        let members = definition.members().to_vec();

        self.project_declared_associated_member(origin, member, &members, &substitution)
    }

    /// Project one associated type declared by a selected scope.
    fn project_declared_associated_member(
        &mut self,
        origin: Origin,
        member: &dir::MemberType,
        members: &[dir::DefinitionMember],
        substitution: &TypeSubstitution,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // select the scope's declared value for this key
        let declared = members.iter().find_map(|declared| match declared {
            dir::DefinitionMember::AssociatedType(associated) if associated.key == member.key => {
                associated.value.map(|value| (associated, value))
            }
            _ => None,
        });

        // apply the implementation's arguments to the declared value
        if let Some((declared, value)) = declared {
            let value = self.project_associated_value(
                origin.module(),
                member,
                declared.symbol,
                value,
                substitution,
            )?;

            return Ok(Some(value));
        }

        Ok(None)
    }

    /// Project one associated type default declared by the qualifying interface.
    fn project_qualified_default(
        &mut self,
        origin: Origin,
        member: &dir::MemberType,
        qualifier: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // read the interface's own members under its applied arguments
        let (interface_module, interface) = self.nominal_application(qualifier)?;
        let scope = self.resolve_symbol_alias(interface.symbol)?;
        let Some(definition) = self.definition(scope)? else {
            return Ok(None);
        };

        let members = definition.members().to_vec();
        let substitution =
            self.qualified_instance_substitution(interface_module, &interface, member.owner)?;

        self.project_declared_associated_member(origin, member, &members, &substitution)
    }

    /// Return the type projected by one selected member lookup.
    fn project_member_lookup(
        &mut self,
        module: ModuleId,
        member: &dir::MemberType,
        lookup: MemberLookup,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        match lookup {
            MemberLookup::Field(field) => field.read_type(self),
            MemberLookup::Found(candidates) => match candidates.as_slice() {
                [candidate] => {
                    // keep nominal singleton identity for variant values
                    if candidate.role == MemberRole::VariantValue {
                        return candidate.read_type(self);
                    }

                    // project associated values through their applied arguments
                    if candidate.value_type.is_some() {
                        let written = self.static_value(candidate.symbol).ok_or_else(|| {
                            CompilerError::Internal {
                                message: format!(
                                    "associated member {:?} lost its declared value",
                                    candidate.symbol,
                                ),
                            }
                        })?;
                        let substitution = TypeSubstitution {
                            bindings: candidate.generic_arguments.iter().copied().collect(),
                            receiver: Some(member.owner),
                        };
                        let written = self.project_associated_value(
                            module,
                            member,
                            candidate.symbol,
                            written,
                            &substitution,
                        )?;

                        return Ok(Some(written));
                    }

                    // static values project as their own singleton
                    if let Some(value) = candidate.value {
                        let ty = self.intern_type(dir::Type::Static(value))?;

                        return Ok(Some(ty));
                    }

                    // keep a member projecting back onto itself symbolic
                    if let Some(projected) = self.member_head(candidate.access_type)?
                        && projected.owner == member.owner
                        && projected.key == member.key
                    {
                        return Ok(None);
                    }

                    candidate.read_type(self)
                }
                // zero or several candidates project no single type
                [] | [_, _, ..] => Ok(None),
            },
            MemberLookup::Union(lookups) => {
                let mut types = Vec::with_capacity(lookups.len());
                for arm in lookups {
                    let Some(ty) = self.project_member_lookup(module, member, arm.lookup)? else {
                        return Ok(None);
                    };
                    types.push(ty);
                }

                self.normalized_union_type(types).map(Some)
            }
            MemberLookup::Intersection(lookups) => {
                let mut types = Vec::with_capacity(lookups.len());
                for lookup in lookups {
                    let Some(ty) = self.project_member_lookup(module, member, lookup)? else {
                        return Ok(None);
                    };
                    types.push(ty);
                }

                self.normalized_intersection_type(types).map(Some)
            }
            MemberLookup::Missing | MemberLookup::Ambiguous => Ok(None),
        }
    }

    /// Project one interface default through a qualified owner.
    fn project_default_member(
        &mut self,
        origin: Origin,
        member: &dir::MemberType,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        if self.is_rigid_projection_owner(member.owner)? {
            return Ok(None);
        }

        // read the interface holding the default from a qualified projection only
        let Some(qualifier) = member.qualifier else {
            return Ok(None);
        };
        let Some((interface_module, interface)) = self.nominal_application_maybe(qualifier)? else {
            return Ok(None);
        };

        // select the declared interface default
        let associated = match self.definition(interface.symbol)? {
            Some(dir::Definition::Interface(definition)) => {
                definition
                    .members
                    .iter()
                    .find_map(|declared| match declared {
                        dir::DefinitionMember::AssociatedType(associated)
                            if associated.key == member.key =>
                        {
                            associated.value.map(|value| (associated.symbol, value))
                        }
                        _ => None,
                    })
            }
            _ => None,
        };
        let Some((symbol, value)) = associated else {
            return Ok(None);
        };

        // apply the interface's arguments to the default
        let substitution =
            self.qualified_instance_substitution(interface_module, &interface, member.owner)?;
        let value =
            self.project_associated_value(origin.module(), member, symbol, value, &substitution)?;

        Ok(Some(value))
    }

    /// Project one associated value through its owner and applied arguments.
    fn project_associated_value(
        &mut self,
        module: ModuleId,
        member: &dir::MemberType,
        symbol: dir::GlobalSymbolId,
        value: dir::GlobalTypeId,
        owner_substitution: &TypeSubstitution,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // substitute owner parameters inside applied member arguments
        let arguments = self.type_ids(module, member.arguments)?;
        let mut arguments = SmallVec::<[dir::GlobalTypeId; 4]>::from_slice(arguments);
        for argument in &mut arguments {
            *argument = self.substitute_type(*argument, owner_substitution)?;
        }

        // extend the owner substitution with the member's own parameters
        let bindings = self.symbol_generic_argument_bindings(symbol, &arguments)?;
        let mut substitution = owner_substitution.clone();
        for binding in bindings {
            substitution.bind(binding.parameter, binding.argument)?;
        }

        self.substitute_type(value, &substitution)
    }

    /// Return whether associated defaults remain overridable beneath one owner.
    pub(in crate::sema) fn is_rigid_projection_owner(
        &self,
        owner: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let owner = self.shallow_resolve(owner)?;
        let is_rigid = matches!(self.ty(owner)?, dir::Type::Parameter(_) | dir::Type::This);

        Ok(is_rigid)
    }

    /// Select the unique interface application declaring one associated member.
    pub(in crate::sema) fn select_associated_qualifier(
        &mut self,
        origin: Origin,
        owner: dir::GlobalTypeId,
        key: dir::StaticKey,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let mut interfaces = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        let owner = self.check.shallow_resolve(owner)?;

        // parameter projections select from their declared bounds
        if let dir::Type::Parameter(parameter) = self.ty(owner)? {
            interfaces.extend(self.parameter_bounds(origin, parameter)?);
        }
        // other owners select from their checked heritage and extension conformances
        else {
            if self.nominal_application_maybe(owner)?.is_some() {
                let closure = self.heritage_closure(origin, owner)?;
                interfaces.extend(
                    closure
                        .applications
                        .into_iter()
                        .map(|application| application.ty),
                );
            }
            interfaces.extend(self.conformed_interfaces(origin, owner, key)?);
        }

        // keep only interfaces declaring this associated member
        let mut qualifier = None;
        for interface in interfaces {
            let declares = self.has_associated_type(interface, key)?;
            if !declares || qualifier == Some(interface) {
                continue;
            }

            // leave the qualifier unselected between two declaring interfaces
            if qualifier.is_some() {
                return Ok(None);
            }

            qualifier = Some(interface);
        }

        Ok(qualifier)
    }

    /// Return whether one applied interface declares an associated member under one key.
    pub(in crate::sema) fn has_associated_type(
        &mut self,
        interface: dir::GlobalTypeId,
        key: dir::StaticKey,
    ) -> CompilerResult<bool> {
        let Some((_, instance)) = self.nominal_application_maybe(interface)? else {
            return Ok(false);
        };
        let Some(dir::Definition::Interface(definition)) = self.definition(instance.symbol)? else {
            return Ok(false);
        };

        let declares = definition.members.iter().any(|member| {
            matches!(
                member,
                dir::DefinitionMember::AssociatedType(associated)
                    if associated.key == key
            )
        });

        Ok(declares)
    }

    /// Return the substituted constraint declared for one rigid projection.
    pub(in crate::sema) fn projection_constraint(
        &mut self,
        origin: Origin,
        member: &dir::MemberType,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let owner = self.strip_form(origin, member.owner)?;
        let dir::Type::Parameter(parameter) = self.ty(owner)? else {
            return Ok(None);
        };

        for bound in self.parameter_bounds(origin, parameter)? {
            // read projected members from interface bounds only
            let dir::Type::Application(instance) = self.ty(bound)? else {
                continue;
            };
            let members = match self.definition(instance.symbol)? {
                Some(dir::Definition::Interface(definition)) => definition.members.clone(),
                _ => continue,
            };

            // read the declared constraint under the bound's application
            for declared in &members {
                let dir::DefinitionMember::AssociatedType(associated) = declared else {
                    continue;
                };
                if associated.key != member.key {
                    continue;
                }

                let Some(constraint) = associated.constraint else {
                    continue;
                };

                let substitution = self
                    .instance_substitution(bound.module_id, &instance)?
                    .with_receiver(owner);
                let constraint = self.substitute_type(constraint, &substitution)?;

                return Ok(Some(constraint));
            }
        }

        Ok(None)
    }

    /// Return one field type projected through the receiver placement.
    fn project_member_place(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let Some(place) = self.receiver_projected_place(receiver)? else {
            return Ok(ty);
        };

        let ty = self.place_relative_type(origin, place, ty)?;

        self.normalize(origin, ty)
    }

    /// Resolve one relative member type in a projected receiver place.
    pub(in crate::sema) fn place_relative_type(
        &mut self,
        origin: Origin,
        place: dir::GlobalTypeId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // bare members of local receivers stay bare
        if self.check.place_space(place)? == Some(dir::Space::Local) {
            return Ok(ty);
        }

        self.resolve_relative_place(origin, ty, place)
    }

    /// Return the place projected by one receiver type.
    pub(in crate::sema) fn receiver_projected_place(
        &mut self,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let mut current = receiver;
        loop {
            let dir::Type::Form(form) = self.ty(current)? else {
                // bare nominal instances live in their declared or inherited space
                if let dir::Type::Application(instance) = self.ty(current)?
                    && let Some(space) = self.check.nominal_space(instance.symbol)?
                {
                    let place = self.check.place_literal(space)?;

                    return Ok(Some(place));
                }

                return Ok(None);
            };

            match form.form {
                dir::Form::Managed { place } => return Ok(Some(place)),
                dir::Form::Borrowed(_)
                | dir::Form::Owned
                | dir::Form::Readonly
                | dir::Form::Raw => current = form.value,
            }
        }
    }

    /// Return whether one projected value keeps a readonly view.
    fn is_readonly_type_projection(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // keep readonly access over every safe reference type
        if self.type_is_reference(origin, ty)? {
            return Ok(true);
        }

        // skip the view for a copied value, keeping it while the copy is undecided
        if self
            .decide_auto_interface(origin, ty, dir::AutoInterface::Copy)?
            .holds()
        {
            return Ok(false);
        }

        // unions project a view only when one element requires it
        if let dir::Type::Union(union) = self.ty(ty)? {
            let elements: SmallVec<[_; 4]> =
                SmallVec::from_slice(self.type_ids(ty.module_id, union.elements)?);
            for element in elements {
                if self.is_readonly_type_projection(origin, element)? {
                    return Ok(true);
                }
            }

            return Ok(false);
        }

        let result = match self.ty(ty)? {
            dir::Type::Error
            | dir::Type::Hole(_)
            | dir::Type::Rigid(_)
            | dir::Type::Never
            | dir::Type::Unknown
            | dir::Type::Void
            | dir::Type::Null
            | dir::Type::Undefined
            | dir::Type::Primitive(_)
            | dir::Type::Literal(_)
            | dir::Type::Key(_)
            | dir::Type::Region(_)
            | dir::Type::Intrinsic
            | dir::Type::This
            | dir::Type::Static(_) => false,
            dir::Type::Union(_)
            | dir::Type::Refined(_)
            | dir::Type::Object(_)
            | dir::Type::Reference(_)
            | dir::Type::Application(_)
            | dir::Type::Member(_)
            | dir::Type::Variant(_)
            | dir::Type::Form(_)
            | dir::Type::Dynamic(_)
            | dir::Type::Operation(_)
            | dir::Type::FixedArray(_)
            | dir::Type::Range(_)
            | dir::Type::Slice(_)
            | dir::Type::Tuple(_)
            | dir::Type::FunctionSignature(_)
            | dir::Type::Function(_)
            | dir::Type::FunctionPointer(_)
            | dir::Type::Variable(_)
            | dir::Type::Parameter(_)
            | dir::Type::Erased(_)
            | dir::Type::Intersection(_) => true,
        };

        Ok(result)
    }

    /// Return whether one memory access component is readonly.
    pub(in crate::sema) fn is_readonly_access(
        &self,
        access: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let is_readonly = self.access_of(access)? == Some(dir::Access::Readonly);

        Ok(is_readonly)
    }
}
