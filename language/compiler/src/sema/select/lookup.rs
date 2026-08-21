use std::sync::Arc;

use destack_core::{FxIndexMap, FxIndexSet, NameMatch, find_best_match};
use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{
    Answer, ApparentInstance, Ask, BodyState, FieldLookup, LookupReceiver, MemberArmLookup,
    MemberCandidate, MemberLookup, MemberRole, Origin, ReceiverSteps, TypeArgumentInference,
    TypeSubstitution, Verdict,
};
use crate::{CompilerError, CompilerResult, diagnostic_suggestion_distance};

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
    /// Exclude extension members.
    Exclude,
    /// Include extension members.
    Include,
}

impl BodyState<'_, '_> {
    /// Return the member space implied by one receiver expression.
    pub(in crate::sema) fn member_receiver_space(
        &mut self,
        receiver: dir::GlobalNodeIdAny,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::MemberSpace> {
        if matches!(self.ty(ty)?, dir::Type::Reference(_)) {
            return Ok(dir::MemberSpace::Static);
        }

        // read the declaration the receiver expression names, if any
        let symbol = match self.name_decision(receiver) {
            Some(resolution) => match resolution.symbols() {
                [symbol] => Some(*symbol),
                _ => None,
            },
            None => match self.decision(receiver) {
                Some(dir::Decision::Function(dir::OperationResolution::One(value))) => {
                    value.target.symbol()
                }
                _ => None,
            },
        };
        let Some(symbol) = symbol else {
            return Ok(dir::MemberSpace::Instance);
        };

        // select static members for names that resolve to types
        let kind = self.symbol_kind(symbol)?;
        let is_type_name =
            kind.is_nominal() || matches!(kind, dir::SymbolKind::GenericTypeParameter);
        let space = if is_type_name {
            dir::MemberSpace::Static
        } else {
            dir::MemberSpace::Instance
        };

        Ok(space)
    }

    /// Collect the declared member keys reachable from one lookup subject.
    pub(in crate::sema) fn subject_member_keys(
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

    /// Look one member up on a settled subject, sharing the canonical answer.
    pub(in crate::sema) fn lookup_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        subject: dir::MemberSubject,
        key: dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        let asked = self.check.ask(
            origin,
            Ask::Member {
                module,
                space: subject.space,
                key,
            },
            &[subject.receiver, subject.target],
            false,
        )?;

        // replay the decided answer at this ask's live roots
        if let Some((question, canonical)) = &asked
            && let Some(Answer::Member(response)) = self.check.answers.get(question).cloned()
        {
            self.check.counters.member_replays += 1;

            return self
                .check
                .instantiate_response(origin, canonical, &response);
        }

        // count each underived ask once, refusals apart from derivations
        if asked.is_some() {
            self.check.counters.member_derivations += 1;
        } else {
            self.check.counters.member_refusals += 1;
        }

        // derive the lookup, guarding the recursion path it walks
        let mut active_queries = FxIndexSet::default();
        let lookup = self.lookup_subject_member(
            origin,
            module,
            subject.receiver,
            subject.target,
            subject.space,
            key,
            ExtensionFilter::Include,
            &mut active_queries,
        )?;

        // remember the decision folded canonical over its ask
        if let Some((question, canonical)) = &asked
            && lookup.is_canonical()
        {
            let checks_from = self.check.fulfill.checks.count();
            self.check.remember_answer(
                question,
                canonical,
                checks_from,
                lookup.clone(),
                Answer::Member,
            )?;
        }

        Ok(lookup)
    }

    /// Look up one non-extension member on a receiver type.
    pub(in crate::sema) fn lookup_inherent_member(
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
            ExtensionFilter::Exclude,
            &mut active_queries,
        )
    }

    /// Look up one member on a receiver type through every visible extension.
    pub(in crate::sema) fn lookup_visible_member(
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
            ExtensionFilter::Include,
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
        // read the declaration for static names through reference and application heads
        let root = self.shallow_resolve(subject)?;
        if space == dir::MemberSpace::Static {
            let named = match self.ty(root)? {
                dir::Type::Reference(reference) => {
                    Some((reference.symbol, SmallVec::<[dir::GlobalTypeId; 4]>::new()))
                }
                dir::Type::Application(instance) => Some((
                    instance.symbol,
                    SmallVec::from_slice(self.type_ids(root.module_id, instance.arguments)?),
                )),
                _ => None,
            };
            if let Some((symbol, arguments)) = named {
                return self.lookup_declaration_member(
                    origin,
                    module,
                    receiver,
                    dir::TypeReference { symbol },
                    &arguments,
                    space,
                    key,
                    extensions,
                    active,
                );
            }
        }

        // normalize assumed heads under the scope before resolving members
        let subject = self.normalize(origin, subject)?;

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

        // look the key up in the settled subject, then leave the path
        let lookup = self.lookup_normalized_member(
            origin, module, receiver, subject, space, key, extensions, active,
        );
        active.swap_remove(&query);

        lookup
    }

