use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{
    CheckState, FlowPointId, FlowSite, MemberCandidate, MemberLookup, MemberRole, Origin, PlaceUse,
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

impl CheckState<'_> {
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
        self.report_duplicate_pattern_fields(module, fields)?;
        self.report_duplicate_pattern_bindings(module, fields)?;
        if !self.report_pattern_rest_fields(module, fields) {
            return self.commit_rejected_pattern(node);
        }

        // project fields off the scrutinee subset the pattern's literal tests accept
        let tests = self.object_pattern_tests(module, flow, scope, fields)?;
        let source = scrutinee;
        let scrutinee = self.narrow_pattern_scrutinee(origin, scrutinee, &tests)?;
        let adjustments = self.project_narrowed_receiver(origin, source, scrutinee)?;

        // reject a scrutinee that carries no keys
        if !self.is_keyed_type(origin, scrutinee)? {
            self.report_pattern_source_not_object_shaped(origin, scrutinee)?;

            return self.commit_rejected_pattern(node);
        }

        // destructure each named field through the scrutinee's borrow, collecting the rest
        let binding = self.pattern_binding_form(origin, scrutinee)?;
        let (fields, rest) =
            self.project_named_fields(node, origin, flow, scope, scrutinee, fields, binding)?;

        // commit the object destructure
        self.commit_pattern(
            node,
            dir::PatternDecision::Destructure(Box::new(dir::PatternDestructureResolution::Object(
                dir::PatternObjectDestructureResolution {
                    adjustments,
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
        self.report_duplicate_assign_fields(module, fields)?;

        // require a well formed rest field
        if !self.report_assign_rest_fields(module, fields) {
            self.commit_decision(node.into_any(), dir::Decision::Rejected)?;

            return Ok(false);
        }

        // destructure the physical value beneath nominal wrappers
        let scrutinee = self.narrow_pattern_scrutinee(origin, scrutinee, &[])?;

        // require a keyed source to destructure
        if !self.is_keyed_type(origin, scrutinee)? {
            self.report_pattern_source_not_object_shaped(origin, scrutinee)?;
            self.commit_decision(node.into_any(), dir::Decision::Rejected)?;

            return Ok(false);
        }

        // project each named field out of the source
        let Some((fields, rest)) =
            self.project_assign_named_fields(node, origin, flow, scope, scrutinee, fields)?
        else {
            self.commit_decision(node.into_any(), dir::Decision::Rejected)?;

            return Ok(false);
        };

        // commit the object assignment pattern
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
        binding: Option<dir::Form>,
    ) -> CompilerResult<(
        Vec<dir::PatternFieldResolution>,
        Option<dir::PatternFieldResolution>,
    )> {
        let module = node.module_id;

        // project each written field in source order
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

                        let projected_value =
                            self.bound_through(origin, binding, projection.ty())?;
                        self.check_pattern_projection(
                            flow,
                            scope,
                            projected_value,
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
                        // stay silent when the owner already reported an error
                        if !self.has_error_operand(&[owner])? {
                            self.report_spread_not_object(rest_origin, owner)?;
                        }

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
                invalid @ (dir::PatternField::Positional { .. } | dir::PatternField::Elision) => {
                    return Err(CompilerError::Internal {
                        message: format!("invalid object pattern field: {invalid:?}"),
                    });
                }
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
            let projected_value = self.bound_through(origin, binding, projection.ty())?;

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
                        let input = projected_value;

                        self.commit_symbol_type(symbol, input)?;
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
                        // stay silent when the owner already reported an error
                        if !self.has_error_operand(&[owner])? {
                            self.report_spread_not_object(rest_origin, owner)?;
                        }

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

            // project the field's access from the pattern's own access
            let source = field.into_global_any(module);
            self.commit_projected_access(source, node.into_any(), key)?;

            projected.push(dir::AssignPatternFieldResolution {
                source,
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
                // decide the place a rejected field names
                self.decide_reference(place)?;
            }

            return Ok(None);
        };

        // check the field pattern against its projected value
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

    /// Project one named assignment field absent from the input type.
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

    /// Collect the member chains one object pattern's literal fields test.
    fn object_pattern_tests(
        &mut self,
        module: ModuleId,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<Vec<(Vec<dir::StaticKey>, dir::GlobalTypeId)>> {
        let mut tests = Vec::new();
        let mut prefix = Vec::new();
        self.collect_object_pattern_tests(module, flow, scope, fields, &mut prefix, &mut tests)?;

        Ok(tests)
    }

    /// Collect the tested member chains beneath one pattern field list.
    fn collect_object_pattern_tests(
        &mut self,
        module: ModuleId,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        fields: &[dir::LocalNodeId<dir::PatternField>],
        prefix: &mut Vec<dir::StaticKey>,
        tests: &mut Vec<(Vec<dir::StaticKey>, dir::GlobalTypeId)>,
    ) -> CompilerResult<()> {
        for field in fields {
            // follow named fields carrying a sub-pattern
            let dir::PatternField::Named {
                name,
                pattern: Some(pattern),
                ..
            } = self.module(module).view().get(*field).clone()
            else {
                continue;
            };

            prefix.push(name.into());
            match self.module(module).view().get(pattern).clone() {
                // singleton-valued sub-patterns test their member exactly
                dir::Pattern::Expression { value } => {
                    let site = FlowSite {
                        node: value.into_global_any(module),
                        flow,
                        scope,
                    };
                    let ty = self.infer_node_type(site, PlaceUse::Read)?;
                    if self.is_singleton_type(ty)? {
                        tests.push((prefix.clone(), ty));
                    }
                }
                // nested object patterns extend the tested chain
                dir::Pattern::Object { fields } => {
                    let fields: SmallVec<[_; 4]> = fields.iter().copied().collect();
                    self.collect_object_pattern_tests(module, flow, scope, &fields, prefix, tests)?;
                }
                // every other sub-pattern binds its member
                _ => {}
            }
            prefix.pop();
        }

        Ok(())
    }

    /// Narrow one scrutinee to the subset the pattern can match, keeping it whole when stuck.
    fn narrow_pattern_scrutinee(
        &mut self,
        origin: Origin,
        scrutinee: dir::GlobalTypeId,
        tests: &[(Vec<dir::StaticKey>, dir::GlobalTypeId)],
    ) -> CompilerResult<dir::GlobalTypeId> {
        // destructure the physical value beneath nominal wrappers
        let (mut narrowed, _) = self.project_newtype_receiver(origin, scrutinee)?;

        // filter the arms through each tested member chain
        for (keys, tested) in tests {
            narrowed = match self.narrow_arms(origin, narrowed, keys, *tested, true)? {
                // continue through the arms this test keeps
                Ok(Some(next)) => next,
                // an irreducible test leaves the arms untouched
                Ok(None) => narrowed,
                // an open arm keeps the scrutinee whole until selection re-runs
                Err(_) => narrowed,
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
        let subject = self.member_subject(origin, owner, owner, dir::MemberSpace::Instance)?;
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
        // project every runtime arm, joining several as one union projection
        if lookup.is_empty() {
            return Ok(ObjectField::Missing);
        }
        let arms = lookup.arms();
        let is_union = arms.iter().any(|group| group.arm.is_some());
        let mut projections = Vec::with_capacity(arms.len());
        let mut types = Vec::with_capacity(arms.len());
        for group in arms {
            let owner = group.arm.map_or(owner, |arm| arm.receiver);
            let projection =
                match self.object_member_field(origin, owner, key, &group.candidates)? {
                    Ok(projection) => projection,
                    // project the key on each runtime arm
                    Err(field) => return Ok(field),
                };
            types.push(projection.ty());
            projections.push(projection);
        }

        Ok(ObjectField::Projection(Box::new(
            match (is_union, projections.as_slice()) {
                (false, [_]) => dir::OperationResolution::One(projections.remove(0)),
                _ => dir::OperationResolution::Union {
                    arms: projections,
                    ty: self.normalized_union_type(types)?,
                },
            },
        )))
    }

    /// Return the lowerable projection one runtime arm's candidates expose, or the rejection.
    fn object_member_field(
        &mut self,
        origin: Origin,
        owner: dir::GlobalTypeId,
        key: dir::StaticKey,
        candidates: &[&MemberCandidate],
    ) -> CompilerResult<Result<dir::Projection, ObjectField>> {
        let field = match candidates {
            [candidate] if candidate.role == MemberRole::Field => {
                // write-only properties expose nothing to an object read
                let Some(ty) = candidate.read_type(self)? else {
                    return Ok(Err(ObjectField::Missing));
                };

                candidate.projected(owner, key, ty)
            }
            [candidate] if candidate.role == MemberRole::Getter => {
                let call = self.select_getter_call(
                    origin,
                    Value {
                        ty: owner,
                        node: None,
                        place: None,
                        is_fresh: false,
                    },
                    candidate,
                )?;

                // the owner's own getter must accept its own receiver
                let Some(call) = call else {
                    return Err(CompilerError::Internal {
                        message: format!("getter {:?} rejects its own owner", candidate.symbol()),
                    });
                };

                dir::Projection::Call(Box::new(call))
            }
            [_candidate] => {
                let key = self.format_static_key(&key);
                self.report_pattern_member_not_field(origin, owner, key)?;

                return Ok(Err(ObjectField::Rejected));
            }
            [] => return Ok(Err(ObjectField::Missing)),
            _ => {
                let key = self.format_static_key(&key);
                self.report_ambiguous_member(origin, key)?;

                return Ok(Err(ObjectField::Rejected));
            }
        };

        Ok(Ok(field))
    }

    /// Return the object rest projection after omitting selected keys.
    fn object_rest_projection(
        &mut self,
        origin: Origin,
        module: ModuleId,
        owner: dir::GlobalTypeId,
        omitted: &[dir::StaticKey],
    ) -> CompilerResult<Option<dir::ProjectionResolution>> {
        // read the fields the spread supplies
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
