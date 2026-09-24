use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexMap;
use smallvec::SmallVec;

use crate::sema::{
    Cause, CauseKind, CheckFailure, CheckOutcome, CheckState, Expectation, FlowSite, Origin,
    PlaceUse, StoreTarget, ValueCheck, ValueUse, WalkState,
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

impl CheckState<'_> {
    /// Select the merged shape of one literal with spread properties.
    pub(in crate::sema) fn select_property_merge(
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
                dir::Property::Field { name, value, .. } => {
                    let key = name.into();

                    entries.push(MergeEntry::Field {
                        key,
                        source: value.into_global_any(module),
                    });
                }
                dir::Property::Method {
                    name, signature, ..
                } => {
                    // reject accessors that cannot initialize struct storage
                    if matches!(
                        signature.role,
                        Some(dir::FunctionRole::Getter | dir::FunctionRole::Setter)
                    ) {
                        self.report_invalid_struct_accessor(property.into_global_any(module));
                    }

                    // keep ordinary method shorthand and diagnosed accessors for checking
                    let Some(name) = name else {
                        continue;
                    };
                    let key = name.into();

                    // walk a method that inference needs before its declaration walk
                    if let Some(symbol) =
                        self.module(module).declaration_symbol(property.into_any())
                        && self.symbol_type_maybe(symbol)?.is_none()
                    {
                        let (parsed, expanded) = self.patched_inputs(module);
                        let tree = dir::View::new(&parsed.tree).patched(&expanded.patch);
                        let mut walk = WalkState::new(module, tree, self);
                        walk.walk_property(*property, &tree.get(*property).clone())?;
                        walk.flush_flows()?;
                    }

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

        // commit the fields this literal takes
        if let Some(target) = target {
            let key_type = self.intern_object(&target_fields)?;
            let subject = self
                .member_subject(origin, target, target, dir::MemberSpace::Instance)?
                .with_key_source(key_type);
            self.module_mut(module).members_tail.commit_subject(
                dir::MemberSite::Node(node.into_any()),
                subject,
                None,
            );
        }

        // name the construct's write obligation as the parent cause of each field check
        let write_cause = target.map(|_| {
            self.intern_cause(Cause::root(
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
                            let field_cause = self.intern_cause(match write_cause {
                                Some(parent) => {
                                    Cause::child(field_origin, CauseKind::Field { key }, parent)
                                }
                                None => Cause::root(field_origin, CauseKind::Field { key }),
                            });
                            let store = match field.is_optional {
                                true => StoreTarget::Optional,
                                false => StoreTarget::Exact,
                            };
                            let field_check = self.check_node(
                                source_site,
                                Expectation {
                                    store,
                                    ..Expectation::assignable(
                                        field.access.store(),
                                        field_cause,
                                        ValueUse::Store,
                                    )
                                },
                            )?;

                            // fold in the mismatch the field site already reported
                            check = check.and(match field_check.outcome {
                                CheckOutcome::Fails(_) => {
                                    CheckOutcome::Fails(CheckFailure::Reported)
                                }
                                outcome => outcome,
                            });

                            match field_check.outcome {
                                CheckOutcome::Holds => field_check.target,
                                _ => field_check.source,
                            }
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

                    let spread = self.resolve_structurally(site, spread)?;

                    // poison the literal when its spread source already reported an error
                    if self.has_error_operand(&[spread])? {
                        let source = self.poison_node(node.into_any())?;
                        let target = target.unwrap_or(source);

                        return Ok(ValueCheck {
                            source,
                            outcome: CheckOutcome::Fails(CheckFailure::Reported),
                            target,
                        });
                    }

                    // reject a source that carries no object fields
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

                    self.commit_spread_subject(site, property, spread, &spread_fields)?;
                    for field in spread_fields {
                        fields.insert(field.key, field);
                    }
                }
            }
        }

        // merge the shape at the literal node
        let fields: Vec<dir::TypeProperty> = fields.into_values().collect();
        let shape = self.intern_object(&fields)?;

        // check the merged shape against the construction target
        match target {
            // struct literals must fill their declared fields
            Some(target) => {
                // bind open construction arguments from their written fields
                if self.type_flags(target)?.has_variable() {
                    self.constrain_struct_construction(origin, &fields, target)?;
                }

                // validate the final key set against the selected constructor fields
                let failure = target_fields
                    .iter()
                    .find(|field| {
                        !field.is_optional && !fields.iter().any(|written| written.key == field.key)
                    })
                    .map(|field| CheckFailure::MissingRequiredProperty { key: field.key })
                    .or_else(|| {
                        fields
                            .iter()
                            .find(|written| {
                                !target_fields.iter().any(|field| field.key == written.key)
                            })
                            .map(|field| CheckFailure::ExcessProperty { key: field.key })
                    });
                if matches!(
                    check,
                    CheckOutcome::Holds | CheckOutcome::Fails(CheckFailure::Reported)
                ) && let Some(failure) = failure
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
            // object literals bind their merged shape
            None => {
                self.commit_node_type(node.into_any(), shape)?;
                Ok(ValueCheck {
                    source: shape,
                    outcome: check,
                    target: shape,
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
    pub(in crate::sema) fn spread_fields(
        &mut self,
        origin: Origin,
        module: ModuleId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<Vec<dir::TypeProperty>>> {
        // require a resolved spread source before merging its fields
        let ty = self.shallow_resolve(ty)?;
        if let Some(variable) = self.root_variable(ty)? {
            return Err(CompilerError::Internal {
                message: format!("spread source {ty:?} has open variable {variable:?}"),
            });
        }

        // peel managed forms down to the value they hold
        let mut current = ty;
        while let dir::Type::Form(form) = self.ty(current)? {
            if !matches!(form.form, dir::Form::Owned) {
                break;
            }
            current = self.shallow_resolve(form.value)?;
        }

        // read the fields each head spreads
        match self.ty(current)? {
            // object shapes spread their fields directly
            dir::Type::Object(shape) => Ok(Some(
                self.object_properties(current.module_id, shape.properties)?
                    .to_vec(),
            )),
            // instances spread their visible fields
            dir::Type::Application(instance) => {
                let keys = self.nominal_member_keys(instance.symbol)?;
                let mut fields = Vec::with_capacity(keys.len());
                let mut seen = SmallVec::<[dir::StaticKey; 8]>::new();
                for key in keys {
                    // keep the first member declared for each key
                    if seen.contains(&key) {
                        continue;
                    }
                    seen.push(key);

                    // read each member through the receiver instance
                    let subject =
                        self.member_subject(origin, current, current, dir::MemberSpace::Instance)?;
                    let lookup = self.lookup_member(origin, module, subject, key)?;
                    let ty = self.member_read_type(&lookup)?;
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