    /// Look one member up by the shape of one normalized subject type.
    fn lookup_normalized_member(
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
                        receiver: LookupReceiver::Direct(ReceiverSteps::new()),
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
                origin,
                module,
                receiver,
                reference,
                &[],
                space,
                key,
                extensions,
                active,
            ),

            // precise enum variants expose their owner's apparent members
            dir::Type::Variant(_) => self.lookup_apparent_instance_member(
                origin, module, receiver, subject, space, key, extensions,
            ),

            // applied declarations read their definition members
            dir::Type::Application(_)
            | dir::Type::Literal(_)
            | dir::Type::Primitive(_)
            | dir::Type::Slice(_)
            | dir::Type::FixedArray(_)
            | dir::Type::Function(_)
            | dir::Type::FunctionSignature(_) => {
                // widen literal subjects to their carrier before lookup
                let subject = match self.ty(subject)? {
                    dir::Type::Literal(literal) => self.intern_type(literal.widen())?,
                    _ => subject,
                };

                let mut lookup = self.lookup_apparent_instance_member(
                    origin, module, receiver, subject, space, key, extensions,
                )?;

                // dispatch erased value receivers through their dynamic payload
                if space == dir::MemberSpace::Instance
                    && self.is_erased_value(subject)?
                    && self.receiver_carries_subject(origin, receiver, subject)?
                {
                    lookup.select_dynamic(subject)?;
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
                    lookup.prepend_adjustment(adjustment);

                    return Ok(lookup);
                }

                Ok(lookup)
            }

            // expose the static space of a type held in a static term
            dir::Type::Static(value) => {
                let term = self.r#static(value).clone();
                match term {
                    dir::StaticTerm::Type { ty } => self.lookup_subject_member(
                        origin,
                        module,
                        receiver,
                        ty,
                        dir::MemberSpace::Static,
                        key,
                        extensions,
                        active,
                    ),
                    _ => Ok(MemberLookup::Missing),
                }
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

                // look the key up in the erased constraint, then dispatch dynamically
                let constraint_receiver = self.replace_form_value(origin, receiver, constraint)?;
                let mut lookup = self.lookup_bound_member(
                    origin,
                    module,
                    constraint_receiver,
                    &[constraint],
                    space,
                    key,
                    ExtensionFilter::Exclude,
                    active,
                )?;
                lookup.select_dynamic(constraint)?;

                // extensions remain direct calls over the erased receiver
                if matches!(lookup, MemberLookup::Missing)
                    && extensions == ExtensionFilter::Include
                    && let Some(instance) = self.apparent_instance(constraint)?
                {
                    return self.lookup_extension_member(
                        origin,
                        module,
                        receiver,
                        constraint,
                        dir::TypeRoot::Declaration(instance.symbol),
                        space,
                        key,
                    );
                }

                Ok(lookup)
            }

