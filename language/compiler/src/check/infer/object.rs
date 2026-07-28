use destack_dir as dir;
use indexmap::IndexMap;
use smallvec::SmallVec;

use crate::check::{
    Answer, BodyState, Cause, CauseKind, CheckAttempt, CheckFailure, CheckOutcome, Decision,
    Expectation, FlowSite, InferMode, Origin, PlaceUse, Relation, ValueCheck, ValueUse, answer,
};
use crate::{CompilerError, CompilerResult};

impl BodyState<'_, '_> {
    /// Infer one object literal from its properties.
    pub(in crate::check) fn infer_object_expression(
        &mut self,
        site: FlowSite,
        properties: &[dir::LocalNodeId<dir::Property>],
        mode: InferMode,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let mut fields = IndexMap::<dir::StaticKey, dir::TypeProperty>::new();
        let mut sources = IndexMap::<dir::StaticKey, dir::GlobalNodeIdAny>::new();
        let field_mode = mode.descend(mode.is_readonly());

        // collect literal fields, methods, and spreads into one shape
        for property in properties {
            match self.module(module).view().get(*property).clone() {
                dir::Property::Field { key, value, .. } => {
                    let Some(key) = answer!(self.select_property_key(site, key)?) else {
                        continue;
                    };
                    let value_site = self.node_site(value.into_global_any(module))?;
                    let ty = answer!(self.infer_node(value_site, PlaceUse::Read, field_mode)?);
                    let ty = answer!(self.flow_type_at(value_site, ty)?);
                    let access = match mode.is_readonly() {
                        true => dir::PropertyAccess::Read(ty),
                        false => dir::PropertyAccess::ReadWrite {
                            read: ty,
                            write: ty,
                        },
                    };
                    fields.insert(
                        key,
                        dir::TypeProperty {
                            key,
                            access,
                            is_optional: false,
                        },
                    );
                    sources.insert(key, value.into_global_any(module));
                }
                dir::Property::Method { key, .. } => {
                    let Some(key) = answer!(match key {
                        Some(key) => self.select_property_key(site, key)?,
                        None => Answer::Ready(None),
                    }) else {
                        continue;
                    };
                    let symbol = self
                        .module(module)
                        .declaration_symbol(property.into_any())
                        .ok_or_else(|| CompilerError::Internal {
                            message: format!("object method property {property:?} has no symbol"),
                        })?;
                    let Some(ty) = self.symbol_type_maybe(symbol) else {
                        return Err(CompilerError::Internal {
                            message: format!("object method property {symbol:?} has no type"),
                        });
                    };
                    let access = match mode.is_readonly() {
                        true => dir::PropertyAccess::Read(ty),
                        false => dir::PropertyAccess::ReadWrite {
                            read: ty,
                            write: ty,
                        },
                    };
                    fields.insert(
                        key,
                        dir::TypeProperty {
                            key,
                            access,
                            is_optional: false,
                        },
                    );
                }
                dir::Property::Spread { value } => {
                    let source = value.into_global_any(module);
                    let source_site = self.node_site(source)?;
                    let spread = answer!(self.infer_node(source_site, PlaceUse::Read, mode)?);
                    let spread = answer!(self.flow_type_at(source_site, spread)?);
                    let Some(spread_fields) = answer!(self.spread_fields(
                        Origin::Node(source, site.scope),
                        module,
                        spread
                    )?) else {
                        self.report_spread_not_object(Origin::Node(source, site.scope), spread)?;
                        self.commit_decision(node.into_any(), Decision::Rejected)?;
                        let error = self.commit_error_node(node.into_any())?;

                        return Ok(Answer::Ready(error));
                    };
                    for field in spread_fields {
                        let field = dir::TypeProperty {
                            access: match mode.is_readonly() {
                                true => field.access.readonly(),
                                false => field.access,
                            },
                            ..field
                        };
                        fields.insert(field.key, field);
                        sources.swap_remove(&field.key);
                    }
                }
                dir::Property::Error => {}
            }
        }

        let fields: Vec<dir::TypeProperty> = fields.into_values().collect();
        let fields = self.intern_properties(module, &fields)?;
        let shape = self.intern_type(
            module,
            dir::Type::Shape(dir::ShapeType {
                properties: fields,
                call_signatures: dir::TypeListId::EMPTY,
                construct_signatures: dir::TypeListId::EMPTY,
                index_signatures: dir::TypeListId::EMPTY,
            }),
        )?;

        let ty = if mode.widens_aggregate() {
            self.widen_type(shape)?
        } else {
            shape
        };

        // convert authored fields into the selected object slots
        let dir::Type::Shape(shape) = self.ty(ty)? else {
            return Ok(Answer::Ready(ty));
        };
        let target_fields = self
            .shape_properties(ty.module_id, shape.properties)?
            .to_vec();
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
            let source_site = self.node_site(source)?;
            let expectation = Expectation::assignable(target_type, cause, ValueUse::Store);
            answer!(self.check_value(source_site, source_type, expectation)?);
        }

