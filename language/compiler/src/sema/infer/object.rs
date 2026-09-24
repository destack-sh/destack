use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexMap;
use smallvec::SmallVec;

use crate::sema::{
    Cause, CauseKind, CheckFailure, CheckState, Expectation, FailedCheck, FlowSite, InferMode,
    MemberRole, Origin, PlaceUse, PropertySource, Relation, StoreTarget, ValueUse, Verdict,
    WalkState,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Infer one object literal from its properties.
    pub(in crate::sema) fn infer_object_expression(
        &mut self,
        site: FlowSite,
        properties: &[dir::LocalNodeId<dir::Property>],
        mode: InferMode,
        context: Option<Expectation>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // read the literal's node and open the shape it collects into
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let origin = site.origin();
        let mut fields = IndexMap::<dir::StaticKey, dir::TypeProperty>::new();
        let mut authored = IndexMap::<dir::StaticKey, dir::PropertyAccess>::new();
        let mut values = IndexMap::<dir::StaticKey, dir::GlobalNodeIdAny>::new();
        let mut refusals = SmallVec::<[CheckFailure; 2]>::new();

        // read the members the storage context declares for the authored keys
        let members = match context {
            Some(expectation) => {
                let unknown = self.intern_type(dir::Type::Unknown)?;
                let mut authored_keys = SmallVec::<[dir::TypeProperty; 8]>::new();
                for property in properties {
                    let dir::Property::Field { name, value, .. } =
                        self.module(module).view().get(*property).clone()
                    else {
                        continue;
                    };

                    // discriminate the arms a context offers by the written literal
                    let written = match self.module(module).view().get(value) {
                        dir::Expression::Literal(literal) => self.literal_type(*literal)?,
                        _ => unknown,
                    };
                    authored_keys.push(dir::TypeProperty {
                        key: name.into(),
                        access: dir::PropertyAccess::ReadWrite {
                            read: written,
                            write: written,
                        },
                        is_optional: false,
                    });
                }
                self.contextual_object_members(origin, expectation.target, &authored_keys)?
            }
            None => None,
        };

        // commit the contextual members as the literal's member subject
        if let (Some((fields, _, _)), Some(expectation)) = (&members, context) {
            let key_source = self.intern_object(fields)?;
            let subject = self
                .member_subject(
                    origin,
                    expectation.target,
                    expectation.target,
                    dir::MemberSpace::Instance,
                )?
                .with_key_source(key_source);
            self.module_mut(module).members_tail.commit_subject(
                dir::MemberSite::Node(node.into_any()),
                subject,
                None,
            );
        }

        // collect literal fields, methods, and spreads into one shape
        for property in properties {
            let property = *property;
            match self.module(module).view().get(property).clone() {
                dir::Property::Field { name, value, .. } => {
                    let key = name.into();

                    // store a contextual field's value into the declared member
                    let value_site = self.visit_site(value.into_global_any(module))?;
                    let member = match (&members, context) {
                        (Some((fields, indexes, _)), Some(expectation))
                            if mode == InferMode::Regular =>
                        {
                            let member = self.contextual_member(origin, fields, indexes, key)?;
                            if member.is_none() {
                                refusals.push(CheckFailure::ExcessProperty { key });
                            }

                            member.map(|member| (member, expectation))
                        }
                        _ => None,
                    };
                    let ty = if let Some((member, expectation)) = member {
                        let cause = self.intern_cause(Cause::child(
                            Origin::Node(value_site.node, site.scope),
                            CauseKind::Field { key },
                            expectation.cause,
                        ));
                        let member_type = self.erase_inference_barriers(member.access.store())?;
                        let store = match member.is_optional {
                            true => StoreTarget::Optional,
                            false => StoreTarget::Exact,
                        };
                        let use_ = expectation.use_;
                        let expectation = Expectation {
                            target: member_type,
                            cause,
                            use_: ValueUse::Store,
                            store,
                            ..expectation
                        };
                        match use_ {
                            // keep a predicate's field at its inferred type
                            ValueUse::Satisfies => {
                                let ty = self.infer_node_in(
                                    value_site,
                                    PlaceUse::Read,
                                    mode,
                                    Some(expectation),
                                )?;
                                let ty = self.flow_type_at(value_site, ty)?;
                                let stored_mode = match self.type_keeps_literal(
                                    origin,
                                    ty,
                                    member_type,
                                    false,
                                )? {
                                    true => InferMode::Literal,
                                    false => InferMode::Regular,
                                };
                                let slot = self.store_into_slot(value_site, ty, stored_mode)?;
                                self.constrain_type(
                                    origin,
                                    cause,
                                    Relation::Subtype,
                                    slot,
                                    member_type,
                                )?;

                                slot
                            }
                            // store a field into the declared member
                            _ => {
                                self.check_node(value_site, expectation)?;

                                member_type
                            }
                        }
                    }
                    // infer the written value into a mutable field
                    else {
                        let ty = self.infer_node(value_site, PlaceUse::Read, mode)?;
                        let ty = self.flow_type_at(value_site, ty)?;
                        match mode {
                            InferMode::Const => ty,
                            mode => self.store_into_slot(value_site, ty, mode)?,
                        }
                    };
                    values.insert(key, value_site.node);

                    // record the field and the value that wrote it
                    let (typed_access, authored_access) =
                        Self::written_field_accesses(ty, mode.is_readonly());
                    let field = dir::TypeProperty {
                        key,
                        access: typed_access,
                        is_optional: false,
                    };
                    self.upsert_object_property(
                        &mut authored,
                        &mut fields,
                        property.into_global_any(module),
                        authored_access,
                        field,
                    )?;
                }
                dir::Property::Method {
                    name, signature, ..
                } => {
                    let Some(name) = name else {
                        continue;
                    };
                    let key = name.into();

                    // read the member type of the method's signature
                    let role = MemberRole::from(signature.role);
                    let (authored_access, field) =
                        self.method_property_slot(module, property, key, role, mode.is_readonly())?;

                    // check the accessor's operations against the member its context declares
                    if let (Some((members, indexes, _)), Some(expectation)) = (&members, context)
                        && mode == InferMode::Regular
                        && let Some(member) =
                            self.contextual_member(origin, members, indexes, key)?
                    {
                        let cause = self.intern_cause(Cause::child(
                            Origin::Node(property.into_global_any(module), site.scope),
                            CauseKind::Field { key },
                            expectation.cause,
                        ));
                        if let Some(relations) = Self::shape_property_relations(
                            expectation.relation,
                            PropertySource::Constructed,
                            &field,
                            &member,
                        ) {
                            self.relate_shape_fields(origin, cause, &relations)?;
                        }
                    }
                    self.upsert_object_property(
                        &mut authored,
                        &mut fields,
                        property.into_global_any(module),
                        authored_access,
                        field,
                    )?;
                }
                dir::Property::Spread { value } => {
                    // infer the spread source
                    let source = value.into_global_any(module);
                    let source_site = self.visit_site(source)?;
                    let spread = self.infer_node(source_site, PlaceUse::Read, mode)?;
                    let spread = self.flow_type_at(source_site, spread)?;

                    let spread = self.resolve_structurally(site, spread)?;

                    // poison the literal when its spread source already reported an error
                    if self.has_error_operand(&[spread])? {
                        return self.poison_node(node.into_any());
                    }

                    // reject a source without object fields
                    let Some(spread_fields) =
                        self.spread_fields(Origin::Node(source, site.scope), module, spread)?
                    else {
                        self.report_spread_not_object(Origin::Node(source, site.scope), spread)?;
                        self.commit_decision(node.into_any(), dir::Decision::Rejected)?;
                        let error = self.commit_error_node(node.into_any())?;

                        return Ok(error);
                    };

                    self.commit_spread_subject(
                        site,
                        property.into_global_any(site.node.module_id),
                        spread,
                        &spread_fields,
                    )?;

                    // overwrite the slots the spread supplies, later keys overriding earlier ones
                    for field in spread_fields {
                        let field = dir::TypeProperty {
                            access: match mode.is_readonly() {
                                true => field.access.readonly(),
                                false => field.access,
                            },
                            ..field
                        };
                        authored.shift_remove(&field.key);

                        // store a spread member into the member its context declares
                        if let (Some((members, indexes, _)), Some(expectation)) =
                            (&members, context)
                            && mode == InferMode::Regular
                        {
                            match self.contextual_member(origin, members, indexes, field.key)? {
                                Some(member) => {
                                    let cause = self.intern_cause(Cause::child(
                                        origin,
                                        CauseKind::Field { key: field.key },
                                        expectation.cause,
                                    ));
                                    self.constrain_type(
                                        origin,
                                        cause,
                                        expectation.relation,
                                        field.access.store(),
                                        member.access.store(),
                                    )?;
                                }
                                None => {
                                    refusals.push(CheckFailure::ExcessProperty { key: field.key })
                                }
                            }
                        }
                        fields.insert(field.key, field);
                    }
                }
                dir::Property::Error => {}
            }
        }

        // require every field one exact context declares
        if let (Some((members, _, Some(_))), InferMode::Regular) = (&members, mode) {
            for member in members {
                if !member.is_optional && !fields.contains_key(&member.key) {
                    refusals.push(CheckFailure::MissingRequiredProperty { key: member.key });
                }
            }
        }

        // intern the collected literal shape, reporting its refusals against it
        let is_refused = !refusals.is_empty();
        let fields: Vec<dir::TypeProperty> = fields.into_values().collect();
        let fields = self.intern_properties(&fields)?;
        let shape = self.intern_type(dir::Type::Object(dir::ObjectType {
            properties: fields,
            call_signatures: dir::TypeListId::EMPTY,
            construct_signatures: dir::TypeListId::EMPTY,
            index_signatures: dir::TypeListId::EMPTY,
        }))?;
        if let Some(expectation) = context {
            for failure in refusals {
                self.push_shape_failure(shape, expectation, failure)?;
            }
        }

        // construct the literal as the concrete object slot an exact context declares
        if let (Some((_, _, Some(stored))), Some(expectation)) = (&members, context)
            && !is_refused
            && mode == InferMode::Regular
            && expectation.use_ != ValueUse::Satisfies
            && self.is_constructed_object_slot(origin, *stored)?
        {
            return Ok(*stored);
        }

        // type the literal as the handle its destination takes
        self.contextual_form(shape, context)
    }

    /// Return whether one slot stores a signature-free object or intersection a literal constructs.
    fn is_constructed_object_slot(
        &mut self,
        origin: Origin,
        slot: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let Some(value) = self.construction_value(origin, slot)? else {
            return Ok(false);
        };

        Ok(match self.ty(value)? {
            dir::Type::Object(shape) => !shape.declares_signatures(),
            dir::Type::Intersection(_) => true,
            _ => false,
        })
    }

    /// Record one shape failure of a literal against its whole expectation.
    fn push_shape_failure(
        &mut self,
        source: dir::GlobalTypeId,
        expectation: Expectation,
        failure: CheckFailure,
    ) -> CompilerResult<()> {
        self.push_failure(FailedCheck {
            cause: expectation.cause,
            relation: expectation.relation,
            use_: Some(expectation.use_),
            source,
            target: expectation.target,
            failure,
        })
    }

    /// Return the members one object context declares with the slot type the literal stores at.
    fn contextual_object_members(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
        written: &[dir::TypeProperty],
    ) -> CompilerResult<
        Option<(
            SmallVec<[dir::TypeProperty; 8]>,
            SmallVec<[dir::TypeIndexSignature; 2]>,
            Option<dir::GlobalTypeId>,
        )>,
    > {
        // read the members the target declares
        let Some(value) = self.construction_value(origin, target)? else {
            return Ok(None);
        };
        if let Some((fields, indexes)) = self.apparent_object_members(origin, target)? {
            let stored = self.replace_form_value(origin, target, value)?;

            return Ok(Some((fields, indexes, Some(stored))));
        }

        // read the fields a struct constructs from
        if let dir::Type::Application(instance) = self.ty(value)?
            && matches!(
                self.definition(instance.symbol)?.as_deref(),
                Some(dir::Definition::Struct(_))
            )
        {
            let fields = self.struct_constructor_fields(origin, value)?;

            return Ok(Some((fields, SmallVec::new(), Some(target))));
        }

        let Some(arms) = self.union_leaves(origin, value)? else {
            return Ok(None);
        };

        // read each arm's members, keeping the arms the written literals fall within
        let mut members = SmallVec::<[_; 4]>::new();
        let mut fitting = SmallVec::<[usize; 4]>::new();
        for arm in arms {
            let Some(constructed) = self.construction_value(origin, arm)? else {
                continue;
            };
            let Some((fields, indexes)) = self.apparent_object_members(origin, arm)? else {
                continue;
            };
            let mut fits = fields.iter().all(|field| {
                field.is_optional || written.iter().any(|property| property.key == field.key)
            });
            for property in written {
                let Some(field) = fields.iter().find(|field| field.key == property.key) else {
                    fits &= !indexes.is_empty();
                    continue;
                };
                let Some(value) = self.slot_value(property.access.store())? else {
                    continue;
                };
                if matches!(self.ty(value)?, dir::Type::Unknown) {
                    continue;
                }
                fits &=
                    self.decide_relation(origin, Relation::Subtype, value, field.access.store())?
                        != Verdict::Fails;
            }
            if fits {
                fitting.push(members.len());
            }
            members.push((constructed, fields, indexes));
        }
        // take the sole arm the written literals fall within
        if let [index] = fitting.as_slice() {
            let (constructed, fields, indexes) = members.swap_remove(*index);
            let stored = self.replace_form_value(origin, target, constructed)?;

            return Ok(Some((fields, indexes, Some(stored))));
        }

        // join the arms' members by key into a union per key
        let mut fields = SmallVec::<[dir::TypeProperty; 8]>::new();
        let mut indexes = SmallVec::new();
        for (_, arm_fields, arm_indexes) in &members {
            indexes.extend(arm_indexes.iter().copied());
            for field in arm_fields {
                let store = field.access.store();
                let slot = match fields.iter().position(|known| known.key == field.key) {
                    Some(index) => {
                        let joined =
                            self.normalized_union_type([fields[index].access.store(), store])?;
                        fields[index].access = dir::PropertyAccess::ReadWrite {
                            read: joined,
                            write: joined,
                        };

                        continue;
                    }
                    None => store,
                };
                fields.push(dir::TypeProperty {
                    key: field.key,
                    access: dir::PropertyAccess::ReadWrite {
                        read: slot,
                        write: slot,
                    },
                    is_optional: true,
                });
            }
        }

        Ok((!members.is_empty()).then_some((fields, indexes, None)))
    }

    /// Return the decided value one slot holds, none while it stays open.
    fn slot_value(&mut self, slot: dir::GlobalTypeId) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let slot = self.shallow_resolve(slot)?;

        Ok(self.root_variable(slot)?.is_none().then_some(slot))
    }

    /// Return the member one authored key fills, a declared field before an index signature.
    fn contextual_member(
        &mut self,
        origin: Origin,
        fields: &[dir::TypeProperty],
        indexes: &[dir::TypeIndexSignature],
        key: dir::StaticKey,
    ) -> CompilerResult<Option<dir::TypeProperty>> {
        // prefer the field declared under that key
        if let Some(field) = fields.iter().find(|field| field.key == key) {
            return Ok(Some(*field));
        }

        // otherwise take the first index signature accepting the key
        let key_type = self.static_key_type(key)?;
        for signature in indexes {
            let accepts =
                self.decide_relation(origin, Relation::Subtype, key_type, signature.key_type)?
                    != Verdict::Fails;
            if accepts {
                let access = match signature.is_readonly {
                    true => dir::PropertyAccess::Read(signature.value_type),
                    false => dir::PropertyAccess::ReadWrite {
                        read: signature.value_type,
                        write: signature.value_type,
                    },
                };

                return Ok(Some(dir::TypeProperty {
                    key,
                    access,
                    is_optional: signature.is_optional,
                }));
            }
        }

        Ok(None)
    }

    /// Open the slot one written value stores into, converting the value into it.
    pub(in crate::sema) fn store_into_slot(
        &mut self,
        site: FlowSite,
        ty: dir::GlobalTypeId,
        mode: InferMode,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // open a slot and convert the written value into it
        let origin = site.origin();
        let variable = self.open_variable(origin);
        let slot = self.variable_type(variable)?;
        let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
        self.check_value(
            site,
            ty,
            Expectation {
                target: slot,
                relation: Relation::Storable,
                cause,
                use_: ValueUse::Store,
                mode,
                store: StoreTarget::Exact,
            },
        )?;

        Ok(slot)
    }

    /// Return the authored access and slot one object method property exposes.
    fn method_property_slot(
        &mut self,
        module: ModuleId,
        property: dir::LocalNodeId<dir::Property>,
        key: dir::StaticKey,
        role: MemberRole,
        is_readonly: bool,
    ) -> CompilerResult<(dir::PropertyAccess, dir::TypeProperty)> {
        // read the method signature from its declared symbol
        let symbol = self
            .module(module)
            .declaration_symbol(property.into_any())
            .ok_or_else(|| CompilerError::Internal {
                message: format!("object method property {property:?} has no symbol"),
            })?;

        // walk methods inference discovers before their walk
        if self.symbol_type_maybe(symbol)?.is_none() {
            let (parsed, expanded) = self.patched_inputs(module);
            let tree = dir::View::new(&parsed.tree).patched(&expanded.patch);
            let mut walk = WalkState::new(module, tree, self);
            walk.walk_property(property, &tree.get(property).clone())?;
            walk.flush_flows()?;
        }
        let Some(ty) = self.symbol_type_maybe(symbol)? else {
            return Err(CompilerError::Internal {
                message: format!("object method property {symbol:?} has no type"),
            });
        };

        // read the operations the method exposes
        let ty = self.shallow_resolve(ty)?;
        let authored_access =
            self.property_access(role, ty, false)?
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("object property {property:?} has no property access"),
                })?;
        let typed_access = match is_readonly {
            true => authored_access.readonly(),
            false => authored_access,
        };

        Ok((
            authored_access,
            dir::TypeProperty {
                key,
                access: typed_access,
                is_optional: false,
            },
        ))
    }

    /// Return the typed and authored accesses one written field slot exposes for a stored type.
    fn written_field_accesses(
        ty: dir::GlobalTypeId,
        is_readonly: bool,
    ) -> (dir::PropertyAccess, dir::PropertyAccess) {
        // read the slot as readonly under a const literal
        let typed = match is_readonly {
            true => dir::PropertyAccess::Read(ty),
            false => dir::PropertyAccess::ReadWrite {
                read: ty,
                write: ty,
            },
        };

        (
            typed,
            dir::PropertyAccess::ReadWrite {
                read: ty,
                write: ty,
            },
        )
    }

    /// Commit the member subject one spread property contributes its fields through.
    pub(in crate::sema) fn commit_spread_subject(
        &mut self,
        site: FlowSite,
        property: dir::GlobalNodeIdAny,
        spread: dir::GlobalTypeId,
        spread_fields: &[dir::TypeProperty],
    ) -> CompilerResult<()> {
        // key the spread's member subject by the fields it supplies
        let key_type = self.intern_object(spread_fields)?;
        let subject = self
            .member_subject(site.origin(), spread, spread, dir::MemberSpace::Instance)?
            .with_key_source(key_type);
        self.module_mut(property.module_id)
            .members_tail
            .commit_subject(dir::MemberSite::Node(property), subject, None);

        Ok(())
    }

    /// Upsert one object property and report overlapping authored operations.
    fn upsert_object_property(
        &mut self,
        authored: &mut IndexMap<dir::StaticKey, dir::PropertyAccess>,
        fields: &mut IndexMap<dir::StaticKey, dir::TypeProperty>,
        source: dir::GlobalNodeIdAny,
        authored_access: dir::PropertyAccess,
        property: dir::TypeProperty,
    ) -> CompilerResult<()> {
        // read the access already authored under this key
        let key = property.key;
        let previous_authored = authored.get(&key).copied();
        let composed = match previous_authored {
            Some(existing) => existing.composed(authored_access),
            None => None,
        };

        // compose one getter and setter already authored for this key
        if let Some(composed) = composed {
            let Some(existing) = fields.get_mut(&key) else {
                return Err(CompilerError::Internal {
                    message: format!("authored property {key:?} has no typed field"),
                });
            };
            if !existing.compose(property) {
                return Err(CompilerError::Internal {
                    message: format!(
                        "property {key:?} has complementary authored operations but overlapping typed operations"
                    ),
                });
            }
            authored.insert(key, composed);
        }
        // otherwise replace a spread or diagnosed overlapping declaration
        else {
            if previous_authored.is_some() {
                self.report_duplicate_definition_member(source, &key);
            }
            authored.insert(key, authored_access);
            fields.insert(key, property);
        }

        Ok(())
    }
}