            // structural shapes expose every operation for the selected key
            dir::Type::Object(shape) => {
                let properties = self.shape_properties(subject.module_id, shape.properties)?;
                let properties = properties
                    .iter()
                    .filter(|property| property.key == key)
                    .copied()
                    .collect::<SmallVec<[_; 2]>>();
                if properties.is_empty() {
                    return Ok(MemberLookup::Missing);
                }

                // project each declared operation through the receiver
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

                // several declarations of one key intersect into one operation
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
                    receiver: LookupReceiver::Direct(ReceiverSteps::new()),
                    owner: subject,
                    access,
                    is_optional,
                }))
            }

            // tuples expose their labeled elements, then their extension members
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
                    return self.lookup_apparent_instance_member(
                        origin, module, receiver, subject, space, key, extensions,
                    );
                };

                // readonly elements expose the read operation alone
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
                    receiver: LookupReceiver::Direct(ReceiverSteps::new()),
                    owner: subject,
                    access,
                    is_optional: element.is_optional,
                }))
            }

            // unions join member lookups across their elements
            dir::Type::Union(union) => {
                let elements: SmallVec<[_; 8]> =
                    self.type_ids(subject.module_id, union.elements)?.into();

                self.lookup_union_member(
                    origin, module, receiver, subject, &elements, space, key, extensions, active,
                )
            }

            // intersections expose each element's members
            dir::Type::Intersection(intersection) => {
                let elements: SmallVec<[_; 8]> = self
                    .type_ids(subject.module_id, intersection.elements)?
                    .into();
                let mut lookups = Vec::with_capacity(elements.len());
                for element in elements.iter().copied() {
                    let element = self.shallow_resolve(element)?;
                    let mut lookup = match self.lookup_subject_member(
                        origin, module, receiver, element, space, key, extensions, active,
                    )? {
                        MemberLookup::Missing => continue,
                        MemberLookup::Intersection(nested) => {
                            lookups.extend(nested);
                            continue;
                        }
                        lookup => lookup,
                    };

                    // read a member of a kept union arm through the carrier beside it
                    let arm = self.replace_form_value(origin, receiver, element)?;
                    for carrier in elements.iter().copied() {
                        if carrier == element {
                            continue;
                        }
                        let carrier = self.replace_form_value(origin, receiver, carrier)?;
                        if let Some(steps) = self.project_carried_arm(origin, carrier, arm)? {
                            for adjustment in steps.into_iter().rev() {
                                lookup.prepend_adjustment(adjustment);
                            }
                            break;
                        }
                    }
                    lookups.push(lookup);
                }

                // answer directly from a sole element's lookup
                let lookup = if lookups.is_empty() {
                    MemberLookup::Missing
                } else if lookups.len() == 1 {
                    lookups.remove(0)
                } else {
                    MemberLookup::Intersection(lookups)
                };

                Ok(lookup)
            }

            // every remaining subject shape exposes no keyed member
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
            if lookup.is_found() {
                return Ok(lookup);
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
        // consult the extensions of a structural subject without an owner
        let Some(instance) = self.apparent_instance(lookup_type)? else {
            let value = self.strip_form(origin, lookup_type)?;
            if extensions == ExtensionFilter::Include
                && let Some(root) = self.structural_root(value)?
            {
                return self.lookup_extension_member(
                    origin,
                    module,
                    receiver,
                    lookup_type,
                    root,
                    space,
                    key,
                );
            }

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
        mut arguments: &[dir::GlobalTypeId],
        space: dir::MemberSpace,
        key: dir::StaticKey,
        extensions: ExtensionFilter,
        active: &mut FxIndexSet<MemberLookupKey>,
    ) -> CompilerResult<MemberLookup> {
        if space != dir::MemberSpace::Static {
            return Ok(MemberLookup::Missing);
        }

        let mut symbol = reference.symbol;

        // name a type alias's root declaration for statics, keeping the body for its own members
        let mut alias_body = None;
        if let Some(dir::Definition::TypeAlias(alias)) = self.definition(symbol)? {
            let body = alias.value;
            alias_body = Some(body);
            if let Some(named) = self.type_symbol(body)? {
                symbol = named;
                arguments = &[];
            }
        }

        if !self.is_own_module(symbol.module_id) {
            self.import_external_module(symbol.module_id)?;
        }

        // search inherent members first
        let is_alias = self.symbol_kind(symbol)?.is_type_alias();
        if !is_alias {
            let inherent =
                self.lookup_inherent_declaration_member(origin, receiver, symbol, arguments, key)?;
            if inherent.is_found() {
                return Ok(inherent);
            }
        }

        // search extensions next: declared members shadow interface defaults
        let lookup = match extensions {
            ExtensionFilter::Include => {
                self.lookup_static_extension_member(origin, module, symbol, arguments, key)?
            }
            ExtensionFilter::Exclude => MemberLookup::Missing,
        };
        if !lookup.is_found() && !is_alias {
            // search associated members through their declaring interface last
            let associated = self.lookup_associated_member(origin, receiver, space, key)?;
            if associated.is_found() {
                return Ok(associated);
            }

            // search the statics of the auto interfaces the declaration derives
            let instance = self.declaration_instance(reference.symbol)?;
            let subject = self.intern_type(dir::Type::Application(instance))?;
            let derived = self.lookup_derived_member(origin, subject, space, key)?;
            if derived.is_found() {
                return Ok(derived);
            }
        }

        // aliased bodies answer whatever the root declaration lacks
        if !lookup.is_found()
            && let Some(body) = alias_body
        {
            return self.lookup_subject_member(
                origin, module, receiver, body, space, key, extensions, active,
            );
        }

        Ok(lookup)
    }

    /// Return whether one receiver's own base value is the settled subject.
    fn receiver_carries_subject(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        subject: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let receiver_base = self.form_chain(origin, receiver)?.base();
        let receiver_base = self.normalize(origin, receiver_base)?;

        Ok(receiver_base == self.normalize(origin, subject)?)
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

        // runtime receiver unions settle each member under its enclosing forms
        let is_receiver_union = self.receiver_carries_subject(origin, receiver, subject)?;

        // require every element to expose the member
        for element in elements {
            let arm_receiver = if is_receiver_union {
                self.replace_form_value(origin, receiver, *element)?
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

        // project distinct singleton fields through the physical union discriminant
        if space == dir::MemberSpace::Instance
            && let Some(projection) =
                self.select_union_discriminant(origin, receiver, elements, key, &lookups)?
        {
            return Ok(MemberLookup::Field(FieldLookup::Projection {
                adjustments: ReceiverSteps::new(),
                projection,
            }));
        }

        Ok(MemberLookup::Union(lookups))
    }

    /// Select a shared required field as one physical union discriminant.
    fn select_union_discriminant(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        elements: &[dir::GlobalTypeId],
        key: dir::StaticKey,
        lookups: &[MemberArmLookup],
    ) -> CompilerResult<Option<dir::Projection>> {
        // require every selected element to belong to one physical union carrier
        let (carrier, _) = self.project_newtype_receiver(origin, receiver)?;
        let carrier = self.form_chain(origin, carrier)?.base();
        let Some(carrier_arms) = self.union_arms(origin, carrier)? else {
            return Ok(None);
        };
        if !elements
            .iter()
            .all(|element| carrier_arms.contains(element))
        {
            return Ok(None);
        }

        // require one distinct singleton-valued field from every selected arm
        if elements.len() != lookups.len() {
            return Err(CompilerError::Internal {
                message: "union member selection has mismatched arms".to_string(),
            });
        }
        let mut cases = Vec::with_capacity(lookups.len());
        let mut types = Vec::with_capacity(lookups.len());
        for (element, arm) in elements.iter().zip(lookups) {
            let Some(ty) = arm.lookup.direct_field_type() else {
                return Ok(None);
            };
            let base = self.strip_form(origin, ty)?;
            let Some(value) = self.ty(base)?.singleton_literal() else {
                return Ok(None);
            };

            // reject a repeated value, which selects no single arm
            if cases
                .iter()
                .any(|case: &dir::DiscriminantCase| case.value == value)
            {
                return Ok(None);
            }

            cases.push(dir::DiscriminantCase {
                arm: *element,
                value,
            });
            types.push(ty);
        }

        // preserve the semantic field type while retaining the physical mapping
        let ty = self.normalized_union_type(types)?;
        let projection = dir::Projection::Discriminant {
            union: carrier,
            key,
            cases,
            ty,
        };

        Ok(Some(projection))
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

        // search inherent members first, from stored bindings for closed subjects
        let is_closed = !self.type_flags(subject)?.has_variable();
        let inherent = if is_closed {
            self.stored_member_lookup(origin, receiver, &instance, space, key)?
        } else {
            self.lookup_inherent_symbol_member(origin, receiver, &instance, space, key)?
        };

        if inherent.is_found() {
            return Ok(inherent);
        }

        // search extensions next: declared members shadow interface defaults
        let mut lookup = MemberLookup::Missing;
        if extensions == ExtensionFilter::Include {
            lookup = self.lookup_extension_member(
                origin,
                module,
                receiver,
                subject,
                dir::TypeRoot::Declaration(instance.symbol),
                space,
                key,
            )?;

            // retry a missing lookup against the receiver's apparent owner
            if matches!(lookup, MemberLookup::Missing) {
                let apparent = self.intern_apparent_type(receiver)?;
                if apparent != receiver {
                    lookup = self.lookup_extension_member(
                        origin,
                        module,
                        receiver,
                        apparent,
                        dir::TypeRoot::Declaration(instance.symbol),
                        space,
                        key,
                    )?;
                }
            }
        }
        if lookup.is_found() {
            return Ok(lookup);
        }

        // search associated members through their declaring interface last
        let associated = self.lookup_associated_member(origin, receiver, space, key)?;
        if associated.is_found() {
            return Ok(associated);
        }

        // search the members of the auto interfaces the subject derives
        self.lookup_derived_member(origin, subject, space, key)
    }

    /// Look up one member declared by an auto interface the subject satisfies intrinsically.
    fn lookup_derived_member(
        &mut self,
        origin: Origin,
        subject: dir::GlobalTypeId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        // the interfaces the compiler implements with members, in declaration order
        let derived = dir::AutoInterface::all()
            .filter(|interface| interface.has_builtin_implementation() && !interface.is_marker());
        for interface in derived {
            let Some(symbol) = self
                .check
                .environment_bound
                .language
                .symbol(dir::LanguageItem::from(interface))
            else {
                continue;
            };

            // pass the subject as the interface's receiver argument
            let arguments = match interface.has_receiver_argument() {
                true => SmallVec::from_slice(&[subject]),
                false => SmallVec::new(),
            };
            let instance = ApparentInstance { symbol, arguments };

            // look the derived member up on the subject itself
            let lookup =
                self.lookup_inherent_symbol_member(origin, subject, &instance, space, key)?;
            if !lookup.is_found() {
                continue;
            }
            match self.satisfies_auto_interface(origin, subject, interface)? {
                Verdict::Holds => return Ok(lookup),
                Verdict::Ambiguous => return Ok(MemberLookup::Undecided),
                Verdict::Fails => {}
            }
        }

        Ok(MemberLookup::Missing)
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
        arguments: &[dir::GlobalTypeId],
        key: dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        // read the static members this declaration declares under the key
        let Some(definition) = self.definition(symbol)? else {
            return Ok(MemberLookup::Missing);
        };
        let members = definition
            .members_with_key(dir::MemberSpace::Static, key)
            .cloned()
            .collect::<SmallVec<[_; 2]>>();

        // read the heritage roots this declaration inherits statics from
        let heritages = definition
            .bases()
            .iter()
            .map(|heritage| heritage.ty)
            .collect::<SmallVec<[_; 2]>>();

        // bind the declaration's template parameters to the applied subject arguments
        let mut applied = TypeSubstitution::default();
        if !arguments.is_empty()
            && let Some(template) = self.symbol_template(symbol)?
        {
            let parameters = self.generic_template_parameters(template)?;
            for (parameter, argument) in parameters.iter().zip(arguments) {
                applied.bind(*parameter, *argument)?;
            }
        }

        // answer inherited statics through substituted heritage applications
        if members.is_empty() {
            for heritage in heritages {
                let heritage = self.substitute_type(heritage, &applied)?;
                let (heritage_module, instance) = self.nominal_application(heritage)?;
                let heritage_arguments: SmallVec<[_; 8]> =
                    self.type_ids(heritage_module, instance.arguments)?.into();
                let lookup = self.lookup_inherent_declaration_member(
                    origin,
                    receiver,
                    instance.symbol,
                    &heritage_arguments,
                    key,
                )?;
                if lookup.is_found() {
                    return Ok(lookup);
                }
            }

            return Ok(MemberLookup::Missing);
        }

        // collect visible static declaration members
        let mut candidates = Vec::new();
        for member in members {
            let key = member.key();
            let Some(member) = self.declared_member(&member)? else {
                continue;
            };

            // project valueless associated members symbolically over the receiver
            let mut ty = match member.ty {
                Some(ty) => ty,
                None if member.role == MemberRole::Associated
                    && let Some(key) = key =>
                {
                    let arguments = self.check.intern_type_ids(&[])?;
                    let projection = dir::MemberType {
                        owner: receiver,
                        key,
                        arguments,
                        qualifier: None,
                    };

                    self.check.intern_member(projection)?
                }
                None => continue,
            };

            let mut written = self.static_value(member.symbol);
            let mut generic_arguments = applied.bindings.to_vec();

            // substitute callable members over the applied arguments and receiver
            if member.role.is_callable() {
                let substitution = applied.clone().with_receiver(receiver);
                ty = self.substitute_type(ty, &substitution)?;
                written = written
                    .map(|written| self.substitute_type(written, &substitution))
                    .transpose()?;
            }
            // instantiate each direct value at its source use
            else if let Some(template) = self.symbol_template(symbol)? {
                let parameters = self.generic_template_parameters(template)?;
                let Some(substitution) = self.instantiate_parameters(
                    origin,
                    &parameters,
                    &[],
                    applied.clone().with_receiver(receiver),
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
                    self.check.push_relation(constraint)?;
                }

                ty = self.substitute_type(ty, &substitution)?;
                written = written
                    .map(|written| self.substitute_type(written, &substitution))
                    .transpose()?;
                generic_arguments = substitution.bindings.to_vec();
            }

            // project the member's operations through the receiver
            let callable = member.callable_type(ty);
            let access_type = member.access_type(self, ty)?;
            let access_type =
                self.projected_member_type(origin, Some(receiver), member.role, access_type)?;
            candidates.push(MemberCandidate {
                symbol: member.symbol,
                owner: symbol,
                origin: dir::MemberOrigin::Declaration,
                requirement: None,
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
                bounds: Vec::new(),
                target: None,
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
            if lookup.is_found() {
                return Ok(lookup);
            }
        }

        Ok(MemberLookup::Missing)
    }

    /// Return one closed subject's inherent members in declaration preorder, grouped by key.
    pub(in crate::sema) fn inherent_member_table(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        instance: &ApparentInstance,
        space: dir::MemberSpace,
    ) -> CompilerResult<Arc<FxIndexMap<dir::StaticKey, MemberLookup>>> {
        // derive the table from the owner's canonical member bindings
        if let Some(bindings) = self.member_bindings(instance.symbol, space)? {
            let mut table = FxIndexMap::default();
            for binding in bindings.iter() {
                let candidates =
                    self.binding_member_candidates(origin, receiver, instance, binding, space)?;
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
    pub(in crate::sema) fn canonical_member_bindings(
        &mut self,
        instance: &ApparentInstance,
        space: dir::MemberSpace,
    ) -> CompilerResult<Option<Vec<dir::MemberBinding>>> {
        let mut bindings: Vec<dir::MemberBinding> = Vec::new();
        let mut stack = vec![(instance.clone(), false)];
        let mut visited = FxIndexSet::default();

        // walk the declaration levels in preorder, first level per key wins
        while let Some((level, is_conformance)) = stack.pop() {
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

                // conformance levels serve their default members only
                if is_conformance {
                    let has_default = match &member {
                        dir::DefinitionMember::AssociatedType(associated) => {
                            associated.value.is_some()
                        }
                        other => other.is_default(),
                    };
                    if !has_default {
                        continue;
                    }
                }

                let Some(declared) = self.declared_member(member)? else {
                    continue;
                };

                let ty = match declared.ty {
                    Some(ty) => ty,
                    // valueless associated members project through receivers
                    None if declared.role == MemberRole::Associated => {
                        let arguments = self.intern_type_ids(&[])?;
                        let owner = level.intern(self.check)?;

                        self.intern_member(dir::MemberType {
                            owner,
                            key,
                            arguments,
                            qualifier: None,
                        })?
                    }
                    None => continue,
                };

                // apply this level's arguments and read the member's operations
                let ty = self.substitute_type(ty, &substitution)?;
                let ty = self.check.shallow_resolve(ty)?;
                let callable = declared.callable_type(ty);
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
                        // join an accessor pair with its counterpart's access
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
                .map(|heritage| (heritage.ty, is_conformance))
                .chain(
                    definition
                        .implementations()
                        .iter()
                        .map(|conformance| (conformance.interface, true)),
                )
                .collect::<SmallVec<[_; 2]>>();
            for (heritage, is_conformance) in heritages.into_iter().rev() {
                let heritage = self.substitute_type(heritage, &substitution)?;
                let Some((heritage_module, heritage)) = self.nominal_application_maybe(heritage)?
                else {
                    continue;
                };

                let arguments = self.type_ids(heritage_module, heritage.arguments)?;
                stack.push((
                    ApparentInstance {
                        symbol: heritage.symbol,
                        arguments: arguments.iter().copied().collect(),
                    },
                    is_conformance,
                ));
            }
        }

        Ok(Some(bindings))
    }

    /// Return the applied heritage level of one declaring owner.
    fn heritage_level_instance(
        &mut self,
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

    /// Derive one key's member lookup from the owner's canonical member bindings.
    fn stored_member_lookup(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        instance: &ApparentInstance,
        space: dir::MemberSpace,
        key: dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        // derive the asked key only; absent owners searched live already
        let Some(bindings) = self.member_bindings(instance.symbol, space)? else {
            let table = self.inherent_member_table(origin, receiver, instance, space)?;

            return Ok(table.get(&key).cloned().unwrap_or(MemberLookup::Missing));
        };
        let Some(binding) = bindings.iter().find(|binding| binding.key == key) else {
            return Ok(MemberLookup::Missing);
        };

        // substitute the binding through this instance and receiver
        let candidates =
            self.binding_member_candidates(origin, receiver, instance, binding, space)?;

        Ok(MemberLookup::Found(candidates))
    }

    /// Return one owner's canonical member bindings, memoized per run.
    pub(in crate::sema) fn member_bindings(
        &mut self,
        symbol: dir::GlobalSymbolId,
        space: dir::MemberSpace,
    ) -> CompilerResult<Option<Arc<Vec<dir::MemberBinding>>>> {
        // serve the memo
        if let Some(bindings) = self.check.bindings.get(&(symbol, space)) {
            self.check.counters.binding_replays += 1;

            return Ok(bindings.clone());
        }

        self.check.counters.binding_derivations += 1;

        // read owners their module's elaborate pass already flattened
        let stored = self.stored_member_bindings(symbol, space);

        // build the canonical bindings for unflattened owners
        let bindings = match stored {
            Some(bindings) => Some(Arc::new(bindings)),
            None => {
                let application = self.declaration_instance(symbol)?;
                let module = self.module_id;
                let arguments: SmallVec<[_; 8]> =
                    self.type_ids(module, application.arguments)?.into();
                let instance = ApparentInstance {
                    symbol,
                    arguments: arguments.into_iter().collect(),
                };

                self.canonical_member_bindings(&instance, space)?
                    .map(Arc::new)
            }
        };

        // memoize the bindings for every later ask
        self.check
            .bindings
            .insert((symbol, space), bindings.clone());

        Ok(bindings)
    }

    /// Return the member bindings one owner's elaborate pass flattened.
    fn stored_member_bindings(
        &self,
        symbol: dir::GlobalSymbolId,
        space: dir::MemberSpace,
    ) -> Option<Vec<dir::MemberBinding>> {
        // read own owners from the pass tail over the committed base
        if self.check.is_own_module(symbol.module_id) {
            let module = &self.check.module;
            if let Some(bindings) = module.members_tail.bindings(symbol, space) {
                return Some(bindings.to_vec());
            }

            return module
                .members
                .as_ref()
                .and_then(|base| base.bindings(symbol, space))
                .map(<[dir::MemberBinding]>::to_vec);
        }

        // read foreign owners from their module's elaborated bindings
        let external = self.check.external_modules.get(&symbol.module_id)?;
        let bindings = external.members.bindings(symbol, space)?;

        Some(bindings.to_vec())
    }

    /// Derive one stored member binding's candidates for a lookup instance.
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

        // build one candidate per declaration behind the key
        let mut candidates = Vec::with_capacity(binding.declarations.len());
        for declaration in &binding.declarations {
            // carry the arguments of the declaring heritage level
            let generic_arguments = if declaration.owner == instance.symbol {
                generic_arguments.clone()
            } else if let Some(level) = self.heritage_level_instance(instance, declaration.owner)? {
                self.symbol_generic_argument_bindings(level.symbol, &level.arguments)?
            } else {
                generic_arguments.clone()
            };

            // pick the stored access basis by the declaration's role
            let access = match declaration.role {
                dir::MemberRole::Setter => binding.access.write(),
                dir::MemberRole::Method if declaration.callable_type.is_some() => {
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

            // substitute the stored static value through the same instance
            let value = self.check.symbol_static_id(declaration.symbol);
            let value_type = match self.static_value(declaration.symbol) {
                Some(written) => Some(self.substitute_type(written, &substitution)?),
                None => None,
            };

            // record the interface whose requirement the member implements
            let requirement = match declaration.origin {
                dir::MemberOrigin::Declaration => None,
                _ => self.requirement_interface(declaration.owner, binding.key)?,
            };
            candidates.push(MemberCandidate {
                symbol: declaration.symbol,
                owner: declaration.owner,
                origin: declaration.origin,
                requirement,
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
                bounds: Vec::new(),
                target: None,
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
        // build one candidate per declaration behind the key
        let substitution = substitution.clone();
        let mut candidates = Vec::new();
        for member in members.iter().cloned() {
            let Some(declared) = self.declared_member(&member)? else {
                continue;
            };

            // project value-less associated types through the receiver
            let symbol = declared.symbol;
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

            // apply this instance's arguments and read the member's operations
            let member = declared;
            let ty = self.substitute_type(ty, &substitution)?;
            let callable = member.callable_type(ty);
            let access_type = member.access_type(self, ty)?;
            let access_type =
                self.projected_member_type(origin, Some(receiver), member.role, access_type)?;

            // substitute static value types for projections
            let written = match self.static_value(symbol) {
                Some(written) => Some(self.substitute_type(written, &substitution)?),
                None => None,
            };

            // carry this instance's solved arguments onto the candidate
            let generic_arguments =
                self.symbol_generic_argument_bindings(instance.symbol, &instance.arguments)?;
            candidates.push(MemberCandidate {
                symbol,
                owner: instance.symbol,
                origin: dir::MemberOrigin::Declaration,
                requirement: None,
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
                bounds: Vec::new(),
                target: None,
            });
        }

        Ok(candidates)
    }

    /// Return the reachable member key closest to one missing key.
    pub(in crate::sema) fn closest_member_key(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        key: &str,
    ) -> CompilerResult<Option<NameMatch<String>>> {
        // enumerate the receiver's keys like keyed lookup, in its selected space
        let module = origin.module();
        let subject = self.normalize_computation(origin, receiver)?;
        let space = match self.ty(subject)? {
            dir::Type::Reference(_) => dir::MemberSpace::Static,
            _ => dir::MemberSpace::Instance,
        };
        let mut keys = FxIndexSet::default();
        let mut visited = FxIndexSet::default();
        self.collect_subject_keys(origin, module, subject, space, &mut keys, &mut visited)?;
        let keys = keys
            .iter()
            .map(|candidate| self.format_static_key(candidate))
            .collect::<Vec<_>>();

        Ok(find_best_match(
            key,
            keys,
            diagnostic_suggestion_distance(key),
        ))
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
        // normalize the subject head like keyed lookup before collecting its keys
        let subject = self.normalize_computation(origin, subject)?;

        // stop cyclic paths through bounds, unions, and heritage
        if !visited.insert(subject) {
            return Ok(());
        }

        match self.ty(subject)? {
            // fail loudly on a variable that survived solving
            dir::Type::Variable(variable) => {
                return Err(CompilerError::Internal {
                    message: format!(
                        "member keys contain unresolved variable {variable:?} after solving"
                    ),
                });
            }
            // fail loudly on a canonical hole or rigid type outside the solver
            dir::Type::Hole(hole) => {
                return Err(CompilerError::Internal {
                    message: format!("member keys contain canonical hole ?{hole}"),
                });
            }
            dir::Type::Rigid(rigid) => {
                return Err(CompilerError::Internal {
                    message: format!("member keys contain canonical rigid type ^{rigid}"),
                });
            }
            // declarations expose the same keys as keyed lookup
            dir::Type::Reference(reference) => {
                self.collect_reference_keys(origin, module, reference, space, keys, visited)?;
            }
            // applied declarations expose their instance and extension keys
            dir::Type::Application(_) => {
                self.collect_instance_keys(origin, module, subject, space, keys)?;

                // include newtype payload keys
                if space == dir::MemberSpace::Instance
                    && let Some(instance) = self.apparent_instance(subject)?
                    && matches!(
                        self.definition(instance.symbol)?,
                        Some(dir::Definition::Newtype(_))
                    )
                    && let Some(payload) = self.newtype_payload(origin, subject)?
                {
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
            // refinements expose their refined key and base members
            dir::Type::Refined(refined) => {
                let refined = self.type_refined(subject.module_id, refined)?;
                keys.insert(refined.key);
                self.collect_subject_keys(origin, module, refined.base, space, keys, visited)?;
            }
            // precise variants expose their enum owner's members
            dir::Type::Variant(variant) => {
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
            // structural subjects expose their property keys
            dir::Type::Object(shape) => {
                for property in self.shape_properties(subject.module_id, shape.properties)? {
                    keys.insert(property.key);
                }
            }
            // read primitive, literal, collection, and function keys from the apparent tables
            dir::Type::Primitive(_)
            | dir::Type::Literal(_)
            | dir::Type::Slice(_)
            | dir::Type::FixedArray(_)
            | dir::Type::Function(_)
            | dir::Type::FunctionSignature(_) => {
                self.collect_instance_keys(origin, module, subject, space, keys)?;
            }

            // tuples expose their labeled elements and their extension members
            dir::Type::Tuple(tuple) => {
                for element in self.tuple_elements(subject.module_id, tuple.elements)? {
                    if let Some(label) = element.label {
                        keys.insert(dir::StaticKey::Name(label));
                    }
                }
                self.collect_instance_keys(origin, module, subject, space, keys)?;
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
                let elements: SmallVec<[_; 8]> = self.type_ids(subject.module_id, elements)?.into();
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

        // expose the keys of a static alias's body
        if space == dir::MemberSpace::Static
            && let Some(alias) = alias
        {
            return self.collect_subject_keys(origin, module, alias, space, keys, visited);
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
        // a structural subject exposes its extension keys alone
        let Some(instance) = self.apparent_instance(subject)? else {
            let value = self.strip_form(origin, subject)?;
            if let Some(root) = self.structural_root(value)? {
                self.collect_root_extension_keys(origin, module, subject, root, space, keys)?;
            }

            return Ok(());
        };

        // enumerate declaration and heritage keys from the inherent table
        let inherent = self.inherent_member_table(origin, subject, &instance, space)?;
        keys.extend(inherent.keys().copied());

        let root = dir::TypeRoot::Declaration(instance.symbol);

        self.collect_root_extension_keys(origin, module, subject, root, space, keys)
    }

    /// Collect the member keys the visible extensions over one root expose for a subject.
    fn collect_root_extension_keys(
        &mut self,
        origin: Origin,
        module: ModuleId,
        subject: dir::GlobalTypeId,
        root: dir::TypeRoot,
        space: dir::MemberSpace,
        keys: &mut FxIndexSet<dir::StaticKey>,
    ) -> CompilerResult<()> {
        // enumerate the visible extension keys from the decided candidates
        let extensions = self.reachable_extensions(origin, module, subject, subject, root)?;
        for extension in extensions {
            let matched = self
                .extension_subject_candidates(
                    origin, module, root, subject, subject, extension, space, None,
                )?
                // a re-entered extension adds no keys at its own fixed point
                .unwrap_or_default();
            keys.extend(matched.iter().map(|(key, _)| *key));
        }

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
        for extension in self.visible_extensions(module, dir::TypeRoot::Declaration(symbol))? {
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
