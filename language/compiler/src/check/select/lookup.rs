use destack_core::FxIndexSet;
use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    Answer, ApparentInstance, BodyState, DecisionKind, FieldLookup, LookupReceiver,
    MemberArmLookup, MemberCandidate, MemberLookup, MemberRole, Origin, ReceiverSteps,
    TypeArgumentInference, TypeSubstitution, answer,
};
use crate::{CompilerError, CompilerResult};

/// One member lookup on the active recursion path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct MemberLookupKey {
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
pub(super) enum ExtensionFilter {
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
                None => {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "member receiver {receiver:?} has a name decision without a resolution"
                        ),
                    });
                }
            },
            Some(DecisionKind::Instantiation) => {
                let Some(resolution) = resolutions.instantiation_resolution(receiver) else {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "member receiver {receiver:?} has an instantiation decision without a resolution"
                        ),
                    });
                };

                Some(resolution.symbol)
            }
            _ => None,
        };
        let Some(symbol) = symbol else {
            return Ok(dir::MemberSpace::Instance);
        };

        let symbol = self.resolve_symbol_alias(symbol)?;
        let kind = self.symbol_kind(symbol)?;

        // select static members for names that resolve to types
        let is_type_name =
            kind.is_nominal() || matches!(kind, dir::SymbolKind::GenericTypeParameter);
        let space = if is_type_name {
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
        subject: dir::MemberSubject,
        key: dir::StaticKey,
    ) -> CompilerResult<Answer<MemberLookup>> {
        let mut active_queries = FxIndexSet::default();

        self.lookup_subject_member(
            origin,
            module,
            subject.receiver,
            subject.target,
            subject.space,
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
        active: &mut FxIndexSet<MemberLookupKey>,
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
        let query = MemberLookupKey {
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
        active: &mut FxIndexSet<MemberLookupKey>,
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
                    return Ok(Answer::Ready(MemberLookup::Field(FieldLookup {
                        receiver: dir::MemberReceiver::direct(receiver),
                        owner: subject,
                        access: dir::PropertyAccess::Read(refined.value),
                        is_optional: false,
                    })));
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

            // precise Tagged variants read through their selected backing case
            dir::Type::Variant(variant) => {
                let Some(backing) = self.tagged_variant_backing(module, &variant)? else {
                    return self.lookup_apparent_instance_member(
                        origin, module, receiver, subject, space, key, extensions,
                    );
                };
                let projected = answer!(self.replace_form_value(origin, receiver, backing)?);
                let mut lookup = answer!(self.lookup_subject_member(
                    origin, module, projected, backing, space, key, extensions, active,
                )?);
                let Some(adjustment) =
                    self.variant_receiver_adjustment(&variant, backing, projected)?
                else {
                    return Err(CompilerError::Internal {
                        message: "Tagged variant has no payload projection".to_string(),
                    });
                };

                // preserve the selected case before any deeper receiver projection
                lookup.prepend_adjustment(adjustment);

                Ok(Answer::Ready(lookup))
            }

            // applied declarations read their definition members
            dir::Type::Application(_)
            | dir::Type::Literal(_)
            | dir::Type::Primitive(_)
            | dir::Type::Array(_)
            | dir::Type::Slice(_)
            | dir::Type::FixedArray(_) => {
                // widen literal subjects to their carrier before lookup
                let subject = match self.ty(subject)? {
                    dir::Type::Literal(literal) => self.intern_type(literal.widen())?,
                    _ => subject,
                };
                let lookup = answer!(self.lookup_apparent_instance_member(
                    origin, module, receiver, subject, space, key, extensions,
                )?);

                // expose members shared by every precise Tagged variant
                if matches!(lookup, MemberLookup::Missing)
                    && space == dir::MemberSpace::Instance
                    && let dir::Type::Application(instance) = self.ty(subject)?
                    && matches!(
                        self.definition(instance.symbol)?,
                        Some(dir::Definition::Newtype(definition)) if definition.is_tagged()
                    )
                {
                    let Some(variants) = self.variant_types(module, subject)? else {
                        return Err(CompilerError::Internal {
                            message: format!("Tagged owner {subject:?} has no variants"),
                        });
                    };

                    return self.lookup_union_member(
                        origin, module, receiver, subject, &variants, space, key, extensions,
                        active,
                    );
                }

                // newtypes dereference to their backing for missing members
                if matches!(lookup, MemberLookup::Missing)
                    && let Some(instance) = self.newtype_payload(origin, subject)?
                {
                    let value = instance.backing;
                    let receiver = answer!(self.replace_form_value(origin, receiver, value)?);
                    let adjustment = instance.into_receiver_adjustment(receiver);
                    let mut lookup = answer!(self.lookup_subject_member(
                        origin, module, receiver, value, space, key, extensions, active,
                    )?);

                    // record the payload adjustment before deeper receiver steps
                    lookup.prepend_adjustment(adjustment);

                    return Ok(Answer::Ready(lookup));
                }

                Ok(Answer::Ready(lookup))
            }

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
                let dispatch = dir::DynamicDispatch {
                    receiver: dir::AdjustedReceiver::direct(receiver),
                    constraint,
                };
                let constraint_receiver =
                    answer!(self.replace_form_value(origin, receiver, constraint)?);
                let mut lookup = answer!(self.lookup_bound_member(
                    origin,
                    module,
                    constraint_receiver,
                    &[constraint],
                    space,
                    key,
                    ExtensionFilter::Inherent,
                    active,
                )?);
                lookup.select_dynamic(dispatch);

                // extensions remain direct calls over the erased receiver
                if matches!(lookup, MemberLookup::Missing)
                    && extensions == ExtensionFilter::All
                    && let Some(instance) = self.apparent_instance(constraint)?
                {
                    return self.lookup_extension_member(
                        origin,
                        module,
                        receiver,
                        constraint,
                        instance.symbol,
                        space,
                        key,
                    );
                }

                Ok(Answer::Ready(lookup))
            }

            // structural shapes expose every operation for the selected key
            dir::Type::Shape(shape) | dir::Type::Object(shape) => {
                let properties = self.shape_properties(subject.module_id, shape.properties)?;
                let properties = properties
                    .iter()
                    .filter(|property| property.key == key)
                    .copied()
                    .collect::<SmallVec<[_; 2]>>();
                if properties.is_empty() {
                    return Ok(Answer::Ready(MemberLookup::Missing));
                }

                let mut reads = SmallVec::<[dir::GlobalTypeId; 2]>::new();
                let mut writes = SmallVec::<[dir::GlobalTypeId; 2]>::new();
                let mut is_optional = true;
                for property in properties {
                    is_optional &= property.is_optional;

                    if let Some(read) = property.access.read() {
                        let read = answer!(self.projected_member_type(
                            origin,
                            Some(receiver),
                            MemberRole::Field,
                            read,
                        )?);
                        reads.push(read);
                    }
                    if let Some(write) = property.access.write() {
                        let write = answer!(self.projected_member_type(
                            origin,
                            Some(receiver),
                            MemberRole::Field,
                            write,
                        )?);
                        writes.push(write);
                    }
                }

                let read = match reads.as_slice() {
                    [] => None,
                    [read] => Some(*read),
                    reads => Some(self.normalized_intersection_type(reads.iter().copied())?),
                };
                let write = match writes.as_slice() {
                    [] => None,
                    [write] => Some(*write),
                    writes => Some(self.normalized_intersection_type(writes.iter().copied())?),
                };
                let access = match (read, write) {
                    (Some(read), Some(write)) => dir::PropertyAccess::ReadWrite { read, write },
                    (Some(read), None) => dir::PropertyAccess::Read(read),
                    (None, Some(write)) => dir::PropertyAccess::Write(write),
                    (None, None) => {
                        return Err(CompilerError::Internal {
                            message: format!("structural property {key:?} has no operation"),
                        });
                    }
                };

                Ok(Answer::Ready(MemberLookup::Field(FieldLookup {
                    receiver: dir::MemberReceiver::direct(receiver),
                    owner: subject,
                    access,
                    is_optional,
                })))
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
                    .copied();

                match element {
                    Some(element) => match self.projected_member_type(
                        origin,
                        Some(receiver),
                        MemberRole::Field,
                        element.ty,
                    )? {
                        Answer::Ready(access_type) => {
                            Ok(Answer::Ready(MemberLookup::Field(FieldLookup {
                                receiver: dir::MemberReceiver::direct(receiver),
                                owner: subject,
                                access: if element.is_readonly {
                                    dir::PropertyAccess::Read(access_type)
                                } else {
                                    dir::PropertyAccess::ReadWrite {
                                        read: access_type,
                                        write: access_type,
                                    }
                                },
                                is_optional: element.is_optional,
                            })))
                        }
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
                let mut lookups = Vec::with_capacity(elements.len());
                for element in elements {
                    let element = self.settled_root(element)?;
                    match self.lookup_subject_member(
                        origin, module, receiver, element, space, key, extensions, active,
                    )? {
                        Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
                        Answer::Ready(MemberLookup::Missing) => continue,
                        Answer::Ready(MemberLookup::Intersection(nested)) => {
                            lookups.extend(nested);
                        }
                        Answer::Ready(lookup) => lookups.push(lookup),
                    }
                }

                let lookup = if lookups.is_empty() {
                    MemberLookup::Missing
                } else if lookups.len() == 1 {
                    lookups.remove(0)
                } else {
                    MemberLookup::Intersection(lookups)
                };

                Ok(Answer::Ready(lookup))
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
        active: &mut FxIndexSet<MemberLookupKey>,
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

        self.lookup_symbol_member(
            origin,
            module,
            receiver,
            lookup_type,
            instance,
            space,
            key,
            extensions,
        )
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
        active: &mut FxIndexSet<MemberLookupKey>,
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
        if !self.is_own_module(symbol.module_id) {
            self.import_external_module(symbol.module_id)?;
        }

        // search declaration members before extensions
        let mut lookup = MemberLookup::Missing;
        if !self.symbol_kind(symbol)?.is_type_alias() {
            let inherent =
                answer!(self.lookup_inherent_declaration_member(origin, receiver, symbol, key)?);
            lookup = match inherent {
                MemberLookup::Found(_)
                | MemberLookup::Field(_)
                | MemberLookup::Union(_)
                | MemberLookup::Intersection(_) => return Ok(Answer::Ready(inherent)),
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
        active: &mut FxIndexSet<MemberLookupKey>,
    ) -> CompilerResult<Answer<MemberLookup>> {
        let mut lookups = Vec::with_capacity(elements.len());

        // runtime receiver unions seal each member under its own arm
        let is_receiver_union = self.settled_root(receiver)? == self.settled_root(subject)?;

        // require every element to expose the member
        for element in elements {
            let arm_receiver = if is_receiver_union {
                *element
            } else {
                receiver
            };
            let lookup = answer!(self.lookup_subject_member(
                origin,
                module,
                arm_receiver,
                *element,
                space,
                key,
                extensions,
                active
            )?);
            if matches!(lookup, MemberLookup::Missing) {
                return Ok(Answer::Ready(MemberLookup::Missing));
            }
            lookups.push(MemberArmLookup {
                receiver: arm_receiver,
                lookup,
            });
        }

        Ok(Answer::Ready(MemberLookup::Union(lookups)))
    }

    /// Look up one member on a declaration reference.
    fn lookup_symbol_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        subject: dir::GlobalTypeId,
        instance: ApparentInstance,
        space: dir::MemberSpace,
        key: dir::StaticKey,
        extensions: ExtensionFilter,
    ) -> CompilerResult<Answer<MemberLookup>> {
        if !self.is_own_module(instance.symbol.module_id) {
            self.import_external_module(instance.symbol.module_id)?;
        }

        // search inherent members before extensions
        let inherent =
            answer!(self.lookup_inherent_symbol_member(origin, receiver, &instance, space, key)?);
        match inherent {
            MemberLookup::Found(_)
            | MemberLookup::Field(_)
            | MemberLookup::Union(_)
            | MemberLookup::Intersection(_) => return Ok(Answer::Ready(inherent)),
            MemberLookup::Missing => {}
        }

        match extensions {
            ExtensionFilter::All => {
                let lookup = answer!(self.lookup_extension_member(
                    origin,
                    module,
                    receiver,
                    subject,
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
                            receiver,
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
        if members.is_empty() {
            return Ok(Answer::Ready(MemberLookup::Missing));
        }

        let mut candidates = Vec::new();

        // collect visible static declaration members
        for member in members {
            let Some(member) = answer!(self.declared_member(&member)?) else {
                continue;
            };
            let Some(ty) = member.ty else {
                continue;
            };
            let mut ty = ty;
            let mut written = self.static_value(member.symbol);
            let mut generic_arguments = Vec::new();

            // instantiate each direct value at its source use
            if !member.role.is_callable()
                && let Some(template) = self.symbol_template(symbol)?
            {
                let parameters = self.generic_template_parameters(template)?;
                let Some(substitution) = answer!(self.instantiate_parameters(
                    origin,
                    &parameters,
                    &[],
                    TypeSubstitution::default().with_receiver(receiver),
                    TypeArgumentInference::Exact,
                )?) else {
                    return Err(CompilerError::Internal {
                        message: format!("declaration template {template:?} cannot instantiate"),
                    });
                };
                for constraint in
                    self.substitute_application_constraints(origin, template, &substitution)?
                {
                    self.check.push_constraint(constraint);
                }
                ty = self.substitute_type(ty, &substitution)?;
                written = written
                    .map(|written| self.substitute_type(written, &substitution))
                    .transpose()?;
                generic_arguments = substitution.bindings.to_vec();
            }

            let callable = member.callable_type(origin.module(), symbol, ty, self)?;
            let access_type = member.access_type(self, ty)?;
            let access_type = answer!(self.projected_member_type(
                origin,
                Some(receiver),
                member.role,
                access_type,
            )?);

            candidates.push(MemberCandidate {
                key,
                symbol: member.symbol,
                owner: symbol,
                origin: dir::MemberOrigin::Declaration,
                space: dir::MemberSpace::Static,
                role: member.role,
                kind: member.kind,
                is_writable: member.is_writable,
                access_type,
                callable,
                is_optional: member.is_optional,
                generic_arguments,
                value: member.value,
                value_type: written,
                receiver: LookupReceiver::Direct(ReceiverSteps::new()),
            });
        }

        Ok(Answer::Ready(MemberLookup::from_candidates(candidates)))
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
            .map(|heritage| heritage.ty)
            .collect::<SmallVec<[_; 2]>>();

        // substitute applied arguments and the receiver value beneath its forms
        let receiver_value = answer!(self.strip_form(origin, receiver)?);
        let substitution = instance.substitution(self)?.with_receiver(receiver_value);
        let mut candidates = Vec::new();
        for member in members {
            let Some(declared) = answer!(self.declared_member(&member)?) else {
                continue;
            };
            let symbol = declared.symbol;

            // value-less associated types project through the receiver, and
            //  defaults stay conformance-only behind rigid owners
            let is_associated = matches!(member, dir::DefinitionMember::AssociatedType(_));
            let is_rigid = self.is_rigid_projection_owner(receiver)?;
            let ty = match declared.ty {
                Some(ty) if !(is_associated && is_rigid) => ty,
                _ if is_associated => {
                    let arguments = self.intern_type_ids(&[])?;

                    self.intern_member(dir::MemberType {
                        owner: receiver,
                        key,
                        arguments,
                        qualifier: None,
                    })?
                }
                _ => continue,
            };
            let member = declared;

            let ty = self.substitute_type(ty, &substitution)?;
            let callable = member.callable_type(origin.module(), instance.symbol, ty, self)?;
            let access_type = member.access_type(self, ty)?;
            let access_type = answer!(self.projected_member_type(
                origin,
                Some(receiver),
                member.role,
                access_type,
            )?);

            // substitute static value types for projections
            let written = match self.static_value(symbol) {
                Some(written) => Some(self.substitute_type(written, &substitution)?),
                written => written,
            };

            let generic_arguments =
                self.symbol_generic_argument_bindings(instance.symbol, &instance.arguments)?;

            candidates.push(MemberCandidate {
                key,
                symbol,
                owner: instance.symbol,
                origin: dir::MemberOrigin::Declaration,
                space,
                role: member.role,
                kind: member.kind,
                is_writable: member.is_writable,
                access_type,
                callable,
                is_optional: member.is_optional,
                generic_arguments,
                value: member.value,
                value_type: written,
                receiver: LookupReceiver::Direct(ReceiverSteps::new()),
            });
        }
        if !candidates.is_empty() {
            return Ok(Answer::Ready(MemberLookup::Found(candidates)));
        }

        // search substituted heritage applications
        for heritage in heritages {
            let heritage = self.substitute_type(heritage, &substitution)?;
            let (heritage_module, heritage) = self.require_nominal_application(heritage)?;
            let arguments = self.type_ids(heritage_module, heritage.arguments)?;
            let heritage = ApparentInstance {
                symbol: heritage.symbol,
                arguments: arguments.iter().copied().collect(),
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

    /// Collect the declared member keys reachable from one lookup subject.
    ///
    /// Keys over-approximate the member set: the keyed lookup judges
    /// applicability, so inapplicable keys resolve as missing lookups.
    pub(in crate::check) fn subject_member_keys(
        &mut self,
        origin: Origin,
        module: ModuleId,
        subject: &dir::MemberSubject,
    ) -> CompilerResult<Vec<dir::StaticKey>> {
        let mut keys = FxIndexSet::default();
        let mut visited = FxIndexSet::default();
        self.collect_subject_keys(
            origin,
            module,
            subject.target,
            subject.space,
            &mut keys,
            &mut visited,
        )?;

        Ok(keys.into_iter().collect())
    }

    /// Collect declared member keys from one subject type.
    fn collect_subject_keys(
        &mut self,
        origin: Origin,
        module: ModuleId,
        subject: dir::GlobalTypeId,
        space: dir::MemberSpace,
        keys: &mut FxIndexSet<dir::StaticKey>,
        visited: &mut FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<()> {
        // stop cyclic paths through bounds, unions, and heritage
        let subject = self.settled_root(subject)?;
        if !visited.insert(subject) {
            return Ok(());
        }

        match self.ty(subject)? {
            // declarations expose their own, inherited, and extension keys
            dir::Type::Reference(reference) => {
                self.collect_symbol_keys(module, reference.symbol, space, keys, visited)?;
            }
            dir::Type::Application(instance) => {
                self.collect_symbol_keys(module, instance.symbol, space, keys, visited)?;
            }
            // structural subjects expose their property keys
            dir::Type::Shape(shape) | dir::Type::Object(shape) => {
                for property in self.shape_properties(subject.module_id, shape.properties)? {
                    keys.insert(property.key);
                }
            }
            // scalar families expose their blanket extension keys
            dir::Type::Primitive(_) | dir::Type::Literal(_) => {
                for extension in self.visible_blanket_extensions(module)? {
                    self.collect_definition_keys(extension, space, keys)?;
                }
            }
            // memory forms expose their pointee keys
            dir::Type::Form(form) => {
                self.collect_subject_keys(origin, module, form.value, space, keys, visited)?;
            }
            // erased subjects expose their constraint keys
            dir::Type::Dynamic(dynamic) => {
                self.collect_subject_keys(
                    origin,
                    module,
                    dynamic.constraint,
                    space,
                    keys,
                    visited,
                )?;
            }
            // parameters expose the keys of their declared bounds
            dir::Type::Parameter(parameter) => {
                for bound in self.parameter_bounds(origin, parameter)? {
                    self.collect_subject_keys(origin, module, bound, space, keys, visited)?;
                }
            }
            // composite subjects expose the keys of every element
            dir::Type::Union(union) => {
                let elements = self.type_ids(subject.module_id, union.elements)?.to_vec();
                for element in elements {
                    self.collect_subject_keys(origin, module, element, space, keys, visited)?;
                }
            }
            dir::Type::Intersection(intersection) => {
                let elements = self
                    .type_ids(subject.module_id, intersection.elements)?
                    .to_vec();
                for element in elements {
                    self.collect_subject_keys(origin, module, element, space, keys, visited)?;
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Collect declared member keys from one nominal declaration.
    fn collect_symbol_keys(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        space: dir::MemberSpace,
        keys: &mut FxIndexSet<dir::StaticKey>,
        visited: &mut FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<()> {
        // collect the declaration's own member keys
        let symbol = self.resolve_symbol_alias(symbol)?;
        self.collect_definition_keys(symbol, space, keys)?;

        // collect inherited keys through the heritage clauses
        let heritages = match self.definition(symbol)? {
            Some(definition) => definition
                .heritages()
                .into_iter()
                .map(|heritage| heritage.ty)
                .collect::<SmallVec<[_; 2]>>(),
            None => SmallVec::new(),
        };
        for heritage in heritages {
            let mut origin_visited = visited.clone();
            self.collect_subject_keys(
                Origin::Symbol(symbol),
                module,
                heritage,
                space,
                keys,
                &mut origin_visited,
            )?;
        }

        // collect extension keys targeting this declaration
        for extension in self.visible_extensions(module, symbol)? {
            self.collect_definition_keys(extension, space, keys)?;
        }

        Ok(())
    }

    /// Collect the member keys declared by one definition.
    fn collect_definition_keys(
        &mut self,
        symbol: dir::GlobalSymbolId,
        space: dir::MemberSpace,
        keys: &mut FxIndexSet<dir::StaticKey>,
    ) -> CompilerResult<()> {
        let Some(definition) = self.definition(symbol)? else {
            return Ok(());
        };
        for member in definition.members() {
            if member.space() == space
                && let Some(key) = member.key()
            {
                keys.insert(key);
            }
        }

        Ok(())
    }
}