        Ok(Answer::Ready(ty))
    }

    /// Check one object literal under an expected object type.
    pub(in crate::check) fn check_object_expression(
        &mut self,
        site: FlowSite,
        properties: &[dir::LocalNodeId<dir::Property>],
        carrier: dir::GlobalTypeId,
        target_value: dir::GlobalTypeId,
        expectation: Expectation,
    ) -> CompilerResult<Answer<CheckAttempt>> {
        let origin = site.origin();
        let node = site.node.into_typed::<dir::Expression>();
        let target = expectation.target;

        let Some((target_fields, index_signatures)) =
            answer!(self.expected_object_members(origin, target_value)?)
        else {
            return Ok(Answer::Ready(CheckAttempt::NotApplicable));
        };
        let mut authored = IndexMap::<dir::StaticKey, ()>::new();
        let mut source_fields = IndexMap::<dir::StaticKey, dir::TypeProperty>::new();
        let mut check = CheckOutcome::Holds;

        // check present fields against their matching expected fields
        for property in properties {
            let property_id = *property;
            let property = self.module(node.module_id).view().get(*property).clone();
            match property {
                dir::Property::Field { key, value, .. } => {
                    let Some(key) = answer!(self.select_property_key(site, key)?) else {
                        return Ok(Answer::Ready(CheckAttempt::NotApplicable));
                    };
                    authored.insert(key, ());

                    // select a declared field before a matching index signature
                    let mut field = target_fields.iter().find(|field| field.key == key).copied();
                    if field.is_none() {
                        let key_type = self.static_key_type(node.module_id, key)?;
                        for signature in &index_signatures {
                            let accepts = answer!(self.check.decide_relation(
                                origin,
                                Relation::Assignable,
                                key_type,
                                signature.key_type,
                            )?);
                            if accepts {
                                let access = match signature.is_readonly {
                                    true => dir::PropertyAccess::Read(signature.value_type),
                                    false => dir::PropertyAccess::ReadWrite {
                                        read: signature.value_type,
                                        write: signature.value_type,
                                    },
                                };
                                field = Some(dir::TypeProperty {
                                    key,
                                    access,
                                    is_optional: signature.is_optional,
                                });
                                break;
                            }
                        }
                    }

                    // type excess values without assigning them to target storage
                    let Some(field) = field else {
                        let child = value.into_global_any(node.module_id);
                        let child_site = self.node_site(child)?;
                        let mode = expectation.mode.descend(false);
                        let ty = answer!(self.infer_node(child_site, PlaceUse::Read, mode,)?);
                        let ty = answer!(self.flow_type_at(child_site, ty)?);
                        let storage = self.inference_candidate_type(ty, mode)?;
                        let access = match expectation.mode.is_readonly() {
                            true => dir::PropertyAccess::Read(storage),
                            false => dir::PropertyAccess::ReadWrite {
                                read: storage,
                                write: storage,
                            },
                        };
                        source_fields.insert(
                            key,
                            dir::TypeProperty {
                                key,
                                access,
                                is_optional: false,
                            },
                        );
                        check =
                            check.and(CheckOutcome::Fails(CheckFailure::ExcessProperty { key }));

                        continue;
                    };
                    let child = value.into_global_any(node.module_id);
                    let child_site = self.node_site(child)?;
                    let field_cause = self.check.intern_cause(Cause::child(
                        Origin::Node(child, site.scope),
                        CauseKind::Field { key },
                        expectation.cause,
                    ));
                    let target_type = field.access.store();
                    let mode = expectation.mode.descend(!field.access.is_writable());
                    let mode = self.contextual_literal_mode(target_type, mode)?;
                    let child_expectation = Expectation {
                        target: target_type,
                        cause: field_cause,
                        mode,
                        ..expectation
                    };
                    let child_check = answer!(self.check_node(child_site, child_expectation)?);
                    let ty = child_check.source;
                    let storage = answer!(self.contextual_literal_type(
                        site.origin(),
                        ty,
                        target_type,
                        mode,
                    )?);
                    let access = match expectation.mode.is_readonly() {
                        true => dir::PropertyAccess::Read(storage),
                        false => dir::PropertyAccess::ReadWrite {
                            read: storage,
                            write: storage,
                        },
                    };
                    source_fields.insert(
                        key,
                        dir::TypeProperty {
                            key,
                            access,
                            is_optional: false,
                        },
                    );
                    if expectation.relation == Relation::Satisfies && ty != storage {
                        let expectation =
                            Expectation::assignable(storage, field_cause, ValueUse::Store);
                        answer!(self.check_value(child_site, ty, expectation)?);
                    }
                    check = check.and(child_check.outcome);
                }
                dir::Property::Spread { .. } => {
                    return Ok(Answer::Ready(CheckAttempt::NotApplicable));
                }
                dir::Property::Method { key, .. } => {
                    let Some(key) = answer!(match key {
                        Some(key) => self.select_property_key(site, key)?,
                        None => Answer::Ready(None),
                    }) else {
                        return Ok(Answer::Ready(CheckAttempt::NotApplicable));
                    };
                    authored.insert(key, ());
                    let symbol = self
                        .module(node.module_id)
                        .declaration_symbol(property_id.into_any())
                        .ok_or_else(|| CompilerError::Internal {
                            message: format!(
                                "object method property {property_id:?} has no symbol"
                            ),
                        })?;
                    let Some(ty) = self.symbol_type_maybe(symbol) else {
                        return Err(CompilerError::Internal {
                            message: format!("object method property {symbol:?} has no type"),
                        });
                    };
                    let access = match expectation.mode.is_readonly() {
                        true => dir::PropertyAccess::Read(ty),
                        false => dir::PropertyAccess::ReadWrite {
                            read: ty,
                            write: ty,
                        },
                    };
                    source_fields.insert(
                        key,
                        dir::TypeProperty {
                            key,
                            access,
                            is_optional: false,
                        },
                    );
                }
                dir::Property::Error => {}
            }
        }

        // require every nonoptional declared field from the final property set
        if let Some(missing) = target_fields
            .iter()
            .find(|field| !field.is_optional && !authored.contains_key(&field.key))
        {
            check = check.and(CheckOutcome::Fails(CheckFailure::MissingRequiredProperty {
                key: missing.key,
            }));
        }

        // preserve the authored shape for a check-only expression
        let carrier = if expectation.relation == Relation::Satisfies {
            let fields: Vec<_> = source_fields.into_values().collect();
            let fields = self.intern_properties(node.module_id, &fields)?;

            self.intern_type(
                node.module_id,
                dir::Type::Shape(dir::ShapeType {
                    properties: fields,
                    call_signatures: dir::TypeListId::EMPTY,
                    construct_signatures: dir::TypeListId::EMPTY,
                    index_signatures: dir::TypeListId::EMPTY,
                }),
            )?
        } else {
            carrier
        };
        self.commit_node_type(node.into_any(), carrier)?;

        Ok(Answer::Ready(CheckAttempt::Checked(ValueCheck {
            source: carrier,
            outcome: check,
            target,
        })))
    }

    /// Return members expected by an object literal target.
    fn expected_object_members(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<
        Answer<
            Option<(
                SmallVec<[dir::TypeProperty; 8]>,
                SmallVec<[dir::TypeIndexSignature; 2]>,
            )>,
        >,
    > {
        match self.ty(target)? {
            dir::Type::Shape(shape) => {
                let fields = SmallVec::from_slice(
                    self.shape_properties(target.module_id, shape.properties)?,
                );
                let indexes = SmallVec::from_slice(
                    self.shape_index_signatures(target.module_id, shape.index_signatures)?,
                );

                Ok(Answer::Ready(Some((fields, indexes))))
            }
            // read fields from a structural interface, a nominal one needs its wrapper
            dir::Type::Application(instance)
                if matches!(
                    self.definition(instance.symbol)?,
                    Some(dir::Definition::Interface(interface)) if !interface.is_nominal
                ) =>
            {
                let fields = answer!(self.check.interface_instance_fields(
                    origin,
                    target.module_id,
                    &instance,
                    target,
                )?);

                let members = fields.map(|fields| (SmallVec::from_vec(fields), SmallVec::new()));

                Ok(Answer::Ready(members))
            }
            _ => Ok(Answer::Ready(None)),
        }
    }
}
