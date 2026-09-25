use smallvec::SmallVec;
use tspp_dir as dir;
use tspp_dir::{MemberRole, TypeFold};
use tspp_source::ModuleId;

use crate::sema::{
    CandidateOutcome, CheckState, DeclaredMember, ExtensionMatch, ImplementedInterface,
    MemberCandidate, MemberLookup, OpenBounds, Origin, TypeSubstitution, UnboundParameters,
    Verdict,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Return the use-site type projected by one declaration member.
    pub(in crate::sema) fn projected_member_access(
        &mut self,
        origin: Origin,
        receiver: Option<dir::GlobalTypeId>,
        member: &DeclaredMember,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::PropertyAccess> {
        let access = member.access(self, ty)?;
        let mut projected = access;
        projected
            .map_types(&mut |ty| self.projected_member_type(origin, receiver, member.role, ty))?;

        Ok(projected)
    }

    /// Project one member's value type through the receiver it is read on.
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
        let ty = self.normalize(origin, ty)?;

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

        // peel the forms above the receiver
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
        // read the annotated refinement bindings of a qualified projection once
        let refinement = match member.qualifier {
            Some(qualifier) => Some(self.refinements(qualifier)?),
            None => None,
        };

        // bind the projection from annotated refinements
        if let Some((_, bindings)) = &refinement
            && let Some((_, value)) = bindings.iter().find(|(key, _)| *key == member.key)
        {
            return Ok(Some(*value));
        }

        // reduce a rigid owner through the refinements its bounds write
        if self.is_rigid_projection_owner(member.owner)? {
            for bound in self.rigid_owner_bounds(origin, member.owner)? {
                let (_, bindings) = self.refinements(bound)?;
                if let Some((_, value)) = bindings.iter().find(|(key, _)| *key == member.key) {
                    return Ok(Some(*value));
                }
            }

            return Ok(None);
        }

        // select the implementation for applied interface scopes
        if let Some((base, _)) = refinement {
            return match self.ty(base)? {
                // project the applied interface's selected implementation
                dir::Type::Application(_) => self.project_selected_member(origin, member, base),
                // select the implementation for an interface qualifier
                dir::Type::Reference(reference)
                    if self.symbol_kind(reference.symbol)?.is_interface() =>
                {
                    let arguments: SmallVec<[_; 4]> =
                        self.type_ids(base.module_id, reference.arguments)?.into();
                    let arguments = self.intern_type_ids(&arguments)?;
                    let applied =
                        self.intern_type(dir::Type::Application(dir::GenericApplication {
                            symbol: reference.symbol,
                            arguments,
                        }))?;

                    self.project_selected_member(origin, member, applied)
                }
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

        // project what the lookup found, else the qualifying interface's declared default
        if lookup.is_empty() {
            return self.project_default_member(origin, member);
        }

        self.project_member_lookup(module, member, lookup)
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
            && !matches!(*definition, dir::Definition::Interface(_))
        {
            let members = definition.members();
            let substitution =
                self.qualified_instance_substitution(interface_module, &interface, owner)?;

            return self.project_declared_associated_member(origin, member, members, &substitution);
        }

        // enumerate candidate extensions by the default ownership of the object beneath the owner
        let object = self.strip_form(origin, owner)?;
        let apparent = self.intern_apparent_type(object)?;
        let extensions =
            self.implementations_over(origin, module, Some(apparent), interface.symbol)?;
        for (extension_symbol, _) in extensions {
            let definition = self.definition(extension_symbol)?;
            let Some(dir::Definition::Extension(extension)) = definition.as_deref() else {
                continue;
            };

            if !extension.is_visible_from(module) {
                continue;
            }

            // read the extension target, interfaces, and members
            let target_type = extension.target.r#type();
            let interfaces = extension
                .implementations()
                .map(|conformance| conformance.interface)
                .collect::<SmallVec<[_; 2]>>();
            let members = extension.members.clone();
            let template = self.symbol_template(extension_symbol)?;

            // decide the candidate, constraining only a holding one
            let verdict = self.decide_candidate(|state| {
                let matched = state.match_extension_implementation(
                    origin,
                    interface_module,
                    owner,
                    owner,
                    &interface,
                    template,
                    target_type,
                    &interfaces,
                    OpenBounds::Decide,
                )?;

                Ok(match matched {
                    ExtensionMatch::Matched(..) => CandidateOutcome::Accepted(()),
                    // unsatisfied bounds leave the candidate unselected here
                    ExtensionMatch::Unmatched | ExtensionMatch::Unproven => {
                        CandidateOutcome::Rejected(())
                    }
                })
            })?;

            // leave the projection open while an undecided candidate may still provide it
            match verdict {
                Verdict::Holds => {}
                Verdict::Ambiguous => return Ok(None),
                Verdict::Fails => continue,
            }

            // rerun the match to commit its substitution
            let matched = self.match_extension_implementation(
                origin,
                interface_module,
                owner,
                owner,
                &interface,
                template,
                target_type,
                &interfaces,
                OpenBounds::Decide,
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

        // match the declared implementations of the object beneath the owner
        if let Some((application_module, application)) = self.nominal_application_maybe(object)?
            && let Some(definition) = self.definition(application.symbol)?
        {
            let interfaces = definition
                .implementations()
                .map(|conformance| conformance.interface)
                .collect::<SmallVec<[_; 2]>>();
            let members = definition.members();

            if !interfaces.is_empty() {
                let mut substitution =
                    self.instance_substitution(application_module, &application)?;
                let matched = self.match_implemented_interface(
                    origin,
                    interface_module,
                    &[],
                    &mut substitution,
                    &interfaces,
                    &interface,
                    true,
                )?;
                if matches!(matched, ImplementedInterface::Matched(_)) {
                    // project the owner's own declared value, falling back to the interface default
                    let projected = self.project_declared_associated_member(
                        origin,
                        member,
                        members,
                        &substitution,
                    )?;
                    if projected.is_some() {
                        return Ok(projected);
                    }

                    return self.project_qualified_default(origin, member, qualifier);
                }
            }
        }

        // project the interface's declared default when nothing overrides it
        self.project_qualified_default(origin, member, qualifier)
    }

    /// Project one associated member declared by a lexical extension scope.
    fn project_scope_member(
        &mut self,
        origin: Origin,
        member: &dir::MemberType,
        scope: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // stay symbolic while the scope is still declaring, and fail loudly after
        let Some(definition) = self.definition(scope)? else {
            if self.is_declaring() {
                return Ok(None);
            }

            return Err(CompilerError::Internal {
                message: format!("associated type scope {scope:?} has no definition"),
            });
        };

        // bind an extension scope's parameters through its matched target
        let substitution = match &*definition {
            dir::Definition::Extension(extension) => {
                let template = self.symbol_template(scope)?;
                let target = extension.target.r#type();

                // read the scope's own receiver at its rigid parameters
                let given_owner = self.shallow_resolve(member.owner)?;
                let mut matched = (given_owner == target)
                    .then(|| TypeSubstitution::default().with_receiver(member.owner));

                // match the owner as given, then its payload beneath any form
                let stripped = self.strip_form(origin, member.owner)?;
                for owner in [given_owner, stripped] {
                    if matched.is_some() {
                        break;
                    }

                    matched = self
                        .match_extension_subject(
                            origin,
                            owner,
                            owner,
                            template,
                            target,
                            UnboundParameters::Open,
                        )?
                        .map(|matched| matched.with_receiver(owner));
                    if matched.is_some() {
                        break;
                    }
                }
                let Some(matched) = matched else {
                    return Ok(None);
                };

                matched
            }
            _ => TypeSubstitution::default().with_receiver(member.owner),
        };

        // project the scope's members against the given owner
        let members = definition.members();
        if let Some(projected) =
            self.project_declared_associated_member(origin, member, members, &substitution)?
        {
            return Ok(Some(projected));
        }

        // fall back to the default the scope's implemented interface declares
        let implements = definition
            .implementations()
            .map(|conformance| conformance.interface)
            .collect::<SmallVec<[_; 2]>>();
        for interface in implements {
            if !self.declares_associated_member(interface, member.key)? {
                continue;
            }
            let qualifier = self.substitute_type(interface, &substitution)?;

            return self.project_qualified_default(origin, member, qualifier);
        }

        Ok(None)
    }

    /// Return the value one associated member holds: a type's type or a const's static value.
    pub(in crate::sema) fn associated_member_value(
        &mut self,
        member: &dir::DefinitionMember,
    ) -> CompilerResult<Option<(dir::GlobalSymbolId, dir::GlobalTypeId)>> {
        let value = match member {
            dir::DefinitionMember::AssociatedType(associated) => {
                associated.value.map(|value| (associated.symbol, value))
            }
            dir::DefinitionMember::AssociatedConst(associated) => self
                .static_value(associated.symbol)?
                .map(|value| (associated.symbol, value)),
            _ => None,
        };

        Ok(value)
    }

    /// Project one associated member declared by a selected scope.
    fn project_declared_associated_member(
        &mut self,
        origin: Origin,
        member: &dir::MemberType,
        members: &[dir::DefinitionMember],
        substitution: &TypeSubstitution,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // select the scope's declared value for this key
        let mut declared = None;
        for candidate in members {
            if candidate.is_associated_at(member.key) {
                declared = self.associated_member_value(candidate)?;
                break;
            }
        }

        // apply the implementation's arguments to the declared value
        if let Some((symbol, value)) = declared {
            let value = self.project_associated_value(
                origin.module(),
                member,
                symbol,
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

        // apply the interface arguments to its declared members
        let members = definition.members();
        let substitution =
            self.qualified_instance_substitution(interface_module, &interface, member.owner)?;

        self.project_declared_associated_member(origin, member, members, &substitution)
    }

    /// Return the type projected by one selected member lookup.
    fn project_member_lookup(
        &mut self,
        module: ModuleId,
        member: &dir::MemberType,
        lookup: MemberLookup,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // project every runtime arm as one candidate each, and join the arms
        let mut types = Vec::new();
        for group in lookup.arms() {
            let [candidate] = group.candidates.as_slice() else {
                return Ok(None);
            };
            let Some(ty) = self.project_member_candidate(module, member, candidate)? else {
                return Ok(None);
            };
            types.push(ty);
        }

        self.normalized_union_type(types).map(Some)
    }

    /// Return the type one selected member candidate projects.
    fn project_member_candidate(
        &mut self,
        module: ModuleId,
        member: &dir::MemberType,
        candidate: &MemberCandidate,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let Some(declared) = candidate.declaration() else {
            return candidate.read_type(self);
        };

        // keep nominal singleton identity for variant values
        if candidate.role == MemberRole::VariantValue {
            return candidate.read_type(self);
        }

        // project associated values through their applied arguments
        if declared.value_type.is_some() {
            let value =
                self.static_value(declared.symbol)?
                    .ok_or_else(|| CompilerError::Internal {
                        message: format!(
                            "associated member {:?} lost its declared value",
                            declared.symbol,
                        ),
                    })?;
            let substitution = TypeSubstitution {
                bindings: declared.generic_arguments.iter().copied().collect(),
                receiver: Some(member.owner),
            };
            let value = self.project_associated_value(
                module,
                member,
                declared.symbol,
                value,
                &substitution,
            )?;

            return Ok(Some(value));
        }

        // static values project as their own singleton
        if let Some(value) = declared.value {
            let ty = self.intern_type(dir::Type::Static(value))?;

            return Ok(Some(ty));
        }

        // keep a member projecting back onto itself symbolic
        if let Some(projected) = self.member_head(candidate.access.store())?
            && projected.owner == member.owner
            && projected.key == member.key
        {
            return Ok(None);
        }

        candidate.read_type(self)
    }

    /// Project one interface default through a qualified owner.
    fn project_default_member(
        &mut self,
        origin: Origin,
        member: &dir::MemberType,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // read the interface holding the default from a qualified projection only
        let Some(qualifier) = member.qualifier else {
            return Ok(None);
        };
        let Some((interface_module, interface)) = self.nominal_application_maybe(qualifier)? else {
            return Ok(None);
        };

        // select the declared interface default
        let declared = match self.definition(interface.symbol)?.as_deref() {
            Some(dir::Definition::Interface(definition)) => definition
                .members
                .iter()
                .find(|declared| declared.is_associated_at(member.key))
                .cloned(),
            _ => None,
        };
        let Some(declared) = declared else {
            return Ok(None);
        };
        let Some((symbol, value)) = self.associated_member_value(&declared)? else {
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

        // extend the owner substitution with the member's own parameters, lifetimes included
        let mut substitution = owner_substitution.clone();
        if !arguments.is_empty()
            && let Some(template) = self.symbol_template(symbol)?
        {
            let parameters = self.generic_template_parameters(template)?;
            for binding in self
                .parameter_substitution(&parameters, &arguments)?
                .bindings
            {
                substitution.bind(binding.parameter, binding.argument)?;
            }
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

    /// Return the bounds one rigid projection owner assumes.
    fn rigid_owner_bounds(
        &mut self,
        origin: Origin,
        owner: dir::GlobalTypeId,
    ) -> CompilerResult<SmallVec<[dir::GlobalTypeId; 2]>> {
        let owner = self.shallow_resolve(owner)?;
        match self.ty(owner)? {
            dir::Type::Parameter(parameter) => self.parameter_bounds(origin, parameter),
            dir::Type::This => self.assumed_bounds(origin, |ty| matches!(ty, dir::Type::This)),
            _ => Ok(SmallVec::new()),
        }
    }

    /// Select the unique interface application declaring one associated member.
    pub(in crate::sema) fn select_associated_qualifier(
        &mut self,
        origin: Origin,
        owner: dir::GlobalTypeId,
        key: dir::StaticKey,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let mut interfaces = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        let owner = self.shallow_resolve(owner)?;

        // an interface application qualifies its own associated projections
        if self.declares_associated_member(owner, key)? {
            return Ok(Some(owner));
        }

        // rigid owners select from their bounds and the interfaces those extend
        let bounds = match self.ty(owner)? {
            dir::Type::Parameter(parameter) => Some(self.parameter_bounds(origin, parameter)?),
            dir::Type::This => Some(self.this_bounds(origin)?),
            _ => None,
        };
        if let Some(bounds) = bounds {
            for bound in bounds {
                interfaces.push(bound);
                let closure = self.heritage_closure(origin, bound)?;
                interfaces.extend(
                    closure
                        .applications
                        .into_iter()
                        .map(|application| application.ty),
                );
            }
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

        // keep only interfaces declaring this associated member, one application per interface
        let mut qualifier: Option<dir::GlobalTypeId> = None;
        for interface in interfaces {
            let declares = self.declares_associated_member(interface, key)?;
            let same = match qualifier {
                Some(selected) => self.ty(selected)?.symbol() == self.ty(interface)?.symbol(),
                None => false,
            };
            if !declares || same {
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
    pub(in crate::sema) fn declares_associated_member(
        &mut self,
        interface: dir::GlobalTypeId,
        key: dir::StaticKey,
    ) -> CompilerResult<bool> {
        let Some((_, instance)) = self.nominal_application_maybe(interface)? else {
            return Ok(false);
        };
        let declared = self.definition(instance.symbol)?;
        let Some(dir::Definition::Interface(definition)) = declared.as_deref() else {
            return Ok(false);
        };

        // read whether the interface declares that associated member
        let declares = definition
            .members
            .iter()
            .any(|member| member.is_associated_at(key));

        Ok(declares)
    }

    /// Return the substituted constraint declared for one rigid projection.
    pub(in crate::sema) fn projection_constraint(
        &mut self,
        origin: Origin,
        member: &dir::MemberType,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let owner = self.strip_form(origin, member.owner)?;

        // read the declaring interface's own bound for a qualified projection
        if let Some(qualifier) = member.qualifier
            && let Some(constraint) = self.associated_member_constraint(qualifier, member.key)?
        {
            let receiver = self.shallow_resolve(member.owner)?;
            let constraint = self.instantiate_interface_type(constraint, qualifier, receiver)?;

            return Ok(Some(constraint));
        }

        // read the where clauses declared exactly on an owner outside a parameter
        let dir::Type::Parameter(parameter) = self.ty(owner)? else {
            let given_owner = self.shallow_resolve(member.owner)?;
            let owner_head = self.ty(given_owner)?;
            let bounds = self.assumed_bounds(origin, |ty| *ty == owner_head)?;
            for bound in bounds {
                let Some(constraint) = self.associated_member_constraint(bound, member.key)? else {
                    continue;
                };

                return Ok(Some(self.instantiate_interface_type(
                    constraint,
                    bound,
                    given_owner,
                )?));
            }

            return Ok(None);
        };

        // search the parameter's declared bounds
        for bound in self.parameter_bounds(origin, parameter)? {
            // read projected members from interface bounds only
            let dir::Type::Application(instance) = self.ty(bound)? else {
                continue;
            };
            let members = match self.definition(instance.symbol)?.as_deref() {
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

    /// Return whether one projected value keeps a readonly view.
    fn is_readonly_type_projection(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // end the view at a managed handle, a borrow keeping the readonly cap
        let chain = self.form_chain(origin, ty)?;
        if self.form_ownership(origin, &chain)? == Some(dir::Ownership::Managed) {
            return Ok(false);
        }
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

        // decide by the type's own head
        let result = match self.ty(ty)? {
            dir::Type::Error
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
