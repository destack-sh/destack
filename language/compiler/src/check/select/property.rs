use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexMap;
use smallvec::SmallVec;

use crate::check::{
    BodyState, Cause, CauseKind, CheckFailure, CheckOutcome, FlowSite, InferMode, Origin, PlaceUse,
    Relation, ValueCheck, ValueUse,
};
use crate::{CompilerError, CompilerResult};

/// One literal property entry collected for merging.
enum MergeEntry {
    /// One direct keyed field or method.
    Field {
        /// The property key.
        key: dir::StaticKey,
        /// The typed source node.
        source: dir::GlobalNodeIdAny,
    },
    /// One spread source.
    Spread {
        /// The spread property node.
        property: dir::GlobalNodeIdAny,
        /// The spread value node.
        source: dir::GlobalNodeIdAny,
    },
}

impl BodyState<'_, '_> {
    /// Select the merged shape of one literal with spread properties.
    pub(in crate::check) fn select_property_merge(
        &mut self,
        site: FlowSite,
        properties: &[dir::LocalNodeId<dir::Property>],
        target: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<ValueCheck> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let origin = site.origin();

        // collect the property entries in source order
        let mut entries = SmallVec::<[MergeEntry; 8]>::new();
        for property in properties {
            match self.module(module).view().get(*property).clone() {
                dir::Property::Field { key, value, .. } => {
                    let Some(key) = self.select_property_key(site, key)? else {
                        continue;
                    };

                    entries.push(MergeEntry::Field {
                        key,
                        source: value.into_global_any(module),
                    });
                }
                dir::Property::Method { key, .. } => {
                    let Some(key) = (match key {
                        Some(key) => self.select_property_key(site, key)?,
                        None => None,
                    }) else {
                        continue;
                    };

                    entries.push(MergeEntry::Field {
                        key,
                        source: property.into_global_any(module),
                    });
                }
                dir::Property::Spread { value } => entries.push(MergeEntry::Spread {
                    property: property.into_global_any(module),
                    source: value.into_global_any(module),
                }),
                dir::Property::Error => {}
            }
        }

        // collect declared fields through the selected construct target
        let target_fields = match target {
            Some(target) => self.struct_constructor_fields(origin, target)?,
            None => SmallVec::new(),
        };

        // record the fields accepted by this literal
        if let Some(target) = target {
            let key_type = self.intern_shape(&target_fields)?;
            let subject = dir::MemberSubject::new(target, target, dir::MemberSpace::Instance)
                .with_scope(site.scope)
                .with_key_type(key_type);
            self.module_mut(module)
                .members
                .record_subject(dir::MemberSite::Node(node.into_any()), subject);
        }

        // the construct's write obligation owns every field check
        let write_cause = target.map(|_| {
            self.check.intern_cause(Cause::root(
                origin,
                CauseKind::Write {
                    place: node.into_any(),
                },
            ))
        });

        // merge entry fields left to right, later keys overriding
        let mut check = CheckOutcome::Holds;
        let mut fields = IndexMap::<dir::StaticKey, dir::TypeProperty>::new();
        for entry in entries {
            match entry {
                // direct fields use declared struct field types when available
                MergeEntry::Field { key, source } => {
                    let expected = target_fields.iter().find(|field| field.key == key);
                    let ty = match expected {
                        Some(field) if source.local_id.ty == dir::NodeType::Expression => {
                            let source_site = self.visit_site(source)?;
                            let field_origin = Origin::Node(source, site.scope);
                            let field_cause = self.check.intern_cause(match write_cause {
                                Some(parent) => {
                                    Cause::child(field_origin, CauseKind::Field { key }, parent)
                                }
                                None => Cause::root(field_origin, CauseKind::Field { key }),
                            });
                            let field_check = self.check_node_expected(
                                source_site,
                                field.access.store(),
                                Relation::Assignable,
                                field_cause,
                                ValueUse::Store,
                                match field.access.is_writable() {
                                    true => InferMode::Widen,
                                    false => InferMode::Exact,
                                },
                            )?;
                            check = check.and(field_check.outcome);

                            field_check.source
                        }
                        _ => self.merge_entry_type(source)?,
                    };
                    fields.insert(
                        key,
                        dir::TypeProperty {
                            key,
                            access: dir::PropertyAccess::ReadWrite {
                                read: ty,
                                write: ty,
                            },
                            is_optional: false,
                        },
                    );
                }
                // spread sources contribute every visible field
                MergeEntry::Spread { property, source } => {
                    let source_site = self.visit_site(source)?;
                    let spread = self.infer_node_type(source_site, PlaceUse::Read)?;

                    let Some(spread_fields) = self.spread_fields(origin, module, spread)? else {
                        let anchored = self.origin_at(origin, source)?;
                        self.report_spread_not_object(anchored, spread)?;
                        self.commit_decision(node.into_any(), dir::Decision::Rejected)?;
                        let source = self.commit_error_node(node.into_any())?;
                        let target = target.unwrap_or(source);
                        return Ok(ValueCheck {
                            source,
                            outcome: CheckOutcome::Fails(CheckFailure::Relation),
                            target,
                        });
                    };

                    // record the fields contributed by this spread
                    let key_type = self.intern_shape(&spread_fields)?;
                    let subject =
                        dir::MemberSubject::new(spread, spread, dir::MemberSpace::Instance)
                            .with_scope(site.scope)
                            .with_key_type(key_type);
                    self.module_mut(module)
                        .members
                        .record_subject(dir::MemberSite::Node(property), subject);

                    for field in spread_fields {
                        fields.insert(field.key, field);
                    }
                }
            }
        }

        // merge the shape at the literal node
        let fields: Vec<dir::TypeProperty> = fields.into_values().collect();
        let field_list = fields.clone();
        let fields = self.intern_properties(&fields)?;
        let shape = self.intern_type(dir::Type::from(dir::ShapeType {
            properties: fields,
            call_signatures: dir::TypeListId::EMPTY,
            construct_signatures: dir::TypeListId::EMPTY,
            index_signatures: dir::TypeListId::EMPTY,
        }))?;

        match target {
            // struct literals must fill their declared fields
            Some(target) => {
                // bind open construction arguments from their written fields
                if self.type_flags(target)?.has_variable() {
                    self.constrain_struct_construction(origin, &field_list, target)?;
                }

                // validate the final key set against the selected constructor fields
                let failure = target_fields
                    .iter()
                    .find(|field| {
                        !field.is_optional
                            && !field_list.iter().any(|written| written.key == field.key)
                    })
                    .map(|field| CheckFailure::MissingRequiredProperty { key: field.key })
                    .or_else(|| {
                        field_list
                            .iter()
                            .find(|written| {
                                !target_fields.iter().any(|field| field.key == written.key)
                            })
                            .map(|field| CheckFailure::ExcessProperty { key: field.key })
                    });
                if check == CheckOutcome::Holds
                    && let Some(failure) = failure
                {
                    check = CheckOutcome::Fails(failure);
                }

                // publish the selected struct instance at the literal
                self.commit_node_type(node.into_any(), target)?;
                Ok(ValueCheck {
                    source: target,
                    outcome: check,
                    target,
                })
            }
            // object literals bind their managed merged shape
            None => {
                let managed = self.intern_type(dir::Type::Form(dir::FormType {
                    form: dir::Form::Managed,
                    value: shape,
                }))?;
                self.commit_node_type(node.into_any(), managed)?;
                Ok(ValueCheck {
                    source: managed,
                    outcome: check,
                    target: managed,
                })
            }
        }
    }

