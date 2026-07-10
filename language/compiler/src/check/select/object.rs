use crate::check::{
    Answer, CheckState, Decision, FlowPointId, FlowSite, MemberCandidate, MemberLookup, MemberRole,
    Origin, answer,
};
use crate::{CompilerError, CompilerResult};
use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

/// Field lookup result for one object destructuring key.
#[derive(Debug, Clone)]
enum ObjectField {
    /// Lowerable field projection.
    Projection(dir::Projection),
    /// Field lookup found no member.
    Missing,
    /// Field lookup found an invalid member and reported it.
    Rejected,
}

impl CheckState<'_> {
    /// Select one object pattern, projecting fields by key.
    pub(in crate::check) fn select_object_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        scrutinee: dir::GlobalTypeId,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;
        self.check_pattern_field_keys(module, fields)?;
        self.check_pattern_bindings(module, fields)?;

        if !self.check_pattern_rest_fields(module, fields) {
            self.commit_decision(node.into_any(), Decision::Rejected)?;

            return Ok(Answer::Ready(()));
        }

        if !answer!(self.is_keyed_type(origin, scrutinee)?) {
            self.report_pattern_source_not_object_shaped(origin, scrutinee)?;
            self.commit_decision(node.into_any(), Decision::Rejected)?;

            return Ok(Answer::Ready(()));
        }

        let (fields, rest) =
            answer!(self.project_named_fields(node, origin, flow, scope, scrutinee, fields)?);

        self.commit_pattern(
            node,
            dir::PatternResolution::Destructure(dir::PatternDestructureResolution::Object(
                dir::PatternObjectDestructureResolution { fields, rest },
            )),
        )
    }

    /// Select one object assignment pattern.
    pub(in crate::check) fn select_assign_object_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::AssignPattern>,
        origin: Origin,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        scrutinee: dir::GlobalTypeId,
        fields: &[dir::LocalNodeId<dir::AssignPatternField>],
    ) -> CompilerResult<Answer<bool>> {
        let module = node.module_id;
        self.check_assign_pattern_field_keys(module, fields)?;

        if !self.check_assign_pattern_rest_fields(module, fields) {
            self.commit_decision(node.into_any(), Decision::Rejected)?;

            return Ok(Answer::Ready(false));
        }

        if !answer!(self.is_keyed_type(origin, scrutinee)?) {
            self.report_pattern_source_not_object_shaped(origin, scrutinee)?;
            self.commit_decision(node.into_any(), Decision::Rejected)?;

            return Ok(Answer::Ready(false));
        }

        let Some((fields, rest)) = answer!(
            self.project_assign_named_fields(node, origin, flow, scope, scrutinee, fields)?
        ) else {
            self.commit_decision(node.into_any(), Decision::Rejected)?;

            return Ok(Answer::Ready(false));
        };

        let () = answer!(self.commit_assign_pattern(
            node,
            dir::AssignPatternResolution::Object(dir::AssignPatternObjectResolution {
                fields,
                rest,
            }),
        )?);

        Ok(Answer::Ready(true))
    }

    /// Project named pattern fields from one input type.
    pub(in crate::check) fn project_named_fields(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        owner: dir::GlobalTypeId,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<
        Answer<(
            Vec<dir::PatternFieldResolution>,
            Option<dir::PatternFieldResolution>,
        )>,
    > {
        let module = node.module_id;

        let mut projected = Vec::with_capacity(fields.len());
        let mut projected_keys = SmallVec::<[dir::StaticKey; 8]>::new();
        let mut rest = None;
        for field in fields {
            let field_node = self.module(module).view().get(*field).clone();
            let field_origin = Origin::Node(field.into_global_any(module), scope);
            let (key, pattern) = match field_node {
                dir::PatternField::Named { name, pattern, .. } => (name.static_key(), pattern),
                dir::PatternField::Computed { key, pattern } => {
                    let key_node = key.into_global_any(module);
                    let key_site = FlowSite {
                        node: key_node,
                        flow,
                        scope,
                    };

                    let Some(key) = answer!(self.select_static_key(key_site, key)?) else {
                        let Some(projection) = answer!(
                            self.subscript_read_projection(origin, module, owner, key_site)?
                        ) else {
                            self.report_computed_pattern_key_not_valid(module, key.into_any());

                            continue;
                        };

                        self.project_pattern_input(
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
                    let Some(projection) = answer!(self.object_rest_projection(
                        rest_origin,
                        module,
                        owner,
                        &projected_keys
                    )?) else {
                        self.report_spread_not_object(rest_origin, owner)?;

                        continue;
                    };

                    if let Some(pattern) = pattern {
                        self.project_pattern_input(
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

            let selected = answer!(self.object_field(field_origin, module, owner, key)?);
            let projection = match selected {
                ObjectField::Projection(projection) => projection,
                ObjectField::Missing => {
                    if let Some(pattern) =
                        pattern.filter(|pattern| self.is_defaulted_pattern(module, *pattern))
                    {
                        let undefined = self.intern_type(module, dir::Type::Undefined)?;

                        self.project_pattern_input(
                            flow,
                            scope,
                            undefined,
                            pattern.into_global_any(module),
                        )?;
                        projected.push(dir::PatternFieldResolution {
                            source: field.into_global_any(module),
                            projection: dir::Projection::FieldGet {
                                field: dir::ProjectionField::Key(key),
                                ty: undefined,
                            },
                            pattern: Some(pattern.into_global_any(module)),
                        });
                        projected_keys.push(key);
                    } else {
                        let key = self.format_static_key(&key);
                        self.report_pattern_field_missing(field_origin, owner, key)?;
                    }

                    continue;
                }
                ObjectField::Rejected => continue,
            };
            let projected_value = projection.ty();

            // flow the projected value into the nested or shorthand hole
            match pattern {
                Some(pattern) => {
                    self.project_pattern_input(
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

        Ok(Answer::Ready((projected, rest)))
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
        Answer<
            Option<(
                Vec<dir::AssignPatternFieldResolution>,
                Option<dir::AssignPatternRestResolution>,
            )>,
        >,
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
                    (name.static_key(), Some(pattern))
                }
                dir::AssignPatternField::Computed { key, pattern } => {
                    let key_node = key.into_global_any(module);
                    let key_site = FlowSite {
                        node: key_node,
                        flow,
                        scope,
                    };

                    let Some(key) = answer!(self.select_static_key(key_site, key)?) else {
                        let Some(projection) = answer!(
                            self.subscript_read_projection(origin, module, owner, key_site)?
                        ) else {
                            self.report_computed_pattern_key_not_valid(module, key.into_any());

                            return Ok(Answer::Ready(None));
                        };

                        self.project_pattern_input(
                            flow,
                            scope,
                            projection.ty(),
                            pattern.into_global_any(module),
                        )?;
                        projected.push(dir::AssignPatternFieldResolution {
                            source,
                            projection,
                            pattern: Some(pattern.into_global_any(module)),
                        });

                        continue;
                    };

                    (key, Some(pattern))
                }
                dir::AssignPatternField::Rest { pattern } => {
                    let rest_origin = Origin::Node(source, scope);
                    let Some(projection) = answer!(self.object_rest_projection(
                        rest_origin,
                        module,
                        owner,
                        &projected_keys
                    )?) else {
                        self.report_spread_not_object(rest_origin, owner)?;

                        return Ok(Answer::Ready(None));
                    };
                    let rest_type = projection.ty();

                    if let Some(pattern) = pattern {
                        self.project_pattern_input(
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

            let selected = answer!(self.object_field(field_origin, module, owner, key)?);
            let projection = match selected {
                ObjectField::Projection(projection) => projection,
                ObjectField::Missing => {
                    let Some(pattern) = pattern else {
                        return Err(CompilerError::Internal {
                            message: "named assignment field has no target pattern".to_string(),
                        });
                    };

                    if self.is_defaulted_assign_pattern(module, pattern) {
                        let undefined = self.intern_type(module, dir::Type::Undefined)?;

                        self.project_pattern_input(
                            flow,
                            scope,
                            undefined,
                            pattern.into_global_any(module),
                        )?;
                        projected.push(dir::AssignPatternFieldResolution {
                            source: field.into_global_any(module),
                            projection: dir::Projection::FieldGet {
                                field: dir::ProjectionField::Key(key),
                                ty: undefined,
                            },
                            pattern: Some(pattern.into_global_any(module)),
                        });
                        projected_keys.push(key);
                    } else {
                        let key = self.format_static_key(&key);
                        self.report_pattern_field_missing(field_origin, owner, key)?;

                        return Ok(Answer::Ready(None));
                    }

                    continue;
                }
                ObjectField::Rejected => return Ok(Answer::Ready(None)),
            };
            let projected_value = projection.ty();

            // flow the projected value into the nested target
            let Some(pattern) = pattern else {
                return Err(CompilerError::Internal {
                    message: "named assignment field has no target pattern".to_string(),
                });
            };
            self.project_pattern_input(
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

        Ok(Answer::Ready(Some((projected, rest))))
    }

    /// Return the lowerable projection for one object destructuring key.
    fn object_field(
        &mut self,
        origin: Origin,
        module: ModuleId,
        owner: dir::GlobalTypeId,
        key: dir::StaticKey,
    ) -> CompilerResult<Answer<ObjectField>> {
        let lookup =
            answer!(self.lookup_member(origin, module, owner, dir::MemberSpace::Instance, key)?);

        let field = match lookup {
            MemberLookup::Field(ty) => ObjectField::Projection(dir::Projection::FieldGet {
                field: dir::ProjectionField::Key(key),
                ty,
            }),
            MemberLookup::Found(candidates) => {
                self.object_member_field(origin, owner, key, candidates)?
            }
            MemberLookup::Missing => ObjectField::Missing,
        };

        Ok(Answer::Ready(field))
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
                ObjectField::Projection(dir::Projection::FieldGet {
                    field: candidate
                        .field(key)
                        .unwrap_or(dir::ProjectionField::Key(key)),
                    ty: candidate.ty,
                })
            }
            [candidate] if candidate.role == MemberRole::Getter => {
                let Some(read) = candidate.getter(owner, key) else {
                    return Err(CompilerError::Internal {
                        message: "getter member candidate has no property read".to_string(),
                    });
                };

                ObjectField::Projection(dir::Projection::PropertyGet {
                    read,
                    ty: candidate.ty,
                })
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
    ) -> CompilerResult<Answer<Option<dir::Projection>>> {
        let Some(fields) = answer!(self.spread_fields(origin, module, owner)?) else {
            return Ok(Answer::Ready(None));
        };

        let fields = fields
            .into_iter()
            .filter(|field| !omitted.contains(&field.key))
            .collect::<Vec<_>>();
        let copied = fields
            .iter()
            .map(|field| dir::ObjectRestField {
                key: field.key,
                projection: dir::Projection::FieldGet {
                    field: dir::ProjectionField::Key(field.key),
                    ty: field.ty,
                },
            })
            .collect();
        let fields = self.intern_fields(module, &fields)?;
        let shape = dir::Type::Shape(dir::ShapeType {
            fields,
            call_signatures: dir::TypeListId::EMPTY,
            construct_signatures: dir::TypeListId::EMPTY,
            index_signatures: dir::TypeListId::EMPTY,
        });
        let ty = self.intern_type(module, shape)?;

        Ok(Answer::Ready(Some(dir::Projection::ObjectRest {
            fields: copied,
            ty,
        })))
    }
}
