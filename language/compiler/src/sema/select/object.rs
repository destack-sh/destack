use std::slice;

use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{
    BodyState, FlowPointId, FlowSite, MemberCandidate, MemberLookup, MemberRole, Origin, PlaceUse,
    Value,
};
use crate::{CompilerError, CompilerResult};

/// Field lookup result for one object destructuring key.
#[derive(Debug, Clone)]
enum ObjectField {
    /// Lowerable field projection.
    Projection(Box<dir::ProjectionResolution>),
    /// Field lookup found no member.
    Missing,
    /// Field lookup found an invalid member and reported it.
    Rejected,
}

impl BodyState<'_, '_> {
    /// Select one object pattern, projecting fields by key.
    pub(in crate::sema) fn select_object_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        scrutinee: dir::GlobalTypeId,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<()> {
        // check the keys, bindings, and rest field the pattern declares
        let module = node.module_id;
        self.check_pattern_field_keys(module, fields)?;
        self.check_pattern_bindings(module, fields)?;
        if !self.check_pattern_rest_fields(module, fields) {
            return self.commit_rejected_pattern(node);
        }

        // project fields off the scrutinee subset the pattern's literal tests accept
        let tests = self.object_pattern_tests(module, flow, scope, fields)?;
        let scrutinee = self.narrow_pattern_scrutinee(origin, scrutinee, &tests)?;

        // reject a scrutinee that carries no keys
        if !self.is_keyed_type(origin, scrutinee)? {
            self.report_pattern_source_not_object_shaped(origin, scrutinee)?;

            return self.commit_rejected_pattern(node);
        }

        // destructure each named field, collecting the rest
        let (fields, rest) =
            self.project_named_fields(node, origin, flow, scope, scrutinee, fields)?;

        self.commit_pattern(
            node,
            dir::PatternDecision::Destructure(Box::new(dir::PatternDestructureResolution::Object(
                dir::PatternObjectDestructureResolution {
                    fields,
                    rest: rest.map(Box::new),
                },
            ))),
        )
    }

