use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexSet;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, Decision, MemberCandidate, MemberLookup, MemberRole, Origin, ReceiverSteps,
    answer,
};

/// One active member lookup query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct MemberQuery {
    /// The module whose visibility rules apply.
    module: ModuleId,
    /// The receiver type retained in selected resolutions.
    receiver: dir::GlobalTypeId,
    /// The type currently searched for matching members.
    lookup_type: dir::GlobalTypeId,
    /// The member namespace.
    space: dir::MemberSpace,
    /// The member key.
    key: dir::StaticKey,
    /// Whether extension members are searched.
    extensions: ExtensionSearch,
}

/// Whether member lookup searches extension declarations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ExtensionSearch {
    /// Search receiver and base declarations only.
    Inherent,
    /// Search inherent members first, then extensions.
    All,
}

impl CheckState<'_> {
    /// Return the member space implied by one receiver expression.
    pub(in crate::check) fn member_receiver_space(
        &mut self,
        receiver: dir::GlobalNodeIdAny,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::MemberSpace> {
        if matches!(self.ty(ty)?, dir::Type::Reference(_)) {
            return Ok(dir::MemberSpace::Static);
        }

        let symbol = match self.decision(receiver) {
            Some(Decision::Name(resolution)) => match resolution.symbols() {
                [symbol] => Some(*symbol),
                _ => None,
            },
            Some(Decision::Instantiation(resolution)) => Some(resolution.symbol),
            _ => None,
        };
        let Some(symbol) = symbol else {
            return Ok(dir::MemberSpace::Instance);
        };

        let symbol = self.resolve_symbol_alias(symbol)?;
        let kind = self.symbol_kind(symbol);

        // a name that spells a type reaches its static members, so
        // parameters serve bound statics like rustc's T::default()
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
        let mut active_queries = IndexSet::new();

        self.lookup_member_query(
            origin,
            module,
            receiver,
            space,
            key,
            ExtensionSearch::All,
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
        let mut active_queries = IndexSet::new();

        self.lookup_member_query(
            origin,
            module,
            receiver,
            space,
            key,
            ExtensionSearch::Inherent,
            &mut active_queries,
        )
    }

    /// Look up one member while tracking active receiver queries.
    fn lookup_member_query(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
        extensions: ExtensionSearch,
        active: &mut IndexSet<MemberQuery>,
    ) -> CompilerResult<Answer<MemberLookup>> {
        // static space dispatches on the written reference: alias
        // expansion would erase which declaration the name names
        let root = self.settled_root(receiver)?;
        if space == dir::MemberSpace::Static
            && let dir::Type::Reference(reference) = self.ty(root)?
        {
            return self.lookup_declaration_member(
                origin, module, reference, space, key, extensions, active,
            );
        }

        let receiver = match self.reduce_type_head(origin, receiver)? {
            Answer::Ready(receiver) => receiver,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };

        self.lookup_member_query_at(
            origin, module, receiver, receiver, space, key, extensions, active,
        )
    }

    /// Look up one member while retaining the selected receiver.
    fn lookup_member_query_at(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        lookup_type: dir::GlobalTypeId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
        extensions: ExtensionSearch,
        active: &mut IndexSet<MemberQuery>,
    ) -> CompilerResult<Answer<MemberLookup>> {
        let lookup_type = match self.reduce_type_head(origin, lookup_type)? {
            Answer::Ready(lookup_type) => lookup_type,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };

        // stop cyclic paths through constraints, unions, and heritage
        let query = MemberQuery {
            module,
            receiver,
            lookup_type,
            space,
            key,
            extensions,
        };
        if !active.insert(query) {
            return Ok(Answer::Ready(MemberLookup::Missing));
        }

        let lookup = self.lookup_member_receiver(
            origin,
            module,
            receiver,
            lookup_type,
            space,
            key,
            extensions,
            active,
        );
        active.swap_remove(&query);

        lookup
    }

    /// Look up one member on an already reduced receiver type.
    fn lookup_member_receiver(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        lookup_type: dir::GlobalTypeId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
        extensions: ExtensionSearch,
        active: &mut IndexSet<MemberQuery>,
    ) -> CompilerResult<Answer<MemberLookup>> {
        match self.ty(lookup_type)? {
            // memory forms look through their payloads
            dir::Type::Form(role) => {
                let value = role.value;

                self.lookup_member_query_at(
                    origin, module, receiver, value, space, key, extensions, active,
                )
            }

            // declaration references search static members
            dir::Type::Reference(reference) => self.lookup_declaration_member(
                origin, module, reference, space, key, extensions, active,
            ),

            // applied declarations search their definition members
            dir::Type::Instance(_)
            | dir::Type::Literal(_)
            | dir::Type::Primitive(_)
            | dir::Type::Array(_)
            | dir::Type::Slice(_)
            | dir::Type::FixedArray(_) => {
                let lookup = answer!(self.lookup_apparent_instance_member(
                    origin,
                    module,
                    receiver,
                    lookup_type,
                    space,
                    key,
                    extensions,
                )?);

                // newtypes dereference to their backing for missing members
                if matches!(lookup, MemberLookup::Missing)
                    && let Some(projection) =
                        answer!(self.newtype_backing_projection(origin, lookup_type)?)
                {
                    let value = projection.ty();
                    let receiver = answer!(self.replace_beneath_forms(origin, receiver, value)?);
                    let mut lookup = answer!(self.lookup_member_query_at(
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
            dir::Type::EnumMember(member) => self.lookup_member_query_at(
                origin,
                module,
                receiver,
                member.owner,
                space,
                key,
                extensions,
                active,
            ),

            // generic parameters search through their bounds
            dir::Type::Parameter(parameter) => {
                let bounds = self.parameter_bounds(origin, parameter)?;

                self.lookup_bound_member(
                    origin, module, receiver, &bounds, space, key, extensions, active,
                )
            }

            // erased values expose their constraint's members
            dir::Type::Dynamic(dynamic) => self.lookup_bound_member(
                origin,
                module,
                receiver,
                &[dynamic.constraint],
                space,
                key,
                extensions,
                active,
            ),

            // structural shapes expose their fields
            dir::Type::Shape(shape) => {
                let field = self
                    .shape_fields(lookup_type.module_id, shape.fields)?
                    .iter()
                    .find(|field| field.key == key)
                    .map(|field| field.ty);

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
                    .tuple_elements(lookup_type.module_id, tuple.elements)?
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
                let elements = self
                    .type_ids(lookup_type.module_id, union.elements)?
                    .to_vec();

                self.lookup_union_member(origin, module, &elements, space, key, extensions, active)
            }

            // intersections expose each element's members
            dir::Type::Intersection(intersection) => {
                let elements = self
                    .type_ids(lookup_type.module_id, intersection.elements)?
                    .to_vec();
                for element in elements {
                    let element = self.settled_root(element)?;
                    match self.lookup_member_query(
                        origin, module, element, space, key, extensions, active,
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
        extensions: ExtensionSearch,
        active: &mut IndexSet<MemberQuery>,
    ) -> CompilerResult<Answer<MemberLookup>> {
        for bound in bounds {
            let lookup = self.lookup_member_query_at(
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
        extensions: ExtensionSearch,
    ) -> CompilerResult<Answer<MemberLookup>> {
        let Some((instance_module, instance)) = self.apparent_instance(lookup_type)? else {
            return Ok(Answer::Ready(MemberLookup::Missing));
        };

        self.lookup_symbol_member(
            origin,
            module,
            receiver,
            instance_module,
            instance,
            space,
            key,
            extensions,
        )
    }

    /// Return one instance's newtype backing with its arguments applied.
    pub(in crate::check) fn newtype_backing(
        &mut self,
        origin: Origin,
        lookup_type: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        match answer!(self.newtype_backing_projection(origin, lookup_type)?) {
            Some(projection) => Ok(Answer::Ready(Some(projection.ty()))),
            None => Ok(Answer::Ready(None)),
        }
    }

    /// Return the payload projection behind one newtype instance.
    pub(in crate::check) fn newtype_backing_projection(
        &mut self,
        origin: Origin,
        lookup_type: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::Projection>>> {
        let Some((instance_module, mut instance)) = self.apparent_instance(lookup_type)? else {
            return Ok(Answer::Ready(None));
        };

        // resolve the defining newtype through aliases and imports
        instance.symbol = self.resolve_symbol_alias(instance.symbol)?;
        if !self.is_component_module(instance.symbol.module_id) {
            self.import_external_module(instance.symbol.module_id)?;
        }
        let Some(dir::Definition::Newtype(definition)) = self.definition(instance.symbol) else {
            return Ok(Answer::Ready(None));
        };
        let value = definition.value;

        // apply the instance arguments to the declared backing
        let substitution = self.instance_substitution(instance_module, &instance)?;
        let value = self.substitute_type(origin.module(), value, &substitution)?;

        let arguments = self.type_ids(instance_module, instance.arguments)?.to_vec();
        Ok(Answer::Ready(Some(dir::Projection::NewtypePayload {
            symbol: instance.symbol,
            generic_arguments: self
                .symbol_generic_argument_bindings(instance.symbol, &arguments)?,
            ty: value,
        })))
    }

    /// Look up one static member on a declaration reference.
    fn lookup_declaration_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        reference: dir::TypeReference,
        space: dir::MemberSpace,
        key: dir::StaticKey,
        extensions: ExtensionSearch,
        active: &mut IndexSet<MemberQuery>,
    ) -> CompilerResult<Answer<MemberLookup>> {
        if space != dir::MemberSpace::Static {
            return Ok(Answer::Ready(MemberLookup::Missing));
        }

        // resolve aliases before reading declaration members
        let mut symbol = self.resolve_symbol_alias(reference.symbol)?;

        // a type alias names its body's root declaration for statics,
        // and the body's own members serve whatever the root lacks;
        // head reduction expands the whole alias chain in one step
        let mut alias_body = None;
        if let Some(dir::Definition::TypeAlias(alias)) = self.definition(symbol) {
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
            let inherent = answer!(self.lookup_inherent_declaration_member(origin, symbol, key)?);
            lookup = match inherent {
                MemberLookup::Found(_) | MemberLookup::Field(_) => {
                    return Ok(Answer::Ready(inherent));
                }
                MemberLookup::Missing => match extensions {
                    ExtensionSearch::All => {
                        answer!(self.lookup_static_extension_member(origin, module, symbol, key)?)
                    }
                    ExtensionSearch::Inherent => MemberLookup::Missing,
                },
            };
        }

        // aliased bodies answer whatever the root declaration lacks
        if matches!(lookup, MemberLookup::Missing)
            && let Some(body) = alias_body
        {
            return self.lookup_member_query(origin, module, body, space, key, extensions, active);
        }

        Ok(Answer::Ready(lookup))
    }

    /// Join member lookups across union elements.
    fn lookup_union_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        elements: &[dir::GlobalTypeId],
        space: dir::MemberSpace,
        key: dir::StaticKey,
        extensions: ExtensionSearch,
        active: &mut IndexSet<MemberQuery>,
    ) -> CompilerResult<Answer<MemberLookup>> {
        let mut candidates = Vec::new();
        let mut fields = SmallVec::<[dir::GlobalTypeId; 4]>::new();

        // every element must expose the member
        for element in elements {
            match answer!(
                self.lookup_member_query(origin, module, *element, space, key, extensions, active)?
            ) {
                MemberLookup::Field(ty) => fields.push(ty),
                MemberLookup::Found(found) => candidates.extend(found),
                MemberLookup::Missing => return Ok(Answer::Ready(MemberLookup::Missing)),
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
        instance_module: ModuleId,
        instance: dir::GenericInstance,
        space: dir::MemberSpace,
        key: dir::StaticKey,
        extensions: ExtensionSearch,
    ) -> CompilerResult<Answer<MemberLookup>> {
        let mut instance = instance;
        instance.symbol = self.resolve_symbol_alias(instance.symbol)?;
        if !self.is_component_module(instance.symbol.module_id) {
            self.import_external_module(instance.symbol.module_id)?;
        }

        // search inherent members before extensions
        let inherent = answer!(self.lookup_inherent_symbol_member(
            origin,
            module,
            receiver,
            instance_module,
            &instance,
            space,
            key
        )?);
        match inherent {
            MemberLookup::Found(_) | MemberLookup::Field(_) => {
                return Ok(Answer::Ready(inherent));
            }
            MemberLookup::Missing => {}
        }

        match extensions {
            ExtensionSearch::All => {
                // extension targets name values, so receivers shed memory forms
                let receiver = answer!(self.value_beneath_forms(origin, receiver)?);
                let lookup = answer!(
                    self.lookup_extension_member(origin, module, receiver, &instance, space, key)?
                );

                // values also match targets naming their apparent owner,
                // so primitives reach extensions of their owning class
                if matches!(lookup, MemberLookup::Missing) {
                    let apparent = self.apparent_type(receiver)?;
                    if apparent != receiver {
                        return self.lookup_extension_member(
                            origin, module, apparent, &instance, space, key,
                        );
                    }
                }

                Ok(Answer::Ready(lookup))
            }
            ExtensionSearch::Inherent => Ok(Answer::Ready(MemberLookup::Missing)),
        }
    }

    /// Look up one inherent static member on a declaration reference.
    fn lookup_inherent_declaration_member(
        &mut self,
        origin: Origin,
        symbol: dir::GlobalSymbolId,
        key: dir::StaticKey,
    ) -> CompilerResult<Answer<MemberLookup>> {
        let Some(definition) = self.definition(symbol) else {
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
            let ty = self.resolve_type_variables(origin.module(), ty)?;
            let ty = answer!(self.projected_member_type(origin, None, member.role, ty)?);

            candidates.push(MemberCandidate {
                symbol: member.symbol,
                owner: symbol,
                role: member.role,
                ty,
                generic_arguments: Vec::new(),
                value: member.value,
                value_type: written,
                steps: ReceiverSteps::new(),
            });
        }

        Ok(Answer::Ready(MemberLookup::from_candidates(candidates)))
    }

    /// Look up one inherent member on a declaration, walking its heritage.
    fn lookup_inherent_symbol_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        instance_module: ModuleId,
        instance: &dir::GenericInstance,
        space: dir::MemberSpace,
        key: dir::StaticKey,
    ) -> CompilerResult<Answer<MemberLookup>> {
        // collect own members and heritage applications
        let Some(definition) = self.definition(instance.symbol) else {
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
        let substitution = self
            .instance_substitution(instance_module, instance)?
            .with_receiver(receiver);
        let mut candidates = Vec::new();
        for member in members {
            let Some(member) = answer!(self.declared_member(&member)?) else {
                continue;
            };
            let symbol = member.symbol;
            let Some(ty) = member.ty else {
                continue;
            };

            let ty = self.substitute_type(origin.module(), ty, &substitution)?;
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

            let instance_arguments = self.type_ids(instance_module, instance.arguments)?.to_vec();
            let generic_arguments =
                self.symbol_generic_argument_bindings(instance.symbol, &instance_arguments)?;

            candidates.push(MemberCandidate {
                symbol,
                owner: instance.symbol,
                role: member.role,
                ty,
                generic_arguments,
                value: member.value,
                value_type: written,
                steps: ReceiverSteps::new(),
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
            let arguments = self.intern_type_ids(module, &arguments)?;

            let heritage = dir::GenericInstance { symbol, arguments };
            let lookup = answer!(self.lookup_inherent_symbol_member(
                origin, module, receiver, module, &heritage, space, key
            )?);
            match lookup {
                MemberLookup::Missing => continue,
                lookup => return Ok(Answer::Ready(lookup)),
            }
        }

        Ok(Answer::Ready(MemberLookup::Missing))
    }
}
