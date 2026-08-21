use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexMap;
use smallvec::SmallVec;

use crate::sema::{
    BodyState, Cause, CauseKind, CheckAttempt, CheckFailure, CheckOutcome, Expectation, FlowSite,
    InferMode, MemberRole, Origin, PlaceUse, Relation, ValueCheck, ValueUse, Verdict, WalkState,
};
use crate::{CompilerError, CompilerResult};

impl BodyState<'_, '_> {
    /// Infer one object literal from its properties.
    pub(in crate::sema) fn infer_object_expression(
        &mut self,
        site: FlowSite,
        properties: &[dir::LocalNodeId<dir::Property>],
        mode: InferMode,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let mut fields = IndexMap::<dir::StaticKey, dir::TypeProperty>::new();
        let mut sources = IndexMap::<dir::StaticKey, dir::GlobalNodeIdAny>::new();
        let mut authored = IndexMap::<dir::StaticKey, dir::PropertyAccess>::new();
        let field_mode = mode;

        // collect literal fields, methods, and spreads into one shape
        for property in properties {
            let property = *property;
            match self.module(module).view().get(property).clone() {
                dir::Property::Field { name, value, .. } => {
                    let key = name.into();

                    // infer the written value under the field mode
                    let value_site = self.visit_site(value.into_global_any(module))?;
                    let ty = self.infer_node(value_site, PlaceUse::Read, field_mode)?;
                    let ty = self.flow_type_at(value_site, ty)?;
                    let ty = match field_mode {
                        // a mutable field widens the fresh literal it stores
                        InferMode::Regular => {
                            let value = self.expression_value(value_site, ty)?;
                            self.widen_fresh(value)?
                        }
                        InferMode::Const => ty,
                    };

                    // record the field slot and the value that wrote it
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
                    sources.insert(key, value.into_global_any(module));
                }
                dir::Property::Method {
                    name, signature, ..
                } => {
                    let Some(name) = name else {
                        continue;
                    };
                    let key = name.into();

                    // read the slot the method's signature exposes
                    let role = MemberRole::from(signature.role);
                    let (authored_access, field) =
                        self.method_property_slot(module, property, key, role, mode.is_readonly())?;
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

                    // reject a source that carries no object fields
                    let Some(spread_fields) =
                        self.spread_fields(Origin::Node(source, site.scope), module, spread)?
                    else {
                        self.report_spread_not_object(Origin::Node(source, site.scope), spread)?;
                        self.commit_decision(node.into_any(), dir::Decision::Rejected)?;
                        let error = self.commit_error_node(node.into_any())?;

                        return Ok(error);
                    };

                    // record the fields contributed by this spread
                    let key_type = self.intern_object(&spread_fields)?;
                    let subject =
                        dir::MemberSubject::new(spread, spread, dir::MemberSpace::Instance)
                            .with_scope(site.scope)
                            .with_key_type(key_type);
                    self.module_mut(module).members_tail.record_subject(
                        dir::MemberSite::Node(property.into_global_any(module)),
                        subject,
                    );

                    // overwrite the slots the spread supplies
                    for field in spread_fields {
                        let field = dir::TypeProperty {
                            access: match mode.is_readonly() {
                                true => field.access.readonly(),
                                false => field.access,
                            },
                            ..field
                        };
                        authored.shift_remove(&field.key);
                        fields.insert(field.key, field);
                        sources.swap_remove(&field.key);
                    }
                }
                dir::Property::Error => {}
            }
        }

        // intern the collected literal shape
        let fields: Vec<dir::TypeProperty> = fields.into_values().collect();
        let fields = self.intern_properties(&fields)?;
        let shape = self.intern_type(dir::Type::Object(dir::ShapeType {
            properties: fields,
            call_signatures: dir::TypeListId::EMPTY,
            construct_signatures: dir::TypeListId::EMPTY,
            index_signatures: dir::TypeListId::EMPTY,
        }))?;

        let ty = shape;

        // convert authored fields into the selected object slots
        let dir::Type::Object(shape) = self.ty(ty)? else {
            return Ok(ty);
        };
        let target_fields: SmallVec<[_; 4]> = self
            .shape_properties(ty.module_id, shape.properties)?
            .into();
        for (key, source) in sources {
            let Some(target) = target_fields.iter().find(|field| field.key == key) else {
                continue;
            };
            let source_type = self.require_node_type(source)?;
            let target_type = target.access.store();
            if source_type == target_type {
                continue;
            }

            let cause = self.intern_cause(Cause::root(
                Origin::Node(source, site.scope),
                CauseKind::Field { key },
            ));
            let source_site = self.visit_site(source)?;
            let expectation = Expectation::assignable(target_type, cause, ValueUse::Store);
            self.check_value(source_site, source_type, expectation)?;
        }

        Ok(ty)
    }

