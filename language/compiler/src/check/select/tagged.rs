use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, Decision, FlowPointId, FlowSite, Opening, Origin, Relation, answer,
};

/// One written tagged pattern head.
#[derive(Debug, Clone)]
pub(in crate::check) struct TaggedPatternHead {
    /// The tagged owner type.
    owner: dir::GlobalTypeId,
    /// The tagged owner instance.
    instance: dir::GenericInstance,
    /// The written case key.
    key: dir::StaticKey,
}

/// One tagged case named by a pattern.
#[derive(Debug, Clone)]
struct TaggedCaseSelection {
    /// The tagged case identity.
    case: dir::VariantCase,
    /// The matched owner generic arguments.
    generic_arguments: Vec<dir::GenericArgumentBinding>,
    /// The runtime discriminant value.
    discriminant: dir::ScalarLiteral,
    /// The compact payload type.
    payload: dir::GlobalTypeId,
    /// The compact payload fields.
    fields: Vec<TaggedPayloadField>,
}

/// One field in a compact tagged payload.
#[derive(Debug, Clone, Copy)]
struct TaggedPayloadField {
    /// The source payload key.
    key: dir::StaticKey,
    /// The projected field type.
    ty: dir::GlobalTypeId,
}

impl CheckState<'_> {
    /// Return the tagged owner and case named by one variant pattern.
    pub(in crate::check) fn tagged_pattern_head(
        &mut self,
        origin: Origin,
        module: ModuleId,
        ty: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<Answer<Option<TaggedPatternHead>>> {
        let source = ty.into_global_any(module);
        let expression = self.module(module).view().get(ty).clone();
        match expression {
            dir::TypeExpression::Member { left, name, .. } => {
                let owner = answer!(self.node_type(left.into_global_any(module))?);
                let key = dir::StaticKey::Name(name);

                self.tagged_pattern_head_from_owner(origin, owner, key)
            }
            dir::TypeExpression::Reference { path, .. } => {
                let reference = self.module(module).resolved.references.get(source).cloned();
                let Some(dir::Reference::Projected { base, from }) = reference else {
                    return self.tagged_pattern_head_from_type(origin, source);
                };
                let Some(name) = path.segments.get(from as usize).copied() else {
                    return Ok(Answer::Ready(None));
                };
                let owner = answer!(self.symbol_type(base)?);
                let key = dir::StaticKey::Name(name);

                self.tagged_pattern_head_from_owner(origin, owner, key)
            }
            _ => self.tagged_pattern_head_from_type(origin, source),
        }
    }

    /// Select one tagged variant pattern.
    pub(in crate::check) fn select_tagged_variant_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        head: TaggedPatternHead,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<Answer<()>> {
        if !self.symbol_has_tagged_derive(head.instance.symbol) {
            return self.reject_pattern(node, origin, head.owner);
        }

        // select the requested owner from the matched input when it is visible
        let written_owner = head.owner;
        let input = answer!(self.node_type(node.into_any())?);
        let heads = answer!(self.tagged_heads_from_input(origin, input, &head)?);

        // otherwise let the written generic owner bind against the input
        let heads = if heads.is_empty() {
            let belongs =
                answer!(self.constrain(origin, Relation::Assignable, input, head.owner)?);
            let variant = self.format_variant_case(head.owner, head.key);
            if !belongs {
                self.report_pattern_variant_not_in_type(origin, variant, input)?;
                self.commit_decision(node.into_any(), Decision::Rejected)?;

                return Ok(Answer::Ready(()));
            }

            vec![head]
        } else {
            heads
        };

        // select the requested case from every matched owner arm
        let Some(case) = answer!(self.tagged_case_selection_from_heads(origin, &heads)?) else {
            let key = self.format_static_key(&heads[0].key);
            self.report_pattern_variant_missing(origin, key, written_owner)?;
            self.commit_decision(node.into_any(), Decision::Rejected)?;

            return Ok(Answer::Ready(()));
        };

        // project written fields out of the compact payload
        let projected = self.project_tagged_payload_fields(
            node,
            origin,
            flow,
            scope,
            case.payload,
            &case.fields,
            fields,
        )?;

        let tag_type = self.intern_type(origin.module(), dir::Type::from(&case.discriminant))?;
        let projection = dir::Projection::VariantPayload {
            case: case.case,
            generic_arguments: case.generic_arguments,
            discriminant: case.discriminant,
            ty: case.payload,
        };
        let owner = self.tagged_selection_owner(origin.module(), &heads)?;
        let predicate = dir::Predicate::unary(
            dir::PredicateOperand::projected(dir::Projection::VariantTag { ty: tag_type }),
            dir::PredicateCondition::Literal(case.discriminant),
        )
        .with_narrowed(owner)
        .with_projection(projection.clone());

        self.commit_pattern(
            node,
            dir::PatternResolution::Destructure(dir::PatternDestructureResolution::Variant(
                dir::PatternVariantDestructureResolution {
                    predicate,
                    projection,
                    fields: projected,
                },
            )),
        )
    }

    /// Return tagged owner instances visible in the matched input.
    fn tagged_heads_from_input(
        &mut self,
        origin: Origin,
        input: dir::GlobalTypeId,
        written: &TaggedPatternHead,
    ) -> CompilerResult<Answer<Vec<TaggedPatternHead>>> {
        let input = answer!(self.reduce_type_head(origin, input)?);
        let mut heads = Vec::new();

        // collect every visible owner instance from the input
        match self.ty(input)? {
            dir::Type::Instance(instance) if instance.symbol == written.instance.symbol => {
                heads.push(TaggedPatternHead {
                    owner: input,
                    instance,
                    key: written.key,
                });
            }
            dir::Type::Union(union) => {
                let arms =
                    SmallVec::<[_; 4]>::from_slice(self.type_ids(input.module_id, union.elements)?);

                for arm in arms {
                    let arm = answer!(self.reduce_type_head(origin, arm)?);
                    if let dir::Type::Instance(instance) = self.ty(arm)?
                        && instance.symbol == written.instance.symbol
                    {
                        heads.push(TaggedPatternHead {
                            owner: arm,
                            instance,
                            key: written.key,
                        });
                    }
                }
            }
            _ => {}
        };

        Ok(Answer::Ready(heads))
    }

    /// Return one selected case from every matched tagged owner arm.
    fn tagged_case_selection_from_heads(
        &mut self,
        origin: Origin,
        heads: &[TaggedPatternHead],
    ) -> CompilerResult<Answer<Option<TaggedCaseSelection>>> {
        let mut selected = Vec::with_capacity(heads.len());
        for head in heads {
            let Some(case) = answer!(self.tagged_case_selection(origin, head)?) else {
                continue;
            };
            selected.push(case);
        }

        match selected.as_slice() {
            [] => Ok(Answer::Ready(None)),
            [case] => Ok(Answer::Ready(Some(case.clone()))),
            _ => self.merge_tagged_cases(origin, selected),
        }
    }

    /// Merge same-case selections from multiple generic owner arms.
    fn merge_tagged_cases(
        &mut self,
        origin: Origin,
        cases: Vec<TaggedCaseSelection>,
    ) -> CompilerResult<Answer<Option<TaggedCaseSelection>>> {
        let first = cases[0].clone();
        let mut payloads = Vec::with_capacity(cases.len());
        let mut fields = Vec::<TaggedPayloadField>::new();
        let mut generic_arguments = Some(first.generic_arguments.clone());

        // merge payloads and field types at their projected positions
        for case in cases {
            payloads.push(case.payload);
            if generic_arguments.as_ref() != Some(&case.generic_arguments) {
                generic_arguments = None;
            }

            for field in case.fields {
                let Some(existing) = fields.iter_mut().find(|existing| existing.key == field.key)
                else {
                    fields.push(field);
                    continue;
                };

                existing.ty =
                    self.normalized_union_type(origin.module(), [existing.ty, field.ty])?;
            }
        }

        let payload = self.normalized_union_type(origin.module(), payloads)?;
        let generic_arguments = generic_arguments.unwrap_or_default();

        Ok(Answer::Ready(Some(TaggedCaseSelection {
            case: first.case,
            generic_arguments,
            discriminant: first.discriminant,
            payload,
            fields,
        })))
    }

    /// Return the narrowed owner type selected by a tagged case.
    fn tagged_selection_owner(
        &mut self,
        module: ModuleId,
        heads: &[TaggedPatternHead],
    ) -> CompilerResult<dir::GlobalTypeId> {
        match heads {
            [head] => Ok(head.owner),
            _ => self.normalized_union_type(module, heads.iter().map(|head| head.owner)),
        }
    }

    /// Select one tagged variant construction call.
    ///
    /// Example:
    /// ```ds
    /// Bound.Included({ value: 1 })
    /// ```
    pub(in crate::check) fn select_variant_construct(
        &mut self,
        site: FlowSite,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        member: dir::EnumMemberType,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        type_arguments: &[dir::GlobalTypeId],
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<()>> {
        let module = origin.module();
        let source = self.origin_source_node(origin)?;
        let argument_sources = self.argument_value_sources(module, argument_nodes);

        // name the case from the variant symbol's declared key
        let owner_head = answer!(self.reduce_type_head(origin, member.owner)?);
        let dir::Type::Instance(owner_instance) = self.ty(owner_head)? else {
            return self.reject_construct(node, origin, arguments);
        };
        let Some(key) = self.tagged_variant_key(owner_instance.symbol, member.member) else {
            return self.reject_construct(node, origin, arguments);
        };

        // open the owner at the call site: the return expectation and
        // the payload arguments bind its holes
        let owner_static = answer!(self.symbol_type(owner_instance.symbol)?);
        let Some(head) = answer!(self.tagged_pattern_head_from_owner(origin, owner_static, key)?)
        else {
            return self.reject_construct(node, origin, arguments);
        };
        let Some(case) = answer!(self.tagged_case_selection(origin, &head)?) else {
            return self.reject_construct(node, origin, arguments);
        };

        // model the constructor: (payload) => owner
        let parameters = if case.fields.is_empty() {
            Vec::new()
        } else {
            vec![dir::FunctionParameterType {
                ty: case.payload,
                static_parameter: None,
                is_optional: false,
                is_rest: false,
            }]
        };
        let parameters = self.intern_parameters(module, &parameters)?;
        let function = dir::FunctionSignatureType {
            asynchrony: dir::Asynchrony::Sync,
            template: None,
            this_parameter: None,
            parameters,
            return_type: Some(head.owner),
            is_generator: false,
        };
        let attempt = self.attempt_signature(
            origin,
            module,
            module,
            source,
            &[],
            None,
            &[],
            type_arguments,
            &function,
            function.return_type,
            None,
            arguments,
            &argument_sources,
        )?;
        let signature = match answer!(attempt) {
            Ok(signature) => signature,
            Err(_) => return self.reject_construct(node, origin, arguments),
        };

        // commit the selected variant construction
        let target = dir::ConstructTarget::Variant(dir::VariantConstructCandidate {
            case: case.case,
            generic_arguments: case.generic_arguments,
            discriminant: case.discriminant,
        });
        let resolution = dir::ConstructResolution::new(
            target,
            Self::parameter_types(&signature.parameters),
            self.argument_bindings(module, argument_nodes, &signature.parameters),
            signature.return_type,
        );
        answer!(self.push_argument_constraints(site, argument_nodes, &resolution.arguments)?);
        self.commit_decision(node, Decision::Construct(resolution))?;
        self.commit_node_type(node, signature.return_type)?;

        Ok(Answer::Ready(()))
    }

    /// Return the tagged head named by one owner.case expression.
    ///
    /// Example:
    /// ```ds
    /// match bound { Bound.Unbounded => 0 }
    /// ```
    pub(in crate::check) fn tagged_expression_head(
        &mut self,
        origin: Origin,
        module: ModuleId,
        value: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Answer<Option<TaggedPatternHead>>> {
        let dir::Expression::Member {
            left,
            name: Some(name),
        } = self.module(module).view().get(value).clone()
        else {
            return Ok(Answer::Ready(None));
        };

        let owner = answer!(self.node_type(left.into_global_any(module))?);
        self.tagged_pattern_head_from_owner(origin, owner, dir::StaticKey::Name(name))
    }

    /// Return the tagged pattern head carried by one computed member type.
    fn tagged_pattern_head_from_type(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Answer<Option<TaggedPatternHead>>> {
        let ty = answer!(self.node_type(source)?);
        let member = match self.ty(ty)? {
            dir::Type::Member(member) => member,
            _ => return Ok(Answer::Ready(None)),
        };

        self.tagged_pattern_head_from_owner(origin, member.owner, member.key)
    }

    /// Return a tagged pattern head from an owner type and a written case key.
    fn tagged_pattern_head_from_owner(
        &mut self,
        origin: Origin,
        owner: dir::GlobalTypeId,
        key: dir::StaticKey,
    ) -> CompilerResult<Answer<Option<TaggedPatternHead>>> {
        let owner = answer!(self.reduce_type_head(origin, owner)?);
        let (owner, instance) = match self.ty(owner)? {
            dir::Type::Instance(instance) => (owner, instance),
            dir::Type::Reference(reference) => {
                // generic owners open inference holes: the matched
                // input binds them through the belongs relation
                let arguments = if let Some(template) = self.symbol_template(reference.symbol)
                    && !self.generic_template_parameters(template).is_empty()
                {
                    // the opening owns its holes across polls
                    let parameters = self.generic_template_parameters(template);
                    let source = self
                        .origin_source_node(origin)?
                        .into_global(origin.module());
                    let opening = Opening {
                        site: source,
                        parameter: parameters[0],
                    };
                    let Some(substitution) =
                        self.instantiate_at_opening(origin, opening, &parameters, &[])?
                    else {
                        return Ok(Answer::Ready(None));
                    };

                    self.intern_type_ids(origin.module(), &substitution.arguments)?
                } else {
                    dir::TypeListId::EMPTY
                };

                let instance = dir::GenericInstance {
                    symbol: reference.symbol,
                    arguments,
                };
                let owner = self.intern_type(origin.module(), dir::Type::Instance(instance))?;

                (owner, instance)
            }
            _ => return Ok(Answer::Ready(None)),
        };

        Ok(Answer::Ready(Some(TaggedPatternHead {
            owner,
            instance,
            key,
        })))
    }

    /// Select one tagged case from a newtype backing.
    fn tagged_case_selection(
        &mut self,
        origin: Origin,
        head: &TaggedPatternHead,
    ) -> CompilerResult<Answer<Option<TaggedCaseSelection>>> {
        let Some(dir::Definition::Newtype(definition)) = self.definition(head.instance.symbol)
        else {
            return Ok(Answer::Ready(None));
        };

        // reduce the owner backing under its matched arguments
        let substitution = self
            .instance_substitution(head.owner.module_id, &head.instance)?
            .with_receiver(head.owner);
        let backing = self.substitute_type(origin.module(), definition.value, &substitution)?;
        let backing = answer!(self.reduce_type_head(origin, backing)?);

        // search every union arm, or the backing itself for one-case newtypes
        let mut arms = SmallVec::<[_; 4]>::new();
        match self.ty(backing)? {
            dir::Type::Union(union) => arms.extend(
                self.type_ids(backing.module_id, union.elements)?
                    .iter()
                    .copied(),
            ),
            _ => arms.push(backing),
        }
        for arm in arms {
            let arm = answer!(self.reduce_type_head(origin, arm)?);
            let Some(case) = answer!(self.tagged_case_from_arm(origin, head, arm)?) else {
                continue;
            };

            return Ok(Answer::Ready(Some(case)));
        }

        Ok(Answer::Ready(None))
    }

    /// Select one tagged case from one reduced backing arm.
    fn tagged_case_from_arm(
        &mut self,
        origin: Origin,
        head: &TaggedPatternHead,
        arm: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<TaggedCaseSelection>>> {
        match self.ty(arm)? {
            dir::Type::Instance(instance) => {
                self.tagged_case_from_instance(origin, head, arm, &instance)
            }
            dir::Type::Shape(shape) => self.tagged_case_from_shape(origin, head, arm, &shape),
            _ => Ok(Answer::Ready(None)),
        }
    }

    /// Select one tagged case from a nominal backing arm.
    fn tagged_case_from_instance(
        &mut self,
        origin: Origin,
        head: &TaggedPatternHead,
        arm: dir::GlobalTypeId,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<Option<TaggedCaseSelection>>> {
        let Some(key) = self
            .binding_table(instance.symbol.module_id)
            .get_symbol(instance.symbol.local_id)
            .key
        else {
            return Ok(Answer::Ready(None));
        };
        if !key.matches(&head.key) {
            return Ok(Answer::Ready(None));
        }

        let tag_key = self.tagged_discriminant_key(origin.module());
        let Some(discriminant) = answer!(self.tagged_instance_discriminant(origin, arm, tag_key)?)
        else {
            return Ok(Answer::Ready(None));
        };

        let fields = answer!(self.tagged_instance_payload_fields(origin, arm, tag_key)?);
        let payload = self.tagged_payload_type(origin, &fields)?;
        let Some(case) = self.tagged_variant_case(head.instance.symbol, key) else {
            return Ok(Answer::Ready(None));
        };
        let head_arguments = self
            .type_ids(head.owner.module_id, head.instance.arguments)?
            .to_vec();
        let generic_arguments =
            self.symbol_generic_argument_bindings(head.instance.symbol, &head_arguments)?;

        Ok(Answer::Ready(Some(TaggedCaseSelection {
            case,
            generic_arguments,
            discriminant,
            payload,
            fields,
        })))
    }

    /// Select one tagged case from a structural backing arm.
    fn tagged_case_from_shape(
        &mut self,
        origin: Origin,
        head: &TaggedPatternHead,
        arm: dir::GlobalTypeId,
        shape: &dir::ShapeType,
    ) -> CompilerResult<Answer<Option<TaggedCaseSelection>>> {
        let tag_key = self.tagged_discriminant_key(origin.module());
        let shape_fields = self.shape_fields(arm.module_id, shape.fields)?.to_vec();
        let Some(tag) = shape_fields.iter().find(|field| field.key == tag_key) else {
            return Ok(Answer::Ready(None));
        };
        let tag = answer!(self.reduce_type_head(origin, tag.ty)?);
        let dir::Type::Literal(discriminant) = self.ty(tag)? else {
            return Ok(Answer::Ready(None));
        };

        let Some(key) = self.tagged_case_key_from_discriminant(origin.module(), discriminant)
        else {
            return Ok(Answer::Ready(None));
        };
        if !key.matches(&head.key) {
            return Ok(Answer::Ready(None));
        }

        let fields = shape_fields
            .iter()
            .filter(|field| field.key != tag_key)
            .map(|field| TaggedPayloadField {
                key: field.key,
                ty: field.ty,
            })
            .collect::<Vec<_>>();
        let payload = self.tagged_payload_type(origin, &fields)?;
        let Some(case) = self.tagged_variant_case(head.instance.symbol, key) else {
            return Ok(Answer::Ready(None));
        };
        let head_arguments = self
            .type_ids(head.owner.module_id, head.instance.arguments)?
            .to_vec();
        let generic_arguments =
            self.symbol_generic_argument_bindings(head.instance.symbol, &head_arguments)?;

        Ok(Answer::Ready(Some(TaggedCaseSelection {
            case,
            generic_arguments,
            discriminant,
            payload,
            fields,
        })))
    }

    /// Return the compact payload fields named by one nominal arm.
    fn tagged_instance_payload_fields(
        &mut self,
        origin: Origin,
        arm: dir::GlobalTypeId,
        tag_key: dir::StaticKey,
    ) -> CompilerResult<Answer<Vec<TaggedPayloadField>>> {
        let dir::Type::Instance(instance) = self.ty(arm)? else {
            return Ok(Answer::Ready(Vec::new()));
        };
        let Some(definition) = self.definition(instance.symbol) else {
            return Ok(Answer::Ready(Vec::new()));
        };

        let fields = definition
            .instance_fields()
            .filter(|field| field.key != tag_key)
            .cloned()
            .collect::<Vec<_>>();
        let mut payload = Vec::with_capacity(fields.len());
        for field in fields {
            let lookup = answer!(self.lookup_member(
                origin,
                origin.module(),
                arm,
                dir::MemberSpace::Instance,
                field.key,
            )?);
            let Some(ty) = lookup.value_type() else {
                continue;
            };

            payload.push(TaggedPayloadField { key: field.key, ty });
        }

        Ok(Answer::Ready(payload))
    }

    /// Return the compact payload object type for named payload fields.
    fn tagged_payload_type(
        &mut self,
        origin: Origin,
        fields: &[TaggedPayloadField],
    ) -> CompilerResult<dir::GlobalTypeId> {
        let module = origin.module();
        let fields = fields
            .iter()
            .map(|field| dir::TypeField {
                key: field.key,
                ty: field.ty,
                is_optional: false,
                is_readonly: false,
            })
            .collect::<Vec<_>>();
        let fields = self.intern_fields(module, &fields)?;
        let shape = dir::ShapeType {
            fields,
            call_signatures: dir::TypeListId::EMPTY,
            construct_signatures: dir::TypeListId::EMPTY,
            index_signatures: dir::TypeListId::EMPTY,
        };

        self.intern_type(module, dir::Type::Shape(shape))
    }

    /// Project written pattern fields from a compact tagged payload.
    fn project_tagged_payload_fields(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        payload: dir::GlobalTypeId,
        payload_fields: &[TaggedPayloadField],
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<Vec<dir::PatternFieldResolution>> {
        let module = node.module_id;
        let mut projected = Vec::with_capacity(fields.len());
        let mut position = 0usize;
        for field in fields {
            let source = field.into_global_any(module);
            let field_node = self.module(module).view().get(*field).clone();
            match field_node {
                dir::PatternField::Positional { pattern } => {
                    let pattern_type = self.tagged_positional_payload_type(
                        module,
                        payload,
                        payload_fields,
                        pattern,
                        position,
                    );
                    let Some(pattern_type) = pattern_type else {
                        let key = position.to_string();
                        self.report_pattern_field_missing(origin, payload, key)?;
                        position += 1;

                        continue;
                    };

                    self.project_pattern_input(
                        flow,
                        scope,
                        pattern_type,
                        pattern.into_global_any(module),
                    )?;
                    projected.push(dir::PatternFieldResolution {
                        source,
                        projection: dir::Projection::FieldGet {
                            field: dir::ProjectionField::Key(dir::StaticKey::Index(position)),
                            ty: pattern_type,
                        },
                        pattern: Some(pattern.into_global_any(module)),
                    });
                    position += 1;
                }
                dir::PatternField::Named { name, pattern, .. } => {
                    let key = name.static_key();
                    let Some(payload_field) = payload_fields.iter().find(|field| field.key == key)
                    else {
                        let key = self.format_static_key(&key);
                        self.report_pattern_field_missing(origin, payload, key)?;

                        continue;
                    };

                    if let Some(pattern) = pattern {
                        self.project_pattern_input(
                            flow,
                            scope,
                            payload_field.ty,
                            pattern.into_global_any(module),
                        )?;
                    } else if let Some(symbol) =
                        self.module(module).declaration_symbol((*field).into_any())
                    {
                        let input = self.pattern_binding_type(symbol, payload_field.ty)?;

                        self.bind_symbol_type(symbol, input)?;
                    }

                    projected.push(dir::PatternFieldResolution {
                        source,
                        projection: dir::Projection::FieldGet {
                            field: dir::ProjectionField::Key(key),
                            ty: payload_field.ty,
                        },
                        pattern: pattern.map(|pattern| pattern.into_global_any(module)),
                    });
                }
                dir::PatternField::Elision => {
                    position += 1;
                }
                dir::PatternField::Computed { .. } | dir::PatternField::Rest { .. } => {
                    self.report_pattern_source_not_object_shaped(origin, payload)?;
                }
            }
        }

        Ok(projected)
    }

    /// Return one positional compact payload type.
    fn tagged_positional_payload_type(
        &self,
        module: ModuleId,
        payload: dir::GlobalTypeId,
        fields: &[TaggedPayloadField],
        pattern: dir::LocalNodeId<dir::Pattern>,
        position: usize,
    ) -> Option<dir::GlobalTypeId> {
        let pattern = self.module(module).view().get(pattern);
        if matches!(pattern, dir::Pattern::Object { .. }) {
            return Some(payload);
        }

        fields.get(position).map(|field| field.ty)
    }
}
