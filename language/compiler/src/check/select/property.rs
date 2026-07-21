use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexMap;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, BodyState, Cause, CauseKind, CheckFailure, CheckOutcome, Constraint, Decision,
    Dependency, FlowSite, Origin, PlaceUse, Relation, ValueUse, answer,
};

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
    ) -> CompilerResult<Answer<CheckOutcome>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let origin = site.origin();

        // collect the property entries in source order
        let mut entries = SmallVec::<[MergeEntry; 8]>::new();
        for property in properties {
            match self.module(module).view().get(*property).clone() {
                dir::Property::Field { key, value, .. } => {
                    let Some(key) = answer!(self.select_property_key(site, key)?) else {
                        continue;
                    };

                    entries.push(MergeEntry::Field {
                        key,
                        source: value.into_global_any(module),
                    });
                }
                dir::Property::Method { key, .. } => {
                    let Some(key) = answer!(match key {
                        Some(key) => self.select_property_key(site, key)?,
                        None => Answer::Ready(None),
                    }) else {
                        continue;
                    };

                    entries.push(MergeEntry::Field {
                        key,
                        source: property.into_global_any(module),
                    });
                }
                dir::Property::Spread { value } => entries.push(MergeEntry::Spread {
                    source: value.into_global_any(module),
                }),
                dir::Property::Error => {}
            }
        }

        // collect declared fields through the selected construct target
        let target_fields = match target {
            Some(target) => answer!(self.struct_field_types(origin, target)?),
            None => SmallVec::new(),
        };

        // merge entry fields left to right, later keys overriding
        let mut check = CheckOutcome::Holds;
        let mut fields = IndexMap::<dir::StaticKey, dir::TypeField>::new();
        for entry in entries {
            match entry {
                // direct fields use declared struct field types when available
                MergeEntry::Field { key, source } => {
                    let expected = target_fields.iter().find(|field| field.key == key);
                    let ty = match expected {
                        Some(field) if source.local_id.ty == dir::NodeType::Expression => {
                            let source_site = self.node_site(source)?;
                            // optional fields accept their value or undefined
                            let expected_ty = match field.is_optional {
                                true => {
                                    let module = origin.module();
                                    let undefined =
                                        self.intern_type(module, dir::Type::Undefined)?;

                                    self.normalized_union_type(module, [field.ty, undefined])?
                                }
                                false => field.ty,
                            };
                            let field_cause = self.check.intern_cause(Cause::root(
                                Origin::Node(source, site.scope),
                                CauseKind::Field { key },
                            ));
                            let field_check = answer!(self.check_node_expected(
                                source_site,
                                expected_ty,
                                Relation::Assignable,
                                field_cause,
                                ValueUse::Store,
                            )?);
                            check = check.and(field_check.outcome);

                            answer!(self.node_type_at(source_site)?)
                        }
                        _ => answer!(self.merge_entry_type(source)?),
                    };
                    fields.insert(
                        key,
                        dir::TypeField {
                            key,
                            ty,
                            is_optional: false,
                            is_readonly: false,
                        },
                    );
                }
                // spread sources contribute every visible field
                MergeEntry::Spread { source } => {
                    let source_site = self.node_site(source)?;
                    let spread = answer!(self.infer_node_type(source_site, PlaceUse::Read)?);
                    let Some(spread_fields) = answer!(self.spread_fields(origin, module, spread)?)
                    else {
                        // reject spreads with no field projection
                        return self.reject_spread(origin, node, source, spread);
                    };
                    for field in spread_fields {
                        fields.insert(field.key, field);
                    }
                }
            }
        }

        // merge the shape at the literal node
        let fields: Vec<dir::TypeField> = fields.into_values().collect();
        let field_list = fields.clone();
        let fields = self.intern_fields(module, &fields)?;
        let shape = self.intern_type(
            module,
            dir::Type::Shape(dir::ShapeType {
                fields,
                call_signatures: dir::TypeListId::EMPTY,
                construct_signatures: dir::TypeListId::EMPTY,
                index_signatures: dir::TypeListId::EMPTY,
            }),
        )?;

        match target {
            // struct literals must fill their declared fields
            Some(target) => {
                // bound open construction arguments before the writable check
                if self.type_flags(target)?.has_variable() {
                    answer!(self.constrain_struct_construction(origin, &field_list, target)?);
                }

                // commit the literal before the writable obligation; a
                //  failed field judgment already carried the report
                self.commit_node_type(node.into_any(), target)?;
                if matches!(check, CheckOutcome::Holds) {
                    let cause = self.check.intern_cause(Cause::root(
                        origin,
                        CauseKind::Write {
                            place: node.into_any(),
                        },
                    ));
                    self.push_constraint(Constraint::r#type(
                        Relation::Writable,
                        shape,
                        target,
                        cause,
                    ));
                }

                Ok(Answer::Ready(check))
            }
            // object literals bind their managed merged shape
            None => {
                let managed = self.intern_type(
                    module,
                    dir::Type::Form(dir::FormType {
                        form: dir::Form::Managed,
                        value: shape,
                    }),
                )?;
                self.commit_node_type(node.into_any(), managed)?;

                Ok(Answer::Ready(check))
            }
        }
    }

    /// Return the merged field type at one entry source node.
    fn merge_entry_type(
        &mut self,
        source: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        // method properties type through their declaration symbol
        if source.local_id.ty == dir::NodeType::Property
            && let Some(symbol) = self
                .module(source.module_id)
                .declaration_symbol(source.local_id)
        {
            return match self.symbol_type_maybe(symbol) {
                Some(ty) => Ok(Answer::Ready(ty)),
                None => Ok(Answer::pending([Dependency::SymbolType(symbol)])),
            };
        }

        let source_site = self.node_site(source)?;
        self.infer_node_type(source_site, PlaceUse::Read)
    }

    /// Return the mergeable fields of one spread source.
    pub(in crate::check) fn spread_fields(
        &mut self,
        origin: Origin,
        module: ModuleId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<Vec<dir::TypeField>>>> {
        // reduce the spread source before merging fields
        let root = answer!(self.reduce_type_head(origin, ty)?);
        if let Some(variable) = self.root_variable(root)? {
            return Ok(Answer::pending([Dependency::Variable(variable)]));
        }

        // peel managed forms to the carried value
        let mut current = root;
        while let dir::Type::Form(form) = self.ty(current)? {
            if form.form != dir::Form::Managed {
                break;
            }
            current = self.settled_root(form.value)?;
        }

        match self.ty(current)? {
            // structural shapes spread their fields directly
            dir::Type::Shape(shape) => Ok(Answer::Ready(Some(
                self.shape_fields(current.module_id, shape.fields)?.to_vec(),
            ))),
            // instances spread their visible fields
            dir::Type::Instance(instance) => {
                let keys = self.nominal_member_keys(instance.symbol)?;
                let mut fields = Vec::with_capacity(keys.len());
                let mut seen = SmallVec::<[dir::StaticKey; 8]>::new();
                for key in keys {
                    if seen.contains(&key) {
                        continue;
                    }
                    seen.push(key);

                    // read each member through the receiver instance
                    let lookup = answer!(self.lookup_member(
                        origin,
                        module,
                        current,
                        dir::MemberSpace::Instance,
                        key,
                    )?);
                    let ty = lookup.value_type();
                    if let Some(ty) = ty {
                        fields.push(dir::TypeField {
                            key,
                            ty,
                            is_optional: false,
                            is_readonly: false,
                        });
                    }
                }

                Ok(Answer::Ready(Some(fields)))
            }
            _ => Ok(Answer::Ready(None)),
        }
    }

    /// Reject one literal whose spread source has no fields.
    fn reject_spread(
        &mut self,
        origin: Origin,
        node: dir::GlobalNodeId<dir::Expression>,
        source: dir::GlobalNodeIdAny,
        spread: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<CheckOutcome>> {
        let anchored = self.origin_at(origin, source)?;
        self.report_spread_not_object(anchored, spread)?;
        self.commit_decision(node.into_any(), Decision::Rejected)?;
        self.commit_error_node(node.into_any())?;

        Ok(Answer::Ready(CheckOutcome::Fails(CheckFailure::Relation)))
    }
}