    /// Check one object literal under an expected object type.
    pub(in crate::sema) fn check_object_expression(
        &mut self,
        site: FlowSite,
        properties: &[dir::LocalNodeId<dir::Property>],
        target_value: dir::GlobalTypeId,
        expectation: Expectation,
    ) -> CompilerResult<CheckAttempt> {
        let origin = site.origin();
        let node = site.node.into_typed::<dir::Expression>();
        let target = expectation.target;

        let Some((target_fields, index_signatures)) =
            self.apparent_object_members(origin, target_value)?
        else {
            return Ok(CheckAttempt::NotApplicable);
        };

        // record the fields accepted by this literal
        let key_type = self.intern_object(&target_fields)?;
        let subject =
            dir::MemberSubject::new(target_value, target_value, dir::MemberSpace::Instance)
                .with_scope(site.scope)
                .with_key_type(key_type);
        self.module_mut(node.module_id)
            .members_tail
            .record_subject(dir::MemberSite::Node(node.into_any()), subject);

        let mut authored = IndexMap::<dir::StaticKey, dir::PropertyAccess>::new();
        let mut source_fields = IndexMap::<dir::StaticKey, dir::TypeProperty>::new();
        let mut check = CheckOutcome::Holds;

        // check present fields against their matching expected fields
        for property in properties {
            let property = *property;
            match self.module(node.module_id).view().get(property).clone() {
                dir::Property::Field { name, value, .. } => {
                    let key = name.into();

                    // select a declared field before a matching index signature
                    let field =
                        self.selected_target_field(origin, &target_fields, &index_signatures, key)?;

                    // type excess values without assigning them to target storage
                    let Some(field) = field else {
                        let child = value.into_global_any(node.module_id);
                        let child_site = self.visit_site(child)?;
                        let ty = self.infer_node(child_site, PlaceUse::Read, expectation.mode)?;
                        let ty = self.flow_type_at(child_site, ty)?;
                        let value = self.expression_value(child_site, ty)?;
                        let storage = self.widen_fresh(value)?;
                        let (typed_access, authored_access) =
                            Self::written_field_accesses(storage, expectation.mode.is_readonly());
                        let field = dir::TypeProperty {
                            key,
                            access: typed_access,
                            is_optional: false,
                        };
                        self.upsert_object_property(
                            &mut authored,
                            &mut source_fields,
                            property.into_global_any(node.module_id),
                            authored_access,
                            field,
                        )?;
                        check =
                            check.and(CheckOutcome::Fails(CheckFailure::ExcessProperty { key }));

                        continue;
                    };

                    // check the written value against the selected field
                    let child = value.into_global_any(node.module_id);
                    let child_site = self.visit_site(child)?;
                    let field_cause = self.check.intern_cause(Cause::child(
                        Origin::Node(child, site.scope),
                        CauseKind::Field { key },
                        expectation.cause,
                    ));
                    let target_type = field.access.store();
                    let child_expectation = Expectation {
                        target: target_type,
                        cause: field_cause,
                        use_: ValueUse::Store,
                        ..expectation
                    };
                    let child_check = self.check_node(child_site, child_expectation)?;

                    // record the slot the checked value commits
                    let (field, authored_access) = self.expected_field_slot(
                        site.origin(),
                        &expectation,
                        &field,
                        child_check.stored,
                    )?;
                    self.upsert_object_property(
                        &mut authored,
                        &mut source_fields,
                        property.into_global_any(node.module_id),
                        authored_access,
                        field,
                    )?;
                    check = check.and(child_check.outcome);
                }
                dir::Property::Spread { value } => {
                    // infer the spread source and read its supplied fields
                    let source = value.into_global_any(node.module_id);
                    let source_site = self.visit_site(source)?;
                    let spread = self.infer_node(source_site, PlaceUse::Read, expectation.mode)?;
                    let spread = self.flow_type_at(source_site, spread)?;
                    let spread_origin = Origin::Node(source, site.scope);
                    let Some(spread_fields) =
                        self.spread_fields(spread_origin, node.module_id, spread)?
                    else {
                        self.report_spread_not_object(spread_origin, spread)?;
                        self.commit_decision(node.into_any(), dir::Decision::Rejected)?;
                        self.commit_error_node(node.into_any())?;

                        return Ok(CheckAttempt::NotApplicable);
                    };

                    // adopt each supplied field into its expected slot
                    for supplied in spread_fields {
                        let key = supplied.key;
                        let Some(read) = supplied.access.read() else {
                            continue;
                        };
                        let target_field = self.selected_target_field(
                            origin,
                            &target_fields,
                            &index_signatures,
                            key,
                        )?;

                        // carry a field outside the expected shape unchanged
                        let Some(target_field) = target_field else {
                            authored.shift_remove(&key);
                            source_fields.insert(key, supplied);

                            continue;
                        };

                        // drop undefined from the value filling an optional slot
                        let read = match target_field.is_optional {
                            true => {
                                self.check
                                    .without_union_members(spread_origin, read, |ty| {
                                        matches!(ty, dir::Type::Undefined)
                                    })?
                            }
                            false => read,
                        };

                        // require the supplied value to fill the expected slot
                        let target_type = target_field.access.store();
                        let field_cause = self.check.intern_cause(Cause::child(
                            spread_origin,
                            CauseKind::Field { key },
                            expectation.cause,
                        ));
                        let holds = self.constrain_type(
                            spread_origin,
                            field_cause,
                            expectation.relation,
                            read,
                            target_type,
                        )?;
                        let outcome = self.complete_constraint_check(
                            spread_origin,
                            expectation.relation,
                            read,
                            target_type,
                            holds,
                        )?;

                        // record the slot at the storage the expected field selects
                        let (mut field, _) = self.expected_field_slot(
                            spread_origin,
                            &expectation,
                            &target_field,
                            read,
                        )?;
                        field.is_optional |= target_field.is_optional && supplied.is_optional;

                        // overwrite the slot the spread supplies
                        authored.shift_remove(&key);
                        source_fields.insert(key, field);
                        check = check.and(outcome);
                    }
                }
                dir::Property::Method {
                    name, signature, ..
                } => {
                    let Some(name) = name else {
                        return Ok(CheckAttempt::NotApplicable);
                    };
                    let key = name.into();

                    // read the slot the method's signature exposes
                    let role = MemberRole::from(signature.role);
                    let (authored_access, field) = self.method_property_slot(
                        node.module_id,
                        property,
                        key,
                        role,
                        expectation.mode.is_readonly(),
                    )?;
                    self.upsert_object_property(
                        &mut authored,
                        &mut source_fields,
                        property.into_global_any(node.module_id),
                        authored_access,
                        field,
                    )?;
                }
                dir::Property::Error => {}
            }
        }

        // require every nonoptional declared field from the final property set
        if let Some(missing) = target_fields
            .iter()
            .find(|field| !field.is_optional && !source_fields.contains_key(&field.key))
        {
            check = check.and(CheckOutcome::Fails(CheckFailure::MissingRequiredProperty {
                key: missing.key,
            }));
        }

        // adopt the slot class and memory form only for concrete object and intersection storage
        let is_adopting = expectation.relation != Relation::Satisfies
            && match self.ty(target_value)? {
                dir::Type::Object(shape) => !shape.declares_signatures(),
                dir::Type::Intersection(_) => true,
                _ => false,
            };
        let source = match is_adopting {
            true => target,
            false => {
                let fields: Vec<_> = source_fields.into_values().collect();
                let fields = self.intern_properties(&fields)?;
                self.intern_type(dir::Type::Object(dir::ShapeType {
                    properties: fields,
                    call_signatures: dir::TypeListId::EMPTY,
                    construct_signatures: dir::TypeListId::EMPTY,
                    index_signatures: dir::TypeListId::EMPTY,
                }))?
            }
        };
        self.commit_node_type(node.into_any(), source)?;

        Ok(CheckAttempt::Checked(ValueCheck {
            source,
            stored: source,
            outcome: check,
            target,
        }))
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
        if self.symbol_type_maybe(symbol).is_none() {
            let (parsed, expanded) = self.patched_inputs(module);
            let tree = dir::View::with_patches(&parsed.tree, std::slice::from_ref(&expanded.patch));
            let mut walk = WalkState::new(module, tree, self.check);
            walk.walk_property(property, &tree.get(property).clone())?;
            walk.flush_flows()?;
        }
        let Some(ty) = self.symbol_type_maybe(symbol) else {
            return Err(CompilerError::Internal {
                message: format!("object method property {symbol:?} has no type"),
            });
        };

        // read the operations the method exposes
        let ty = self.check.shallow_resolve(ty)?;
        let authored_access = self
            .check
            .property_access(role, ty, false)?
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

    /// Build the committed slot one expected field stores for a supplied value.
    fn expected_field_slot(
        &mut self,
        origin: Origin,
        expectation: &Expectation,
        target_field: &dir::TypeProperty,
        source_type: dir::GlobalTypeId,
    ) -> CompilerResult<(dir::TypeProperty, dir::PropertyAccess)> {
        // select the storage the expected slot commits
        let storage = self.slot_storage(
            origin,
            expectation.relation,
            target_field.access.store(),
            source_type,
        )?;

        // commit a read slot for a read-only expectation or an unwritable field
        let is_readonly = expectation.mode.is_readonly()
            || expectation.relation != Relation::Satisfies && !target_field.access.is_writable();
        let typed_access = match is_readonly {
            true => dir::PropertyAccess::Read(storage),
            false => dir::PropertyAccess::ReadWrite {
                read: storage,
                write: storage,
            },
        };
        let authored_access = dir::PropertyAccess::ReadWrite {
            read: storage,
            write: storage,
        };
        let field = dir::TypeProperty {
            key: target_field.key,
            access: typed_access,
            is_optional: false,
        };

        Ok((field, authored_access))
    }

    /// Select the expected field one authored key writes, consulting index signatures.
    fn selected_target_field(
        &mut self,
        origin: Origin,
        target_fields: &[dir::TypeProperty],
        index_signatures: &[dir::TypeIndexSignature],
        key: dir::StaticKey,
    ) -> CompilerResult<Option<dir::TypeProperty>> {
        // prefer the declared field with this key
        if let Some(field) = target_fields.iter().find(|field| field.key == key) {
            return Ok(Some(*field));
        }

        // fall back to the first index signature accepting the key
        let key_type = self.static_key_type(key)?;
        for signature in index_signatures {
            let accepts = self.check.evaluate_relation(
                origin,
                Relation::Assignable,
                key_type,
                signature.key_type,
            )? != Verdict::Fails;
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

    /// Upsert one object property and report overlapping authored operations.
    fn upsert_object_property(
        &mut self,
        authored: &mut IndexMap<dir::StaticKey, dir::PropertyAccess>,
        fields: &mut IndexMap<dir::StaticKey, dir::TypeProperty>,
        source: dir::GlobalNodeIdAny,
        authored_access: dir::PropertyAccess,
        property: dir::TypeProperty,
    ) -> CompilerResult<()> {
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
                self.check.report_duplicate_definition_member(source, &key);
            }
            authored.insert(key, authored_access);
            fields.insert(key, property);
        }

        Ok(())
    }
}