    /// Select one object assignment pattern.
    pub(in crate::sema) fn select_assign_object_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::AssignPattern>,
        origin: Origin,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        scrutinee: dir::GlobalTypeId,
        fields: &[dir::LocalNodeId<dir::AssignPatternField>],
    ) -> CompilerResult<bool> {
        let module = node.module_id;
        self.check_assign_pattern_field_keys(module, fields)?;

        if !self.check_assign_pattern_rest_fields(module, fields) {
            self.commit_decision(node.into_any(), dir::Decision::Rejected)?;

            return Ok(false);
        }

        // destructure the physical value beneath nominal wrappers
        let scrutinee = self.narrow_pattern_scrutinee(origin, scrutinee, &[])?;

        if !self.is_keyed_type(origin, scrutinee)? {
            self.report_pattern_source_not_object_shaped(origin, scrutinee)?;
            self.commit_decision(node.into_any(), dir::Decision::Rejected)?;

            return Ok(false);
        }

        let Some((fields, rest)) =
            self.project_assign_named_fields(node, origin, flow, scope, scrutinee, fields)?
        else {
            self.commit_decision(node.into_any(), dir::Decision::Rejected)?;

            return Ok(false);
        };

        let () = self.commit_assign_pattern(
            node,
            dir::AssignPatternDecision::Object(dir::AssignPatternObjectResolution {
                fields,
                rest: rest.map(Box::new),
            }),
        )?;

        Ok(true)
    }

    /// Project named pattern fields from one input type.
    pub(in crate::sema) fn project_named_fields(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        owner: dir::GlobalTypeId,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<(
        Vec<dir::PatternFieldResolution>,
        Option<dir::PatternFieldResolution>,
    )> {
        let module = node.module_id;

        let mut projected = Vec::with_capacity(fields.len());
        let mut projected_keys = SmallVec::<[dir::StaticKey; 8]>::new();
        let mut rest = None;
        for field in fields {
            let field_node = self.module(module).view().get(*field).clone();
            let field_origin = Origin::Node(field.into_global_any(module), scope);
            let (key, pattern) = match field_node {
                dir::PatternField::Named { name, pattern, .. } => (name.into(), pattern),
                dir::PatternField::Computed { key, pattern } => {
                    let key_node = key.into_global_any(module);
                    let key_site = FlowSite {
                        node: key_node,
                        flow,
                        scope,
                    };

                    let Some(key) = self.select_static_key(key_site)? else {
                        let Some(projection) =
                            self.subscript_read_projection(origin, module, owner, key_site)?
                        else {
                            self.report_computed_pattern_key_not_valid(module, key.into_any());
                            self.poison_pattern_field(field.into_global(module))?;

                            continue;
                        };

                        self.check_pattern_projection(
                            flow,
                            scope,
                            projection.ty(),
                            pattern.into_global_any(module),
                        )?;
                        projected.push(dir::PatternFieldResolution {
                            source: field.into_global_any(module),
                            projection,
                            pattern: Some(pattern.into_global_any(module)),
                        });

                        continue;
                    };

                    (key, Some(pattern))
                }
                dir::PatternField::Rest { pattern } => {
                    let rest_origin = Origin::Node(field.into_global_any(module), scope);
                    let Some(projection) =
                        self.object_rest_projection(rest_origin, module, owner, &projected_keys)?
                    else {
                        self.report_spread_not_object(rest_origin, owner)?;

                        continue;
                    };

                    if let Some(pattern) = pattern {
                        self.check_pattern_projection(
                            flow,
                            scope,
                            projection.ty(),
                            pattern.into_global_any(module),
                        )?;
                    }

                    rest = Some(dir::PatternFieldResolution {
                        source: field.into_global_any(module),
                        projection,
                        pattern: pattern.map(|pattern| pattern.into_global_any(module)),
                    });

                    continue;
                }
                _ => continue,
            };

            let selected = self.object_field(field_origin, module, owner, key)?;
            let projection = match selected {
                ObjectField::Projection(projection) => *projection,
                ObjectField::Missing => {
                    if let Some(pattern) =
                        pattern.filter(|pattern| self.is_defaulted_pattern(module, *pattern))
                    {
                        let undefined = self.intern_type(dir::Type::Undefined)?;

                        self.check_pattern_projection(
                            flow,
                            scope,
                            undefined,
                            pattern.into_global_any(module),
                        )?;
                        projected.push(dir::PatternFieldResolution {
                            source: field.into_global_any(module),
                            projection: dir::Projection::Absent { ty: undefined }.into(),
                            pattern: Some(pattern.into_global_any(module)),
                        });
                        projected_keys.push(key);
                    } else {
                        let key = self.format_static_key(&key);
                        self.report_pattern_field_missing(field_origin, owner, key)?;
                        self.poison_pattern_field(field.into_global(module))?;
                    }

                    continue;
                }
                ObjectField::Rejected => {
                    self.poison_pattern_field(field.into_global(module))?;

                    continue;
                }
            };
            let projected_value = projection.ty();

            // flow the projected value into the nested or shorthand hole
            match pattern {
                Some(pattern) => {
                    self.check_pattern_projection(
                        flow,
                        scope,
                        projected_value,
                        pattern.into_global_any(module),
                    )?;
                }
                // shorthand fields bind through their own field node
                None => {
                    if let Some(symbol) =
                        self.module(module).declaration_symbol((*field).into_any())
                    {
                        let input = self.pattern_binding_type(symbol, projected_value)?;

                        self.bind_symbol_type(symbol, input)?;
                    }
                }
            }
            projected.push(dir::PatternFieldResolution {
                source: field.into_global_any(module),
                projection,
                pattern: pattern.map(|pattern| pattern.into_global_any(module)),
            });
            projected_keys.push(key);
        }

        Ok((projected, rest))
    }

    /// Project named assignment fields from one input type.
    fn project_assign_named_fields(
        &mut self,
        node: dir::GlobalNodeId<dir::AssignPattern>,
        origin: Origin,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        owner: dir::GlobalTypeId,
        fields: &[dir::LocalNodeId<dir::AssignPatternField>],
    ) -> CompilerResult<
        Option<(
            Vec<dir::AssignPatternFieldResolution>,
            Option<dir::AssignPatternRestResolution>,
        )>,
    > {
        let module = node.module_id;

        let mut projected = Vec::with_capacity(fields.len());
        let mut projected_keys = SmallVec::<[dir::StaticKey; 8]>::new();
        let mut rest = None;
        for field in fields {
            let source = field.into_global_any(module);
            let field_origin = Origin::Node(source, scope);
            let field_node = self.module(module).view().get(*field).clone();
            let (key, pattern) = match field_node {
                dir::AssignPatternField::Named { name, pattern, .. } => {
                    (name.into(), Some(pattern))
                }
                dir::AssignPatternField::Computed { key, pattern } => {
                    let key_node = key.into_global_any(module);
                    let key_site = FlowSite {
                        node: key_node,
                        flow,
                        scope,
                    };

                    let Some(static_key) = self.select_static_key(key_site)? else {
                        let projected_field = self.project_computed_assign_field(
                            origin, owner, source, key, key_site, pattern,
                        )?;
                        let Some(projected_field) = projected_field else {
                            return Ok(None);
                        };
                        projected.push(projected_field);

                        continue;
                    };

                    (static_key, Some(pattern))
                }
                dir::AssignPatternField::Rest { pattern } => {
                    let rest_origin = Origin::Node(source, scope);
                    let Some(projection) =
                        self.object_rest_projection(rest_origin, module, owner, &projected_keys)?
                    else {
                        self.report_spread_not_object(rest_origin, owner)?;

                        return Ok(None);
                    };
                    let rest_type = projection.ty();

                    if let Some(pattern) = pattern {
                        self.check_pattern_projection(
                            flow,
                            scope,
                            rest_type,
                            pattern.into_global_any(module),
                        )?;
                    }

                    rest = Some(dir::AssignPatternRestResolution {
                        source,
                        projection,
                        pattern: pattern.map(|pattern| pattern.into_global_any(module)),
                    });

                    continue;
                }
                _ => continue,
            };

            // named and computed fields both carry a target pattern
            let Some(pattern) = pattern else {
                return Err(CompilerError::Internal {
                    message: "named assignment field has no target pattern".to_string(),
                });
            };

            let selected = self.object_field(field_origin, module, owner, key)?;
            let projection = match selected {
                ObjectField::Projection(projection) => *projection,
                ObjectField::Missing => {
                    let projected_field = self
                        .project_missing_assign_field(flow, scope, owner, source, key, pattern)?;
                    let Some(projected_field) = projected_field else {
                        return Ok(None);
                    };
                    projected.push(projected_field);
                    projected_keys.push(key);

                    continue;
                }
                ObjectField::Rejected => return Ok(None),
            };
            let projected_value = projection.ty();

            // flow the projected value into the nested target
            self.check_pattern_projection(
                flow,
                scope,
                projected_value,
                pattern.into_global_any(module),
            )?;
            projected.push(dir::AssignPatternFieldResolution {
                source: field.into_global_any(module),
                projection,
                pattern: Some(pattern.into_global_any(module)),
            });
            projected_keys.push(key);
        }

        Ok(Some((projected, rest)))
    }

    /// Project one computed assignment field whose key is not statically known.
    fn project_computed_assign_field(
        &mut self,
        origin: Origin,
        owner: dir::GlobalTypeId,
        source: dir::GlobalNodeIdAny,
        key: dir::LocalNodeId<dir::Expression>,
        key_site: FlowSite,
        pattern: dir::LocalNodeId<dir::AssignPattern>,
    ) -> CompilerResult<Option<dir::AssignPatternFieldResolution>> {
        let module = source.module_id;
        let Some(projection) = self.subscript_read_projection(origin, module, owner, key_site)?
        else {
            self.report_computed_pattern_key_not_valid(module, key.into_any());

            // decide the place target even after rejecting the key
            if let dir::AssignPattern::Place { expression } =
                self.module(module).view().get(pattern)
            {
                let place = expression.into_global_any(module);
                self.decide_reference(place)?;
            }

            return Ok(None);
        };

        self.check_pattern_projection(
            key_site.flow,
            key_site.scope,
            projection.ty(),
            pattern.into_global_any(module),
        )?;

        Ok(Some(dir::AssignPatternFieldResolution {
            source,
            projection,
            pattern: Some(pattern.into_global_any(module)),
        }))
    }

    /// Project one named assignment field the input type does not declare.
    fn project_missing_assign_field(
        &mut self,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        owner: dir::GlobalTypeId,
        source: dir::GlobalNodeIdAny,
        key: dir::StaticKey,
        pattern: dir::LocalNodeId<dir::AssignPattern>,
    ) -> CompilerResult<Option<dir::AssignPatternFieldResolution>> {
        let module = source.module_id;

        // report a field the input type neither declares nor defaults
        if !self.is_defaulted_assign_pattern(module, pattern) {
            let origin = Origin::Node(source, scope);
            let key = self.format_static_key(&key);
            self.report_pattern_field_missing(origin, owner, key)?;

            return Ok(None);
        }

        // bind a defaulted target against undefined
        let undefined = self.intern_type(dir::Type::Undefined)?;
        self.check_pattern_projection(flow, scope, undefined, pattern.into_global_any(module))?;

        Ok(Some(dir::AssignPatternFieldResolution {
            source,
            projection: dir::Projection::Absent { ty: undefined }.into(),
            pattern: Some(pattern.into_global_any(module)),
        }))
    }

    /// Collect the member values one object pattern's literal fields test.
    fn object_pattern_tests(
        &mut self,
        module: ModuleId,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<Vec<(dir::StaticKey, dir::GlobalTypeId)>> {
        let mut tests = Vec::new();

        // collect one test per named field carrying an expression sub-pattern
        for field in fields {
            let dir::PatternField::Named {
                name,
                pattern: Some(pattern),
                ..
            } = self.module(module).view().get(*field).clone()
            else {
                continue;
            };
            let dir::Pattern::Expression { value } = *self.module(module).view().get(pattern)
            else {
                continue;
            };

            // singleton-valued sub-patterns test their field exactly
            let site = FlowSite {
                node: value.into_global_any(module),
                flow,
                scope,
            };
            let ty = self.infer_node_type(site, PlaceUse::Read)?;
            if self.is_singleton_type(ty)? {
                tests.push((name.into(), ty));
            }
        }

        Ok(tests)
    }

    /// Narrow one scrutinee to the subset the pattern can match, keeping it whole when stuck.
    fn narrow_pattern_scrutinee(
        &mut self,
        origin: Origin,
        scrutinee: dir::GlobalTypeId,
        tests: &[(dir::StaticKey, dir::GlobalTypeId)],
    ) -> CompilerResult<dir::GlobalTypeId> {
        // destructure the physical value beneath nominal wrappers
        let (mut narrowed, _) = self.project_newtype_receiver(origin, scrutinee)?;

        // filter the alternatives through each tested member value
        for (key, tested) in tests {
            let keys = slice::from_ref(key);
            narrowed = match self.narrow_type_alternatives(origin, narrowed, keys, *tested, true)? {
                Ok(Some(next)) => next,
                Ok(None) | Err(_) => narrowed,
            };
        }

        // keep the whole scrutinee when the tests leave nothing to project
        if matches!(self.ty(narrowed)?, dir::Type::Never) {
            return Ok(scrutinee);
        }

        Ok(narrowed)
    }

    /// Select the field projection one object destructuring key names.
    fn object_field(
        &mut self,
        origin: Origin,
        module: ModuleId,
        owner: dir::GlobalTypeId,
        key: dir::StaticKey,
    ) -> CompilerResult<ObjectField> {
        let subject = dir::MemberSubject::new(owner, owner, dir::MemberSpace::Instance);
        let lookup = self.lookup_member(origin, module, subject, key)?;

        let field = self.object_lookup_field(origin, owner, key, lookup)?;

        Ok(field)
    }

    /// Return the lowerable projection selected by one completed member lookup.
    fn object_lookup_field(
        &mut self,
        origin: Origin,
        owner: dir::GlobalTypeId,
        key: dir::StaticKey,
        lookup: MemberLookup,
    ) -> CompilerResult<ObjectField> {
        let field = match lookup {
            MemberLookup::Field(field) => {
                // write-only properties expose nothing to an object read
                let Some(projection) = field.projection(owner, key, self)? else {
                    return Ok(ObjectField::Missing);
                };

                ObjectField::Projection(Box::new(projection.into()))
            }
            MemberLookup::Found(candidates) => {
                self.object_member_field(origin, owner, key, candidates)?
            }
            MemberLookup::Union(lookups) => {
                let mut projections = Vec::with_capacity(lookups.len());
                let mut types = Vec::with_capacity(lookups.len());
                for arm in lookups {
                    let ObjectField::Projection(projection) =
                        self.object_lookup_field(origin, arm.receiver, key, arm.lookup)?
                    else {
                        return Ok(ObjectField::Rejected);
                    };
                    let dir::OperationResolution::One(projection) = *projection else {
                        return Err(CompilerError::Internal {
                            message: "union object lookup produced a nested union projection"
                                .to_string(),
                        });
                    };
                    types.push(projection.ty());
                    projections.push(projection);
                }
                let ty = self.normalized_union_type(types)?;

                ObjectField::Projection(Box::new(dir::OperationResolution::Union {
                    arms: projections,
                    ty,
                }))
            }
            lookup @ MemberLookup::Intersection(_) => {
                let Some(resolution) = self.select_member_read(
                    origin,
                    Value {
                        ty: owner,
                        place: None,
                    },
                    key,
                    &lookup,
                )?
                else {
                    return Ok(ObjectField::Rejected);
                };

                ObjectField::Projection(Box::new(resolution.into()))
            }
            MemberLookup::Missing => ObjectField::Missing,
        };

        Ok(field)
    }

    /// Return the lowerable projection for declaration-backed object fields.
    fn object_member_field(
        &mut self,
        origin: Origin,
        owner: dir::GlobalTypeId,
        key: dir::StaticKey,
        candidates: Vec<MemberCandidate>,
    ) -> CompilerResult<ObjectField> {
        let field = match candidates.as_slice() {
            [candidate] if candidate.role == MemberRole::Field => {
                let Some(field) = candidate.field(key) else {
                    return Err(CompilerError::Internal {
                        message: "field member candidate has no field projection".to_string(),
                    });
                };

                ObjectField::Projection(Box::new(
                    dir::Projection::Field(dir::FieldResolution {
                        receiver: candidate.receiver.resolve(owner),
                        target: field,
                        ty: candidate
                            .read_type(self)?
                            .ok_or_else(|| CompilerError::Internal {
                                message: format!(
                                    "field {:?} has no readable type",
                                    candidate.symbol
                                ),
                            })?,
                    })
                    .into(),
                ))
            }
            [candidate] if candidate.role == MemberRole::Getter => {
                let call = self.select_getter_call(
                    origin,
                    Value {
                        ty: owner,
                        place: None,
                    },
                    candidate,
                )?;
                // the owner's own getter must accept its own receiver
                let Some(call) = call else {
                    return Err(CompilerError::Internal {
                        message: format!("getter {:?} rejects its own owner", candidate.symbol),
                    });
                };

                let projection = dir::Projection::Call(Box::new(call));

                ObjectField::Projection(Box::new(projection.into()))
            }
            [_candidate] => {
                let key = self.format_static_key(&key);
                self.report_pattern_member_not_field(origin, owner, key)?;

                ObjectField::Rejected
            }
            [] => ObjectField::Missing,
            _ => {
                let key = self.format_static_key(&key);
                self.report_ambiguous_member(origin, key)?;

                ObjectField::Rejected
            }
        };

        Ok(field)
    }

    /// Return the object rest projection after omitting selected keys.
    fn object_rest_projection(
        &mut self,
        origin: Origin,
        module: ModuleId,
        owner: dir::GlobalTypeId,
        omitted: &[dir::StaticKey],
    ) -> CompilerResult<Option<dir::ProjectionResolution>> {
        let Some(fields) = self.spread_fields(origin, module, owner)? else {
            return Ok(None);
        };

        let fields = fields
            .into_iter()
            .filter(|field| !omitted.contains(&field.key))
            .collect::<Vec<_>>();
        let copied = fields
            .iter()
            .map(|field| dir::ObjectRestField {
                key: field.key,
                projection: dir::Projection::Field(dir::FieldResolution {
                    receiver: dir::MemberReceiver::direct(owner),
                    target: dir::FieldTarget::Structural {
                        owner,
                        key: field.key,
                    },
                    ty: field.access.store(),
                }),
            })
            .collect();
        let ty = self.intern_object(&fields)?;

        let projection = dir::Projection::ObjectRest { fields: copied, ty };

        Ok(Some(projection.into()))
    }
}
