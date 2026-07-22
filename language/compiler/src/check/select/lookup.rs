use destack_core::FxIndexSet;
use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, ApparentInstance, BodyState, DecisionKind, MemberCandidate, MemberLookup, MemberRole,
    Origin, ReceiverSteps, TypeSubstitution, answer,
};

/// One active member lookup query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct MemberQuery {
    /// The module whose visibility rules apply.
    module: ModuleId,
    /// The receiver type retained in selected resolutions.
    receiver: dir::GlobalTypeId,
    /// The type currently queried for matching members.
    subject: dir::GlobalTypeId,
    /// The member namespace.
    space: dir::MemberSpace,
    /// The member key.
    key: dir::StaticKey,
    /// Whether extension members are admitted.
    extensions: ExtensionFilter,
}

/// Which member sources one lookup admits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ExtensionFilter {
    /// Search receiver and base declarations only.
    Inherent,
    /// Search inherent members first, then extensions.
    All,
}

impl BodyState<'_, '_> {
    /// Return the member space implied by one receiver expression.
    pub(in crate::check) fn member_receiver_space(
        &mut self,
        receiver: dir::GlobalNodeIdAny,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::MemberSpace> {
        if matches!(self.ty(ty)?, dir::Type::Reference(_)) {
            return Ok(dir::MemberSpace::Static);
        }

        let resolutions = self.resolutions(receiver.module_id);
        let symbol = match self.decision_kind(receiver) {
            Some(DecisionKind::Name) => match resolutions.name_resolution(receiver) {
                Some(resolution) => match resolution.symbols() {
                    [symbol] => Some(*symbol),
                    _ => None,
                },
                None => None,
            },
            Some(DecisionKind::Instantiation) => resolutions
                .instantiation_resolution(receiver)
                .map(|resolution| resolution.symbol),
            _ => None,
        };
        let Some(symbol) = symbol else {
            return Ok(dir::MemberSpace::Instance);
        };

        let symbol = self.resolve_symbol_alias(symbol)?;
        let kind = self.symbol_kind(symbol);

        // a name naming a type reaches its static members, so
        //  parameters serve bound statics like rustc's T::default()
        let names_type = kind.is_nominal() || matches!(kind, dir::SymbolKind::GenericTypeParameter);
        let space = if names_type {
            dir::MemberSpace::Static
        } else {
            dir::MemberSpace::Instance
        };

        Ok(space)
    }

    /// Look up one member on a receiver type.
    pub(in crate::check) fn lookup_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
    ) -> CompilerResult<Answer<MemberLookup>> {
        let mut active_queries = FxIndexSet::default();

        self.lookup_subject_member(
            origin,
            module,
            receiver,
            receiver,
            space,
            key,
            ExtensionFilter::All,
            &mut active_queries,
        )
    }

