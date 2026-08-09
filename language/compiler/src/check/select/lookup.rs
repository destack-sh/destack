use std::sync::Arc;

use destack_core::{FxIndexMap, FxIndexSet};
use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    ApparentInstance, BodyState, FieldLookup, LookupReceiver, MemberArmLookup, MemberCandidate,
    MemberLookup, MemberRole, Origin, ReceiverSteps, TypeArgumentInference, TypeSubstitution,
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

        let symbol = match self.name_decision(receiver) {
            Some(resolution) => match resolution.symbols() {
                [symbol] => Some(*symbol),
                _ => None,
            },
            None => match self.decision(receiver) {
                Some(dir::Decision::Instantiation(resolution)) => Some(resolution.symbol),
                _ => None,
            },
        };
        let Some(symbol) = symbol else {
            return Ok(dir::MemberSpace::Instance);
        };

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
    ) -> CompilerResult<MemberLookup> {
        // select compiler-defined fields before declaration lookup
        if subject.space == dir::MemberSpace::Instance
            && let Some(projection) =
                self.tagged_discriminator_projection(module, subject.receiver, key)?
        {
            return Ok(MemberLookup::Field(FieldLookup::Projection(projection)));
        }

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
    ) -> CompilerResult<MemberLookup> {
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
    ) -> CompilerResult<MemberLookup> {
        // static names read the written declaration before aliases reduce
        let root = self.shallow_resolve(subject)?;
        if space == dir::MemberSpace::Static
            && let dir::Type::Reference(reference) = self.ty(root)?
        {
            return self.lookup_declaration_member(
                origin, module, receiver, reference, space, key, extensions, active,
            );
        }

        let subject = self.reduce_type_head(origin, subject)?;

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
            return Ok(MemberLookup::Missing);
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
    ) -> CompilerResult<MemberLookup> {
        match self.ty(subject)? {
            // memory forms look through their payloads
            dir::Type::Form(form) => self.lookup_subject_member(
                origin, module, receiver, form.value, space, key, extensions, active,
            ),

            // refinements answer their refined member, then their base
            dir::Type::Refined(refined) => {
                let refined = self.type_refined(subject.module_id, refined)?;
                if refined.key == key {
                    return Ok(MemberLookup::Field(FieldLookup::Structural {
                        receiver: dir::MemberReceiver::direct(receiver),
                        owner: subject,
                        access: dir::PropertyAccess::Read(refined.value),
                        is_optional: false,
                    }));
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
                let projected = self.replace_form_value(origin, receiver, backing)?;
                let mut lookup = self.lookup_subject_member(
                    origin, module, projected, backing, space, key, extensions, active,
                )?;
                let Some(adjustment) =
                    self.variant_receiver_adjustment(&variant, backing, projected)?
                else {
                    return Err(CompilerError::Internal {
                        message: "Tagged variant has no payload projection".to_string(),
                    });
                };

                // preserve the selected case before any deeper receiver projection
                lookup.prepend_adjustment(adjustment)?;

                Ok(lookup)
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
                let lookup = self.lookup_apparent_instance_member(
                    origin, module, receiver, subject, space, key, extensions,
                )?;

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
                    let receiver = self.replace_form_value(origin, receiver, value)?;
                    let adjustment = instance.into_receiver_adjustment(receiver);
                    let mut lookup = self.lookup_subject_member(
                        origin, module, receiver, value, space, key, extensions, active,
                    )?;

                    // record the payload adjustment before deeper receiver steps
                    lookup.prepend_adjustment(adjustment)?;

                    return Ok(lookup);
                }

                Ok(lookup)
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
                let constraint_receiver = self.replace_form_value(origin, receiver, constraint)?;
                let mut lookup = self.lookup_bound_member(
                    origin,
                    module,
                    constraint_receiver,
                    &[constraint],
                    space,
                    key,
                    ExtensionFilter::Inherent,
                    active,
                )?;
                lookup.select_dynamic(dispatch)?;

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

                Ok(lookup)
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
                    return Ok(MemberLookup::Missing);
                }

                let mut reads = SmallVec::<[dir::GlobalTypeId; 2]>::new();
                let mut writes = SmallVec::<[dir::GlobalTypeId; 2]>::new();
                let mut is_optional = true;
                for property in properties {
                    is_optional &= property.is_optional;

                    if let Some(read) = property.access.read() {
                        let read = self.projected_member_type(
                            origin,
                            Some(receiver),
                            MemberRole::Field,
                            read,
                        )?;
                        reads.push(read);
                    }
                    if let Some(write) = property.access.write() {
                        let write = self.projected_member_type(
                            origin,
                            Some(receiver),
                            MemberRole::Field,
                            write,
                        )?;
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

                Ok(MemberLookup::Field(FieldLookup::Structural {
                    receiver: dir::MemberReceiver::direct(receiver),
                    owner: subject,
                    access,
                    is_optional,
                }))
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

                let Some(element) = element else {
                    return Ok(MemberLookup::Missing);
                };
                let access_type = self.projected_member_type(
                    origin,
                    Some(receiver),
                    MemberRole::Field,
                    element.ty,
                )?;
                let access = if element.is_readonly {
                    dir::PropertyAccess::Read(access_type)
                } else {
                    dir::PropertyAccess::ReadWrite {
                        read: access_type,
                        write: access_type,
                    }
                };

                Ok(MemberLookup::Field(FieldLookup::Structural {
                    receiver: dir::MemberReceiver::direct(receiver),
                    owner: subject,
                    access,
                    is_optional: element.is_optional,
                }))
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
                    let element = self.shallow_resolve(element)?;
                    match self.lookup_subject_member(
                        origin, module, receiver, element, space, key, extensions, active,
                    )? {
                        MemberLookup::Missing => continue,
                        MemberLookup::Intersection(nested) => {
                            lookups.extend(nested);
                        }
                        lookup => lookups.push(lookup),
                    }
                }

                let lookup = if lookups.is_empty() {
                    MemberLookup::Missing
                } else if lookups.len() == 1 {
                    lookups.remove(0)
                } else {
                    MemberLookup::Intersection(lookups)
                };

                Ok(lookup)
            }

            _ => Ok(MemberLookup::Missing),
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
    ) -> CompilerResult<MemberLookup> {
        for bound in bounds {
            let lookup = self.lookup_subject_member(
                origin, module, receiver, *bound, space, key, extensions, active,
            )?;
            match lookup {
                MemberLookup::Missing => continue,
                lookup => return Ok(lookup),
            }
        }

        Ok(MemberLookup::Missing)
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
    ) -> CompilerResult<MemberLookup> {
        let Some(instance) = self.apparent_instance(lookup_type)? else {
            return Ok(MemberLookup::Missing);
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
    ) -> CompilerResult<MemberLookup> {
        if space != dir::MemberSpace::Static {
            return Ok(MemberLookup::Missing);
        }

        let mut symbol = reference.symbol;

        // a type alias names its body's root declaration for statics,
        //  and the body's own members serve whatever the root lacks;
        //  head reduction expands the whole alias chain in one step
        let mut alias_body = None;
        if let Some(dir::Definition::TypeAlias(alias)) = self.definition(symbol)? {
            let value = alias.value;
            let head = self.reduce_type_head(origin, value)?;
            alias_body = Some(head);
            if let Some(named) = self.type_symbol(head)? {
                symbol = named;
            }
        }
        if !self.is_own_module(symbol.module_id) {
            self.import_external_module(symbol.module_id)?;
        }

        // search declaration members before extensions
        let mut lookup = MemberLookup::Missing;
        if !self.symbol_kind(symbol)?.is_type_alias() {
            let inherent =
                self.lookup_inherent_declaration_member(origin, receiver, symbol, key)?;
            lookup = match inherent {
                MemberLookup::Found(_)
                | MemberLookup::Field(_)
                | MemberLookup::Union(_)
                | MemberLookup::Intersection(_) => return Ok(inherent),
                MemberLookup::Missing => match extensions {
                    ExtensionFilter::All => {
                        self.lookup_static_extension_member(origin, module, symbol, key)?
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

        Ok(lookup)
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
    ) -> CompilerResult<MemberLookup> {
        let mut lookups = Vec::with_capacity(elements.len());

        // runtime receiver unions settle each member under its own arm
        let is_receiver_union = self.shallow_resolve(receiver)? == self.shallow_resolve(subject)?;

        // require every element to expose the member
        for element in elements {
            let arm_receiver = if is_receiver_union {
                *element
            } else {
                receiver
            };
            let lookup = self.lookup_subject_member(
                origin,
                module,
                arm_receiver,
                *element,
                space,
                key,
                extensions,
                active,
            )?;
            if matches!(lookup, MemberLookup::Missing) {
                return Ok(MemberLookup::Missing);
            }
            lookups.push(MemberArmLookup {
                receiver: arm_receiver,
                lookup,
            });
        }

        Ok(MemberLookup::Union(lookups))
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
    ) -> CompilerResult<MemberLookup> {
        if !self.is_own_module(instance.symbol.module_id) {
            self.import_external_module(instance.symbol.module_id)?;
        }

        // search inherent members first: closed subjects derive the asked
        //  key from the owner's flattened surface, open subjects search live
        let closed = !self.type_flags(subject)?.has_variable();
        let inherent = if closed {
            self.stored_member_lookup(origin, receiver, &instance, space, key)?
        } else {
            self.lookup_inherent_symbol_member(origin, receiver, &instance, space, key)?
        };
        if inherent.is_found() {
            return Ok(inherent);
        }

        // search associated members before extensions
        let associated = self.lookup_associated_member(origin, receiver, space, key)?;
        if associated.is_found() {
            return Ok(associated);
        }

        match extensions {
            ExtensionFilter::All => {
                let lookup = self.lookup_extension_member(
                    origin,
                    module,
                    receiver,
                    subject,
                    instance.symbol,
                    space,
                    key,
                )?;

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

                Ok(lookup)
            }
            ExtensionFilter::Inherent => Ok(MemberLookup::Missing),
        }
    }

    /// Look up one associated member through its uniquely declaring interface.
    fn lookup_associated_member(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        if space != dir::MemberSpace::Static {
            return Ok(MemberLookup::Missing);
        }
        let Some(qualifier) = self.select_associated_qualifier(origin, receiver, key)? else {
            return Ok(MemberLookup::Missing);
        };

        // search the interface application selected for this projection
        let (module, qualifier) = self.nominal_application(qualifier)?;
        let arguments = self.type_ids(module, qualifier.arguments)?;
        let qualifier = ApparentInstance {
            symbol: qualifier.symbol,
            arguments: arguments.iter().copied().collect(),
        };

        self.lookup_inherent_symbol_member(origin, receiver, &qualifier, space, key)
    }

    /// Look up one inherent static member on a declaration reference.
    fn lookup_inherent_declaration_member(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        symbol: dir::GlobalSymbolId,
        key: dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        let Some(definition) = self.definition(symbol)? else {
            return Ok(MemberLookup::Missing);
        };
        let members = definition
            .members_with_key(dir::MemberSpace::Static, key)
            .cloned()
            .collect::<SmallVec<[_; 2]>>();
        if members.is_empty() {
            return Ok(MemberLookup::Missing);
        }

        let mut candidates = Vec::new();

        // collect visible static declaration members
        for member in members {
            let Some(member) = self.declared_member(&member)? else {
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
                let Some(substitution) = self.instantiate_parameters(
                    origin,
                    &parameters,
                    &[],
                    TypeSubstitution::default().with_receiver(receiver),
                    TypeArgumentInference::Exact,
                )?
                else {
                    return Err(CompilerError::Internal {
                        message: format!("declaration template {template:?} cannot instantiate"),
                    });
                };
                for constraint in
                    self.substitute_application_constraints(origin, template, &substitution)?
                {
                    self.check.push_constraint(constraint)?;
                }
                ty = self.substitute_type(ty, &substitution)?;
                written = written
                    .map(|written| self.substitute_type(written, &substitution))
                    .transpose()?;
                generic_arguments = substitution.bindings.to_vec();
            }

            let callable = member.callable_type(origin.module(), symbol, ty, self)?;
            let access_type = member.access_type(self, ty)?;
            let access_type =
                self.projected_member_type(origin, Some(receiver), member.role, access_type)?;

            candidates.push(MemberCandidate {
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

        Ok(MemberLookup::from_candidates(candidates))
    }

    /// Look up one inherent member on a declaration, walking its heritage.
    fn lookup_inherent_symbol_member(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        instance: &ApparentInstance,
        space: dir::MemberSpace,
        key: dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        // collect own members and heritage applications
        let Some(definition) = self.definition(instance.symbol)? else {
            return Ok(MemberLookup::Missing);
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
        let receiver_value = self.strip_form(origin, receiver)?;
        let substitution = instance.substitution(self)?.with_receiver(receiver_value);
        let candidates = self.instance_member_candidates(
            origin,
            receiver,
            instance,
            &substitution,
            &members,
            space,
            key,
        )?;
        if !candidates.is_empty() {
            return Ok(MemberLookup::Found(candidates));
        }

        // search substituted heritage applications
        for heritage in heritages {
            let heritage = self.substitute_type(heritage, &substitution)?;
            let (heritage_module, heritage) = self.nominal_application(heritage)?;
            let arguments = self.type_ids(heritage_module, heritage.arguments)?;
            let heritage = ApparentInstance {
                symbol: heritage.symbol,
                arguments: arguments.iter().copied().collect(),
            };
            let lookup =
                self.lookup_inherent_symbol_member(origin, receiver, &heritage, space, key)?;
            match lookup {
                MemberLookup::Missing => continue,
                lookup => return Ok(lookup),
            }
        }

        Ok(MemberLookup::Missing)
    }

    /// Return one closed subject's inherent members, grouped by key.
    ///
    /// Levels walk in declaration preorder, so own members claim their keys
    /// first and each heritage level serves only the keys nearer levels left
    /// open, matching the per key search order.
    pub(in crate::check) fn inherent_member_table(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        instance: &ApparentInstance,
        space: dir::MemberSpace,
    ) -> CompilerResult<Arc<FxIndexMap<dir::StaticKey, MemberLookup>>> {
        // derive the table from the owner's stored member bindings
        if let Some(bindings) = self.stored_member_bindings(instance.symbol, space) {
            let mut table = FxIndexMap::default();
            for binding in bindings {
                let candidates =
                    self.binding_member_candidates(origin, receiver, instance, &binding, space)?;
                table.insert(binding.key, MemberLookup::Found(candidates));
            }

            return Ok(Arc::new(table));
        }

        // substitute the receiver value once for every level
        let receiver_value = self.strip_form(origin, receiver)?;

        // walk the declaration levels in preorder
        let mut table = FxIndexMap::<dir::StaticKey, MemberLookup>::default();
        let mut stack = vec![instance.clone()];
        let mut visited = FxIndexSet::default();
        while let Some(level) = stack.pop() {
            if !visited.insert((level.symbol, level.arguments.clone())) {
                continue;
            }
            let Some(definition) = self.definition(level.symbol)? else {
                continue;
            };

            // collect the keys this level declares in the searched space
            let mut keys = Vec::new();
            for member in definition.members() {
                let Some(key) = member.key() else {
                    continue;
                };
                if member.space() == space && !table.contains_key(&key) && !keys.contains(&key) {
                    keys.push(key);
                }
            }
            let heritages = definition
                .bases()
                .iter()
                .map(|heritage| heritage.ty)
                .collect::<SmallVec<[_; 2]>>();

            // build the level's candidates for each unclaimed key
            let substitution = level.substitution(self)?.with_receiver(receiver_value);
            for key in keys {
                let members = self
                    .definition(level.symbol)?
                    .map(|definition| {
                        definition
                            .members_with_key(space, key)
                            .cloned()
                            .collect::<SmallVec<[_; 2]>>()
                    })
                    .unwrap_or_default();
                let candidates = self.instance_member_candidates(
                    origin,
                    receiver,
                    &level,
                    &substitution,
                    &members,
                    space,
                    key,
                )?;
                if !candidates.is_empty() {
                    table.insert(key, MemberLookup::Found(candidates));
                }
            }

            // push heritage levels in reverse for preorder traversal
            for heritage in heritages.into_iter().rev() {
                let heritage = self.substitute_type(heritage, &substitution)?;
                let (heritage_module, heritage) = self.nominal_application(heritage)?;
                let arguments = self.type_ids(heritage_module, heritage.arguments)?;
                stack.push(ApparentInstance {
                    symbol: heritage.symbol,
                    arguments: arguments.iter().copied().collect(),
                });
            }
        }

        Ok(Arc::new(table))
    }

    /// Build one owner's canonical member bindings with `this` symbolic.
    ///
    /// Bindings substitute each heritage level's arguments but never a
    /// receiver, so use sites substitute their own receivers and the
    /// stored types stay canonical.
    pub(in crate::check) fn canonical_member_bindings(
        &mut self,
        origin: Origin,
        instance: &ApparentInstance,
        space: dir::MemberSpace,
    ) -> CompilerResult<Option<Vec<dir::MemberBinding>>> {
        let mut bindings: Vec<dir::MemberBinding> = Vec::new();
        let mut stack = vec![instance.clone()];
        let mut visited = FxIndexSet::default();

        // walk the declaration levels in preorder, first level per key wins
        while let Some(level) = stack.pop() {
            if !visited.insert((level.symbol, level.arguments.clone())) {
                continue;
            }
            let Some(definition) = self.definition(level.symbol)?.cloned() else {
                return Ok(None);
            };
            let substitution = level.substitution(self.check)?;

            // bind each declared member the nearer levels left open
            for member in definition.members() {
                let Some(key) = member.key() else {
                    continue;
                };
                if member.space() != space {
                    continue;
                }
                let Some(declared) = self.declared_member(&member.clone())? else {
                    continue;
                };
                let ty = match declared.ty {
                    Some(ty) => ty,
                    // valueless associated members project through receivers
                    None if declared.role == MemberRole::Associated => {
                        let arguments = self.intern_type_ids(&[])?;
                        let owner = level.intern(origin.module(), self.check)?;

                        self.intern_member(dir::MemberType {
                            owner,
                            key,
                            arguments,
                            qualifier: None,
                        })?
                    }
                    None => continue,
                };
                let ty = self.substitute_type(ty, &substitution)?;
                let callable = declared.callable_type(origin.module(), level.symbol, ty, self)?;
                let access_type = declared.access_type(self.check, ty)?;
                let access = match declared.role {
                    MemberRole::Setter => dir::PropertyAccess::Write(access_type),
                    _ if declared.is_writable => dir::PropertyAccess::ReadWrite {
                        read: access_type,
                        write: access_type,
                    },
                    _ => dir::PropertyAccess::Read(access_type),
                };
                let declaration = dir::MemberDeclaration {
                    symbol: declared.symbol,
                    owner: level.symbol,
                    origin: dir::MemberOrigin::Declaration,
                    role: declared.role,
                    callable_type: callable,
                };

                // overloads and accessor pairs extend their key in place
                if let Some(binding) = bindings.iter_mut().find(|binding| binding.key == key) {
                    let same_level = binding
                        .declarations
                        .first()
                        .is_some_and(|first| first.owner == level.symbol);
                    let known = binding
                        .declarations
                        .iter()
                        .any(|previous| previous.symbol == declared.symbol);
                    if same_level && !known {
                        // an accessor pair joins its counterpart's access
                        binding.access = match (binding.access.read(), declared.role) {
                            (Some(read), MemberRole::Setter) => {
                                match binding.access.write().or(access.write()) {
                                    Some(write) => dir::PropertyAccess::ReadWrite { read, write },
                                    None => binding.access,
                                }
                            }
                            (None, MemberRole::Getter) => match access.read() {
                                Some(read) => match binding.access.write() {
                                    Some(write) => dir::PropertyAccess::ReadWrite { read, write },
                                    None => dir::PropertyAccess::Read(read),
                                },
                                None => binding.access,
                            },
                            _ => binding.access,
                        };
                        binding.declarations.push(declaration);
                    }

                    continue;
                }

                bindings.push(dir::MemberBinding::new(
                    key,
                    declared.kind,
                    access,
                    declared.is_optional,
                    vec![declaration],
                ));
            }

            // push substituted heritage levels in reverse for preorder
            let heritages = definition
                .bases()
                .iter()
                .map(|heritage| heritage.ty)
                .collect::<SmallVec<[_; 2]>>();
            for heritage in heritages.into_iter().rev() {
                let heritage = self.substitute_type(heritage, &substitution)?;
                let Some((heritage_module, heritage)) = self.nominal_application_maybe(heritage)?
                else {
                    continue;
                };
                let arguments = self.type_ids(heritage_module, heritage.arguments)?;
                stack.push(ApparentInstance {
                    symbol: heritage.symbol,
                    arguments: arguments.iter().copied().collect(),
                });
            }
        }

        Ok(Some(bindings))
    }

    /// Return the applied heritage level of one declaring owner.
    fn heritage_level_instance(
        &mut self,
        _origin: Origin,
        instance: &ApparentInstance,
        owner: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<ApparentInstance>> {
        let mut stack = vec![instance.clone()];
        let mut visited = FxIndexSet::default();

        // walk the substituted heritage levels until the owner appears
        while let Some(level) = stack.pop() {
            if level.symbol == owner {
                return Ok(Some(level));
            }
            if !visited.insert((level.symbol, level.arguments.clone())) {
                continue;
            }
            let Some(definition) = self.definition(level.symbol)? else {
                continue;
            };
            let heritages = definition
                .bases()
                .iter()
                .map(|heritage| heritage.ty)
                .collect::<SmallVec<[_; 2]>>();
            let substitution = level.substitution(self.check)?;
            for heritage in heritages {
                let heritage = self.substitute_type(heritage, &substitution)?;
                let Some((heritage_module, heritage)) = self.nominal_application_maybe(heritage)?
                else {
                    continue;
                };
                let arguments = self.type_ids(heritage_module, heritage.arguments)?;
                stack.push(ApparentInstance {
                    symbol: heritage.symbol,
                    arguments: arguments.iter().copied().collect(),
                });
            }
        }

        Ok(None)
    }

    /// Return one owner's canonical instance over its own parameters.
    fn canonical_instance(
        &mut self,
        instance: &ApparentInstance,
    ) -> CompilerResult<Option<ApparentInstance>> {
        let Some(template) = self.symbol_template(instance.symbol)? else {
            return Ok(None);
        };
        let parameters = self.generic_template_parameters(template)?;
        let mut arguments = SmallVec::with_capacity(parameters.len());
        for parameter in parameters {
            let Some(binding) = self.generic_parameter(parameter) else {
                return Ok(None);
            };
            arguments.push(binding.ty);
        }

        Ok(Some(ApparentInstance {
            symbol: instance.symbol,
            arguments,
        }))
    }

    /// Return whether one instance applies its owner's own parameters.
    fn is_canonical_instance(&mut self, instance: &ApparentInstance) -> CompilerResult<bool> {
        let Some(template) = self.symbol_template(instance.symbol)? else {
            return Ok(instance.arguments.is_empty());
        };
        let parameters = self.generic_template_parameters(template)?;
        if parameters.len() != instance.arguments.len() {
            return Ok(false);
        }

        // every argument names the declared parameter in its own slot
        for (argument, parameter) in instance.arguments.iter().zip(parameters) {
            let Some(binding) = self.generic_parameter(parameter) else {
                return Ok(false);
            };
            if binding.ty != *argument {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Derive one key's member lookup from the owner's flattened surface.
    fn stored_member_lookup(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        instance: &ApparentInstance,
        space: dir::MemberSpace,
        key: dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        // flatten the owner's canonical surface on first use
        if self
            .stored_member_bindings(instance.symbol, space)
            .is_none()
        {
            let canonical = match self.is_canonical_instance(instance)? {
                true => None,
                false => self.canonical_instance(instance)?,
            };
            let canonical = canonical.as_ref().unwrap_or(instance);
            let _ = self.inherent_member_table(origin, receiver, canonical, space)?;
        }

        // derive the asked key only; absent owners searched live already
        let Some(bindings) = self.stored_member_bindings(instance.symbol, space) else {
            let table = self.inherent_member_table(origin, receiver, instance, space)?;

            return Ok(table.get(&key).cloned().unwrap_or(MemberLookup::Missing));
        };
        let Some(binding) = bindings.into_iter().find(|binding| binding.key == key) else {
            return Ok(MemberLookup::Missing);
        };
        let candidates =
            self.binding_member_candidates(origin, receiver, instance, &binding, space)?;

        Ok(MemberLookup::Found(candidates))
    }

    /// Return the stored member bindings of one owner's canonical type.
    ///
    /// Each checking module flattens each owner it uses once into its own
    /// member segment, keyed by the owner's declared canonical type.
    fn stored_member_bindings(
        &self,
        symbol: dir::GlobalSymbolId,
        space: dir::MemberSpace,
    ) -> Option<Vec<dir::MemberBinding>> {
        let canonical = self.canonical_owner_type(symbol)?;
        let subject = dir::MemberSubject::new(canonical, canonical, space);

        // read foreign owners from their module's stored bindings
        if !self.check.is_own_module(symbol.module_id) {
            let external = self.check.external_modules.get(&symbol.module_id)?;
            let bindings = external.members.subject_bindings(&subject)?;

            return Some(bindings.to_vec());
        }
        let bindings = self.check.module.members.subject_bindings(&subject)?;

        Some(bindings.to_vec())
    }

    /// Return one owner's declared canonical self type.
    ///
    /// The declared id keys the stored member bindings, so the settled
    /// declared stage answers before this pass's re-canonicalized tail.
    fn canonical_owner_type(&self, symbol: dir::GlobalSymbolId) -> Option<dir::GlobalTypeId> {
        if self.check.is_own_module(symbol.module_id) {
            let module = &self.check.module;

            module
                .types
                .get_symbol_type_id(symbol)
                .or_else(|| module.types_tail.get_symbol_type_id(symbol))
        } else {
            let external = self.check.external_modules.get(&symbol.module_id)?;

            external.types.get_symbol_type_id(symbol)
        }
    }

    /// Derive one stored member binding's candidates for a lookup instance.
    ///
    /// Stored access and callable types are canonical: the owner resolved
    /// them for its own self type, so instances substitute their applied
    /// arguments and receiver into the stored types.
    fn binding_member_candidates(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        instance: &ApparentInstance,
        binding: &dir::MemberBinding,
        space: dir::MemberSpace,
    ) -> CompilerResult<Vec<MemberCandidate>> {
        // substitute the canonical types for this instance and receiver
        let receiver_value = self.strip_form(origin, receiver)?;
        let substitution = instance
            .substitution(self.check)?
            .with_receiver(receiver_value);
        let generic_arguments =
            self.symbol_generic_argument_bindings(instance.symbol, &instance.arguments)?;

        let mut candidates = Vec::with_capacity(binding.declarations.len());
        for declaration in &binding.declarations {
            // carry the declaring heritage level's arguments, not the top instance's
            let generic_arguments = if declaration.owner == instance.symbol {
                generic_arguments.clone()
            } else if let Some(level) =
                self.heritage_level_instance(origin, instance, declaration.owner)?
            {
                self.symbol_generic_argument_bindings(level.symbol, &level.arguments)?
            } else {
                generic_arguments.clone()
            };
            // pick the stored access basis by the declaration's role
            let access = match declaration.role {
                dir::MemberRole::Setter => binding.access.write(),
                dir::MemberRole::Method | dir::MemberRole::VariantConstructor
                    if declaration.callable_type.is_some() =>
                {
                    declaration.callable_type
                }
                _ => binding.access.read(),
            };
            let Some(access) = access else {
                return Err(CompilerError::Internal {
                    message: format!(
                        "member binding {:?} stores no access for {:?}",
                        binding.key, declaration.role
                    ),
                });
            };
            // rigid receivers project associated members through themselves
            let access_type = if declaration.role == dir::MemberRole::Associated
                && self.is_rigid_projection_owner(receiver)?
            {
                let arguments = self.intern_type_ids(&[])?;

                self.intern_member(dir::MemberType {
                    owner: receiver,
                    key: binding.key,
                    arguments,
                    qualifier: None,
                })?
            } else {
                self.substitute_type(access, &substitution)?
            };
            let access_type =
                self.projected_member_type(origin, Some(receiver), declaration.role, access_type)?;
            let callable = match declaration.callable_type {
                Some(callable) => Some(self.substitute_type(callable, &substitution)?),
                None => None,
            };
            let value = self.check.symbol_static_id(declaration.symbol);
            let value_type = match self.static_value(declaration.symbol) {
                Some(written) => Some(self.substitute_type(written, &substitution)?),
                None => None,
            };

            candidates.push(MemberCandidate {
                symbol: declaration.symbol,
                owner: declaration.owner,
                origin: declaration.origin,
                space,
                role: declaration.role,
                kind: binding.kind,
                is_writable: binding.access.write().is_some(),
                access_type,
                callable,
                is_optional: binding.is_optional,
                generic_arguments,
                value,
                value_type,
                receiver: LookupReceiver::Direct(ReceiverSteps::new()),
            });
        }

        Ok(candidates)
    }

    /// Build the member candidates one instance declares for one key.
    fn instance_member_candidates(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        instance: &ApparentInstance,
        substitution: &TypeSubstitution,
        members: &[dir::DefinitionMember],
        space: dir::MemberSpace,
        key: dir::StaticKey,
    ) -> CompilerResult<Vec<MemberCandidate>> {
        let substitution = substitution.clone();
        let mut candidates = Vec::new();
        for member in members.iter().cloned() {
            let Some(declared) = self.declared_member(&member)? else {
                continue;
            };
            let symbol = declared.symbol;

            // project value-less associated types through the receiver while keeping
            // defaults conformance-only behind rigid owners
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
            let access_type =
                self.projected_member_type(origin, Some(receiver), member.role, access_type)?;

            // substitute static value types for projections
            let written = match self.static_value(symbol) {
                Some(written) => Some(self.substitute_type(written, &substitution)?),
                written => written,
            };

            let generic_arguments =
                self.symbol_generic_argument_bindings(instance.symbol, &instance.arguments)?;

            candidates.push(MemberCandidate {
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
        Ok(candidates)
    }

    /// Collect the declared member keys reachable from one lookup subject.
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
            subject.key_type,
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
        // preserve static declaration references before reducing other heads
        let subject = self.shallow_resolve(subject)?;
        let subject = match (space, self.ty(subject)?) {
            (dir::MemberSpace::Static, dir::Type::Reference(_)) => subject,
            _ => self.reduce_type_head(origin, subject)?,
        };

        // stop cyclic paths through bounds, unions, and heritage
        if !visited.insert(subject) {
            return Ok(());
        }

        match self.ty(subject)? {
            // solved member subjects cannot retain inference variables
            dir::Type::Variable(variable) => {
                return Err(CompilerError::Internal {
                    message: format!(
                        "member keys contain unresolved variable {variable:?} after solving"
                    ),
                });
            }
            // declarations expose the same keys as keyed lookup
            dir::Type::Reference(reference) => {
                self.collect_reference_keys(origin, module, reference, space, keys, visited)?;
            }
            dir::Type::Application(_) => {
                self.collect_instance_keys(origin, module, subject, space, keys)?;

                // include the tagged discriminator and newtype payload keys
                if space == dir::MemberSpace::Instance
                    && let Some(instance) = self.apparent_instance(subject)?
                    && let Some(dir::Definition::Newtype(definition)) =
                        self.definition(instance.symbol)?
                {
                    if let Some(discriminator) = definition.discriminator {
                        keys.insert(discriminator);
                    }
                    if let Some(payload) = self.newtype_payload(origin, subject)? {
                        self.collect_subject_keys(
                            origin,
                            module,
                            payload.backing,
                            space,
                            keys,
                            visited,
                        )?;
                    }
                }
            }
            // refinements expose their refined key and base members
            dir::Type::Refined(refined) => {
                let refined = self.type_refined(subject.module_id, refined)?;
                keys.insert(refined.key);
                self.collect_subject_keys(origin, module, refined.base, space, keys, visited)?;
            }
            // precise variants expose their selected backing fields
            dir::Type::Variant(variant) => {
                if let Some(backing) = self.tagged_variant_backing(module, &variant)? {
                    self.collect_subject_keys(origin, module, backing, space, keys, visited)?;
                } else {
                    let dir::Type::Application(owner) = self.ty(variant.owner)? else {
                        return Err(CompilerError::Internal {
                            message: format!(
                                "variant {:?} has non-application owner {:?}",
                                variant.variant, variant.owner
                            ),
                        });
                    };
                    self.collect_symbol_keys(module, owner.symbol, space, keys, visited)?;
                }
            }
            // structural subjects expose their property keys
            dir::Type::Shape(shape) | dir::Type::Object(shape) => {
                for property in self.shape_properties(subject.module_id, shape.properties)? {
                    keys.insert(property.key);
                }
            }
            // primitives, literals, and built-in collections expose their
            //  apparent declaration and extension keys through the tables
            dir::Type::Primitive(_)
            | dir::Type::Literal(_)
            | dir::Type::Array(_)
            | dir::Type::Slice(_)
            | dir::Type::FixedArray(_) => {
                self.collect_instance_keys(origin, module, subject, space, keys)?;
            }
            // tuples expose their labeled elements
            dir::Type::Tuple(tuple) => {
                for element in self.tuple_elements(subject.module_id, tuple.elements)? {
                    if let Some(label) = element.label {
                        keys.insert(dir::StaticKey::Name(label));
                    }
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
            dir::Type::Union(dir::UnionType { elements })
            | dir::Type::Intersection(dir::IntersectionType { elements }) => {
                let elements = self.type_ids(subject.module_id, elements)?.to_vec();
                for element in elements {
                    self.collect_subject_keys(origin, module, element, space, keys, visited)?;
                }
            }
            // remaining types expose no keyed members
            dir::Type::Error
            | dir::Type::Never
            | dir::Type::Any
            | dir::Type::Unknown
            | dir::Type::Void
            | dir::Type::Null
            | dir::Type::Undefined
            | dir::Type::Key(_)
            | dir::Type::Memory(_)
            | dir::Type::Static(_)
            | dir::Type::Intrinsic
            | dir::Type::Erased(_)
            | dir::Type::This
            | dir::Type::Member(_)
            | dir::Type::Operation(_)
            | dir::Type::Range(_)
            | dir::Type::FunctionSignature(_)
            | dir::Type::Function(_)
            | dir::Type::FunctionPointer(_) => {}
        }

        Ok(())
    }

    /// Collect the keys selected through one declaration reference.
    fn collect_reference_keys(
        &mut self,
        origin: Origin,
        module: ModuleId,
        reference: dir::TypeReference,
        space: dir::MemberSpace,
        keys: &mut FxIndexSet<dir::StaticKey>,
        visited: &mut FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<()> {
        let symbol = reference.symbol;
        let alias = match self.definition(symbol)? {
            Some(dir::Definition::TypeAlias(alias)) => Some(alias.value),
            _ => None,
        };

        // static aliases expose the keys of their reduced body
        if space == dir::MemberSpace::Static
            && let Some(alias) = alias
        {
            let body = self.reduce_type_head(origin, alias)?;

            return self.collect_subject_keys(origin, module, body, space, keys, visited);
        }

        self.collect_symbol_keys(module, symbol, space, keys, visited)
    }

    /// Collect one nominal subject's keys from its member tables.
    fn collect_instance_keys(
        &mut self,
        origin: Origin,
        module: ModuleId,
        subject: dir::GlobalTypeId,
        space: dir::MemberSpace,
        keys: &mut FxIndexSet<dir::StaticKey>,
    ) -> CompilerResult<()> {
        let Some(instance) = self.apparent_instance(subject)? else {
            return Ok(());
        };

        // the inherent table enumerates declaration and heritage keys
        let inherent = self.inherent_member_table(origin, subject, &instance, space)?;
        keys.extend(inherent.keys().copied());

        // the extension table enumerates the visible extension keys
        let extensions = self.subject_extension_members(
            origin,
            module,
            subject,
            subject,
            instance.symbol,
            space,
        )?;
        keys.extend(extensions.keys().copied());

        Ok(())
    }

    /// Collect the member keys one declaration exposes, with its heritage and extensions.
    fn collect_symbol_keys(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        space: dir::MemberSpace,
        keys: &mut FxIndexSet<dir::StaticKey>,
        visited: &mut FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<()> {
        // collect the declaration's own member keys
        self.collect_definition_keys(symbol, space, keys)?;

        // collect the tagged discriminator
        if space == dir::MemberSpace::Instance
            && let Some(dir::Definition::Newtype(definition)) = self.definition(symbol)?
            && let Some(discriminator) = definition.discriminator
        {
            keys.insert(discriminator);
        }

        // collect inherited keys through base declarations and interfaces
        let heritages = match self.definition(symbol)? {
            Some(definition) => {
                let mut heritages = definition
                    .bases()
                    .into_iter()
                    .map(|heritage| heritage.ty)
                    .collect::<SmallVec<[_; 2]>>();
                heritages.extend(
                    definition
                        .implementations()
                        .iter()
                        .map(|conformance| conformance.interface),
                );

                heritages
            }
            None => SmallVec::new(),
        };
        for heritage in heritages {
            self.collect_subject_keys(
                Origin::Symbol(symbol),
                module,
                heritage,
                space,
                keys,
                visited,
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
            return Err(CompilerError::Internal {
                message: format!("member key declaration is missing: {symbol:?}"),
            });
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