    /// Return the merged field type at one entry source node.
    fn merge_entry_type(
        &mut self,
        source: dir::GlobalNodeIdAny,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // method properties type through their declaration symbol
        if source.local_id.ty == dir::NodeType::Property {
            let symbol = self
                .module(source.module_id)
                .declaration_symbol(source.local_id)
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("object method property {source:?} has no symbol"),
                })?;

            return self.symbol_type(symbol);
        }

        let source_site = self.visit_site(source)?;
        self.infer_node_type(source_site, PlaceUse::Read)
    }

    /// Return the mergeable fields of one spread source.
    pub(in crate::check) fn spread_fields(
        &mut self,
        origin: Origin,
        module: ModuleId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<Vec<dir::TypeProperty>>> {
        // reduce the spread source before merging fields
        let root = self.reduce_type_head(origin, ty)?;
        if let Some(variable) = self.root_variable(root)? {
            return Err(CompilerError::Internal {
                message: format!("spread source {ty:?} has open variable {variable:?}"),
            });
        }

        // peel managed forms down to the value they hold
        let mut current = root;
        while let dir::Type::Form(form) = self.ty(current)? {
            if form.form != dir::Form::Managed {
                break;
            }
            current = self.shallow_resolve(form.value)?;
        }

        match self.ty(current)? {
            // structural shapes spread their fields directly
            dir::Type::Shape(shape) | dir::Type::Object(shape) => Ok(Some(
                self.shape_properties(current.module_id, shape.properties)?
                    .to_vec(),
            )),
            // instances spread their visible fields
            dir::Type::Application(instance) => {
                let keys = self.nominal_member_keys(instance.symbol)?;
                let mut fields = Vec::with_capacity(keys.len());
                let mut seen = SmallVec::<[dir::StaticKey; 8]>::new();
                for key in keys {
                    if seen.contains(&key) {
                        continue;
                    }
                    seen.push(key);

                    // read each member through the receiver instance
                    let subject =
                        dir::MemberSubject::new(current, current, dir::MemberSpace::Instance);
                    let lookup = self.lookup_member(origin, module, subject, key)?;
                    let ty = self.member_read_type(origin, &lookup)?;
                    if let Some(ty) = ty {
                        fields.push(dir::TypeProperty {
                            key,
                            access: dir::PropertyAccess::ReadWrite {
                                read: ty,
                                write: ty,
                            },
                            is_optional: false,
                        });
                    }
                }

                Ok(Some(fields))
            }
            _ => Ok(None),
        }
    }
}