    /// Look up one non-extension member on a receiver type.
    pub(in crate::check) fn lookup_inherent_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
    ) -> CompilerResult<Answer<MemberLookup>> {
        let mut active_queries = FxIndexSet::default();

        self.lookup_subject_member(
            origin,
            module,
            receiver,
            receiver,
            space,
            key,
            ExtensionFilter::Inherent,
            &mut active_queries,
        )
    }

    /// Look one member up in `subject` while retaining `receiver` for projection.
    fn lookup_subject_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        subject: dir::GlobalTypeId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
        extensions: ExtensionFilter,
        active: &mut FxIndexSet<MemberQuery>,
    ) -> CompilerResult<Answer<MemberLookup>> {
        // static names read the written declaration before aliases reduce
        let root = self.settled_root(subject)?;
        if space == dir::MemberSpace::Static
            && let dir::Type::Reference(reference) = self.ty(root)?
        {
            return self.lookup_declaration_member(
                origin, module, receiver, reference, space, key, extensions, active,
            );
        }

        let subject = match self.reduce_type_head(origin, subject)? {
            Answer::Ready(subject) => subject,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };

        // stop cyclic paths through constraints, unions, and heritage
        let query = MemberQuery {
            module,
            receiver,
            subject,
            space,
            key,
            extensions,
        };
        if !active.insert(query) {
            return Ok(Answer::Ready(MemberLookup::Missing));
        }

        let lookup = self.lookup_settled_member(
            origin, module, receiver, subject, space, key, extensions, active,
        );
        active.swap_remove(&query);

        lookup
    }

    /// Look one member up in one already reduced subject type.
    fn lookup_settled_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        subject: dir::GlobalTypeId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
        extensions: ExtensionFilter,
        active: &mut FxIndexSet<MemberQuery>,
    ) -> CompilerResult<Answer<MemberLookup>> {
        match self.ty(subject)? {
            // memory forms look through their payloads
            dir::Type::Form(form) => self.lookup_subject_member(
                origin, module, receiver, form.value, space, key, extensions, active,
            ),

            // refinements answer their refined member, then their base
            dir::Type::Refined(refined) => {
                let refined = self.type_refined(subject.module_id, refined)?;
                if refined.key == key {
                    return Ok(Answer::Ready(MemberLookup::Field(refined.value)));
                }

                self.lookup_subject_member(
                    origin,
                    module,
                    receiver,
                    refined.base,
                    space,
                    key,
                    extensions,
                    active,
                )
            }

            // declaration references read static members
            dir::Type::Reference(reference) => self.lookup_declaration_member(
                origin, module, receiver, reference, space, key, extensions, active,
            ),

            // applied declarations read their definition members
            dir::Type::Application(_)
            | dir::Type::Literal(_)
            | dir::Type::Primitive(_)
            | dir::Type::Array(_)
            | dir::Type::Slice(_)
            | dir::Type::FixedArray(_) => {
                let lookup = answer!(self.lookup_apparent_instance_member(
                    origin, module, receiver, subject, space, key, extensions,
                )?);

                // newtypes dereference to their backing for missing members
                if matches!(lookup, MemberLookup::Missing)
                    && let Some(instance) = self.decompose_newtype(origin, subject)?
                {
                    let value = instance.backing;
                    let projection = instance.into_projection();
                    let receiver = answer!(self.replace_form_value(origin, receiver, value)?);
                    let mut lookup = answer!(self.lookup_subject_member(
                        origin, module, receiver, value, space, key, extensions, active,
                    )?);

                    // record the payload projection before deeper receiver steps
                    let dir::Projection::NewtypePayload {
                        symbol,
                        generic_arguments,
                        ..
                    } = projection
                    else {
                        unreachable!("newtype backing projects a payload");
                    };
                    let projection = dir::Projection::NewtypePayload {
                        symbol,
                        generic_arguments,
                        ty: receiver,
                    };
                    if let MemberLookup::Found(candidates) = &mut lookup {
                        for candidate in candidates {
                            candidate.steps.insert(0, projection.clone());
                        }
                    }

                    return Ok(Answer::Ready(lookup));
                }

                Ok(Answer::Ready(lookup))
            }

            // enum members use the owner enum's instance members
            dir::Type::EnumMember(member) => self.lookup_subject_member(
                origin,
                module,
                receiver,
                member.owner,
                space,
                key,
                extensions,
                active,
            ),

            // generic parameters look through their bounds
            dir::Type::Parameter(parameter) => {
                let bounds = self.parameter_bounds(origin, parameter)?;

                self.lookup_bound_member(
                    origin, module, receiver, &bounds, space, key, extensions, active,
                )
            }

            // erased values expose members through their dynamic payload
            dir::Type::Dynamic(dynamic) => {
                let constraint = dynamic.constraint;
                let receiver = answer!(self.replace_form_value(origin, receiver, constraint)?);
                let mut lookup = answer!(self.lookup_bound_member(
                    origin,
                    module,
                    receiver,
                    &[constraint],
                    space,
                    key,
                    extensions,
                    active,
                )?);
                let projection = dir::Projection::DynamicPayload { ty: receiver };

                if let MemberLookup::Found(candidates) = &mut lookup {
                    for candidate in candidates {
                        candidate.steps.insert(0, projection.clone());
                    }
                }

                Ok(Answer::Ready(lookup))
            }

            // structural shapes expose their fields
            dir::Type::Shape(shape) => {
                let field = self
                    .shape_fields(subject.module_id, shape.fields)?
                    .iter()
                    .find(|field| field.key == key)
                    .copied();

                // optional fields read as their value or undefined
                let field = match field {
                    Some(field) if field.is_optional => {
                        let undefined = self.intern_type(module, dir::Type::Undefined)?;

                        Some(self.normalized_union_type(module, [field.ty, undefined])?)
                    }
                    field => field.map(|field| field.ty),
                };

                match field {
                    Some(ty) => match self.projected_member_type(
                        origin,
                        Some(receiver),
                        MemberRole::Field,
                        ty,
                    )? {
                        Answer::Ready(ty) => Ok(Answer::Ready(MemberLookup::Field(ty))),
                        Answer::Pending(blockers) => Ok(Answer::Pending(blockers)),
                    },
                    None => Ok(Answer::Ready(MemberLookup::Missing)),
                }
            }

            // tuples expose their labeled elements
            dir::Type::Tuple(tuple) => {
                let element = self
                    .tuple_elements(subject.module_id, tuple.elements)?
                    .iter()
                    .find(|element| {
                        element
                            .label
                            .is_some_and(|label| key == dir::StaticKey::Name(label))
                    })
                    .map(|element| element.ty);

                match element {
                    Some(ty) => match self.projected_member_type(
                        origin,
                        Some(receiver),
                        MemberRole::Field,
                        ty,
                    )? {
                        Answer::Ready(ty) => Ok(Answer::Ready(MemberLookup::Field(ty))),
                        Answer::Pending(blockers) => Ok(Answer::Pending(blockers)),
                    },
                    None => Ok(Answer::Ready(MemberLookup::Missing)),
                }
            }

            // unions join member lookups across their elements
            dir::Type::Union(union) => {
                let elements = self.type_ids(subject.module_id, union.elements)?.to_vec();

                self.lookup_union_member(
                    origin, module, receiver, subject, &elements, space, key, extensions, active,
                )
            }

            // intersections expose each element's members
            dir::Type::Intersection(intersection) => {
                let elements = self
                    .type_ids(subject.module_id, intersection.elements)?
                    .to_vec();
                for element in elements {
                    let element = self.settled_root(element)?;
                    match self.lookup_subject_member(
                        origin, module, receiver, element, space, key, extensions, active,
                    )? {
                        Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
                        Answer::Ready(MemberLookup::Missing) => continue,
                        Answer::Ready(lookup) => return Ok(Answer::Ready(lookup)),
                    }
                }

                Ok(Answer::Ready(MemberLookup::Missing))
            }

            _ => Ok(Answer::Ready(MemberLookup::Missing)),
        }
    }

    /// Look up one member on a receiver known only by its bounds.
    fn lookup_bound_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        bounds: &[dir::GlobalTypeId],
        space: dir::MemberSpace,
        key: dir::StaticKey,
        extensions: ExtensionFilter,
        active: &mut FxIndexSet<MemberQuery>,
    ) -> CompilerResult<Answer<MemberLookup>> {
        for bound in bounds {
            let lookup = self.lookup_subject_member(
                origin, module, receiver, *bound, space, key, extensions, active,
            )?;
            match lookup {
                Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
                Answer::Ready(MemberLookup::Missing) => continue,
                Answer::Ready(lookup) => return Ok(Answer::Ready(lookup)),
            }
        }

        Ok(Answer::Ready(MemberLookup::Missing))
    }

    /// Look up one member through the receiver's apparent declaration instance.
    fn lookup_apparent_instance_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        lookup_type: dir::GlobalTypeId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
        extensions: ExtensionFilter,
    ) -> CompilerResult<Answer<MemberLookup>> {
        let Some(instance) = self.apparent_instance(lookup_type)? else {
            return Ok(Answer::Ready(MemberLookup::Missing));
        };

        self.lookup_symbol_member(origin, module, receiver, instance, space, key, extensions)
    }

    /// Look up one static member on a declaration reference.
    fn lookup_declaration_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        reference: dir::TypeReference,
        space: dir::MemberSpace,
        key: dir::StaticKey,
        extensions: ExtensionFilter,
        active: &mut FxIndexSet<MemberQuery>,
    ) -> CompilerResult<Answer<MemberLookup>> {
        if space != dir::MemberSpace::Static {
            return Ok(Answer::Ready(MemberLookup::Missing));
        }

        // resolve aliases before reading declaration members
        let mut symbol = self.resolve_symbol_alias(reference.symbol)?;

        // a type alias names its body's root declaration for statics,
        //  and the body's own members serve whatever the root lacks;
        //  head reduction expands the whole alias chain in one step
        let mut alias_body = None;
        if let Some(dir::Definition::TypeAlias(alias)) = self.definition(symbol)? {
            let value = alias.value;
            let head = answer!(self.reduce_type_head(origin, value)?);
            alias_body = Some(head);
            if let Some(named) = self.type_symbol(head)? {
                symbol = self.resolve_symbol_alias(named)?;
            }
        }
        if !self.is_component_module(symbol.module_id) {
            self.import_external_module(symbol.module_id)?;
        }

        // search declaration members before extensions
        let mut lookup = MemberLookup::Missing;
        if !self.symbol_kind(symbol).is_type_alias() {
            let inherent =
                answer!(self.lookup_inherent_declaration_member(origin, receiver, symbol, key)?);
            lookup = match inherent {
                MemberLookup::Found(_) | MemberLookup::Field(_) => {
                    return Ok(Answer::Ready(inherent));
                }
                MemberLookup::Missing => match extensions {
                    ExtensionFilter::All => {
                        answer!(self.lookup_static_extension_member(origin, module, symbol, key)?)
                    }
                    ExtensionFilter::Inherent => MemberLookup::Missing,
                },
            };
        }

        // aliased bodies answer whatever the root declaration lacks
        if matches!(lookup, MemberLookup::Missing)
            && let Some(body) = alias_body
        {
            return self.lookup_subject_member(
                origin, module, receiver, body, space, key, extensions, active,
            );
        }

        Ok(Answer::Ready(lookup))
    }

    /// Join member lookups across union elements.
    fn lookup_union_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        subject: dir::GlobalTypeId,
        elements: &[dir::GlobalTypeId],
        space: dir::MemberSpace,
        key: dir::StaticKey,
        extensions: ExtensionFilter,
        active: &mut FxIndexSet<MemberQuery>,
    ) -> CompilerResult<Answer<MemberLookup>> {
        let mut candidates: Vec<MemberCandidate> = Vec::new();
        let mut arms: Vec<SmallVec<[dir::GlobalTypeId; 2]>> = Vec::new();
        let mut fields = SmallVec::<[dir::GlobalTypeId; 4]>::new();

        // direct union receivers seal each member under its own arm
        let is_direct = self.settled_root(receiver)? == self.settled_root(subject)?;

        // every element must expose the member
        for element in elements {
            let arm_receiver = if is_direct { *element } else { receiver };
            match answer!(self.lookup_subject_member(
                origin,
                module,
                arm_receiver,
                *element,
                space,
                key,
                extensions,
                active
            )?) {
                MemberLookup::Field(ty) => fields.push(ty),
                MemberLookup::Found(found) => {
                    for candidate in found {
                        let shared = candidate.symbol.is_some().then(|| {
                            candidates
                                .iter()
                                .position(|existing| existing.symbol == candidate.symbol)
                        });
                        match shared.flatten() {
                            Some(index) => arms[index].push(*element),
                            None => {
                                candidates.push(candidate);
                                arms.push(SmallVec::from_slice(&[*element]));
                            }
                        }
                    }
                }
                MemberLookup::Missing => return Ok(Answer::Ready(MemberLookup::Missing)),
            }
        }

        // dispatched candidates bind their narrowed runtime receivers;
        //  one shared candidate keeps the whole union receiver
        if candidates.len() > 1 {
            for (candidate, arms) in candidates.iter_mut().zip(&arms) {
                let arm = match arms.as_slice() {
                    [single] => *single,
                    _ => self.normalized_union_type(origin.module(), arms.iter().copied())?,
                };
                candidate.receiver.get_or_insert(arm);
            }
        }

        // pure field unions join into one field type
        if candidates.is_empty() {
            let joined = match fields.as_slice() {
                [single] => *single,
                _ => self.normalized_union_type(origin.module(), fields)?,
            };

            return Ok(Answer::Ready(MemberLookup::Field(joined)));
        }

        Ok(Answer::Ready(MemberLookup::from_candidates(candidates)))
    }

    /// Look up one member on a declaration reference.
    fn lookup_symbol_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        instance: ApparentInstance,
        space: dir::MemberSpace,
        key: dir::StaticKey,
        extensions: ExtensionFilter,
    ) -> CompilerResult<Answer<MemberLookup>> {
        if !self.is_component_module(instance.symbol.module_id) {
            self.import_external_module(instance.symbol.module_id)?;
        }

        // search inherent members before extensions
        let inherent =
            answer!(self.lookup_inherent_symbol_member(origin, receiver, &instance, space, key)?);
        match inherent {
            MemberLookup::Found(_) | MemberLookup::Field(_) => {
                return Ok(Answer::Ready(inherent));
            }
            MemberLookup::Missing => {}
        }

        match extensions {
            ExtensionFilter::All => {
                // extension targets name values, so receivers shed memory forms
                let receiver = answer!(self.strip_form(origin, receiver)?);
                let lookup = answer!(self.lookup_extension_member(
                    origin,
                    module,
                    receiver,
                    instance.symbol,
                    space,
                    key
                )?);

                // values also match targets naming their apparent owner,
                //  so primitives reach extensions of their owning class
                if matches!(lookup, MemberLookup::Missing) {
                    let apparent = self.intern_apparent_type(module, receiver)?;
                    if apparent != receiver {
                        return self.lookup_extension_member(
                            origin,
                            module,
                            apparent,
                            instance.symbol,
                            space,
                            key,
                        );
                    }
                }

                Ok(Answer::Ready(lookup))
            }
            ExtensionFilter::Inherent => Ok(Answer::Ready(MemberLookup::Missing)),
        }
    }

    /// Look up one inherent static member on a declaration reference.
    fn lookup_inherent_declaration_member(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        symbol: dir::GlobalSymbolId,
        key: dir::StaticKey,
    ) -> CompilerResult<Answer<MemberLookup>> {
        let Some(definition) = self.definition(symbol)? else {
            return Ok(Answer::Ready(MemberLookup::Missing));
        };
        let members = definition
            .members_with_key(dir::MemberSpace::Static, key)
            .cloned()
            .collect::<SmallVec<[_; 2]>>();
        let mut candidates = Vec::new();

        // collect visible static declaration members
        for member in members {
            let Some(member) = answer!(self.declared_member(&member)?) else {
                continue;
            };
            let Some(ty) = member.ty else {
                continue;
            };
            let ty = member.value_type(self, ty)?;
            let written = member.symbol.and_then(|symbol| self.static_value(symbol));
            let ty =
                answer!(self.projected_member_type(origin, Some(receiver), member.role, ty)?);

            // variant singletons instantiate their owner freshly per use
            let (ty, generic_arguments) = match member.role {
                MemberRole::Variant => {
                    answer!(self.instantiate_variant_member(origin, symbol, ty)?)
                }
                _ => (ty, Vec::new()),
            };

            candidates.push(MemberCandidate {
                symbol: member.symbol,
                owner: symbol,
                space: dir::MemberSpace::Static,
                role: member.role,
                ty,
                generic_arguments,
                value: member.value,
                value_type: written,
                steps: ReceiverSteps::new(),
                receiver: None,
            });
        }

        Ok(Answer::Ready(MemberLookup::from_candidates(candidates)))
    }

    /// Instantiate one variant singleton's owner with fresh arguments.
    fn instantiate_variant_member(
        &mut self,
        origin: Origin,
        owner: dir::GlobalSymbolId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<(dir::GlobalTypeId, Vec<dir::GenericArgumentBinding>)>> {
        let Some(template) = self.symbol_template(owner)? else {
            return Ok(Answer::Ready((ty, Vec::new())));
        };
        let parameters = self.generic_template_parameters(template);
        if parameters.is_empty() {
            return Ok(Answer::Ready((ty, Vec::new())));
        }

        // open one inference argument per owner parameter
        let Some(substitution) = self.instantiate_parameter_arguments(
            origin,
            &parameters,
            &[],
            TypeSubstitution::default(),
        )?
        else {
            return Ok(Answer::Ready((ty, Vec::new())));
        };
        let ty = self.substitute_type(origin.module(), ty, &substitution)?;

        // unconstrained singleton arguments settle back to their parameters
        for (parameter, argument) in substitution
            .parameters
            .iter()
            .copied()
            .zip(substitution.arguments.iter().copied())
        {
            let argument = self.settled_root(argument)?;
            let Some(variable) = self.root_variable(argument)? else {
                continue;
            };
            if self.solver.variables.variable_default(variable).is_none() {
                let rigid = self.generic_parameter_type(parameter)?;
                self.set_variable_default(variable, rigid);
            }
        }
        let generic_arguments = substitution
            .parameters
            .iter()
            .copied()
            .zip(substitution.arguments.iter().copied())
            .map(|(parameter, argument)| dir::GenericArgumentBinding::new(parameter, argument))
            .collect();

        Ok(Answer::Ready((ty, generic_arguments)))
    }

    /// Look up one inherent member on a declaration, walking its heritage.
    fn lookup_inherent_symbol_member(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        instance: &ApparentInstance,
        space: dir::MemberSpace,
        key: dir::StaticKey,
    ) -> CompilerResult<Answer<MemberLookup>> {
        // collect own members and heritage applications
        let Some(definition) = self.definition(instance.symbol)? else {
            return Ok(Answer::Ready(MemberLookup::Missing));
        };
        let members = definition
            .members_with_key(space, key)
            .cloned()
            .collect::<SmallVec<[_; 2]>>();
        let heritages = definition
            .bases()
            .iter()
            .map(|heritage| (heritage.symbol, heritage.arguments.clone()))
            .collect::<SmallVec<[_; 2]>>();

        // substitute applied arguments and the qualified receiver
        let substitution = instance.substitution(self)?.with_receiver(receiver);
        let mut candidates = Vec::new();
        for member in members {
            let Some(declared) = answer!(self.declared_member(&member)?) else {
                continue;
            };
            let symbol = declared.symbol;

            // value-less associated types project through the receiver, and
            //  defaults stay conformance-only behind rigid owners
            let is_associated = matches!(member, dir::DefinitionMember::AssociatedType(_));
            let receiver_root = self.settled_root(receiver)?;
            let rigid_owner = matches!(
                self.ty(receiver_root)?,
                dir::Type::Parameter(_) | dir::Type::This
            );
            let ty = match declared.ty {
                Some(ty) if !(is_associated && rigid_owner) => ty,
                _ if is_associated => {
                    let arguments = self.intern_type_ids(origin.module(), &[])?;

                    self.intern_member(
                        origin.module(),
                        dir::MemberType {
                            owner: receiver,
                            key,
                            arguments,
                            qualifier: None,
                        },
                    )?
                }
                _ => continue,
            };
            let member_definition = member;
            let member = declared;

            let ty = self.substitute_type(origin.module(), ty, &substitution)?;
            // optional fields read as their value or undefined
            let ty = match &member_definition {
                dir::DefinitionMember::Field(field) if field.is_optional => {
                    let undefined = self.intern_type(origin.module(), dir::Type::Undefined)?;

                    self.normalized_union_type(origin.module(), [ty, undefined])?
                }
                _ => ty,
            };
            let ty = member.value_type(self, ty)?;
            let ty =
                answer!(self.projected_member_type(origin, Some(receiver), member.role, ty)?);

            // carry substituted static value types for projections
            let written = match symbol.and_then(|symbol| self.static_value(symbol)) {
                Some(written) => {
                    Some(self.substitute_type(origin.module(), written, &substitution)?)
                }
                written => written,
            };

            let generic_arguments =
                self.symbol_generic_argument_bindings(instance.symbol, &instance.arguments)?;

            candidates.push(MemberCandidate {
                symbol,
                owner: instance.symbol,
                space,
                role: member.role,
                ty,
                generic_arguments,
                value: member.value,
                value_type: written,
                steps: ReceiverSteps::new(),
                receiver: None,
            });
        }
        if !candidates.is_empty() {
            return Ok(Answer::Ready(MemberLookup::Found(candidates)));
        }

        // search substituted heritage applications
        for (symbol, arguments) in heritages {
            let mut arguments = arguments;
            for argument in &mut arguments {
                *argument = self.substitute_type(origin.module(), *argument, &substitution)?;
            }
            let heritage = ApparentInstance {
                symbol,
                arguments: arguments.into_iter().collect(),
            };
            let lookup = answer!(
                self.lookup_inherent_symbol_member(origin, receiver, &heritage, space, key)?
            );
            match lookup {
                MemberLookup::Missing => continue,
                lookup => return Ok(Answer::Ready(lookup)),
            }
        }

        Ok(Answer::Ready(MemberLookup::Missing))
    }
}
