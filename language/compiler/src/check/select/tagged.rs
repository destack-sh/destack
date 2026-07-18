use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    Answer, BodyState, CandidateOutcome, CandidatePass, Cause, CauseKind, CheckState, Decision,
    FlowPointId, FlowSite, Origin, Relation, TypeSubstitution, answer,
};
use crate::{CompilerError, CompilerResult};

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
    /// The matched owner arguments, absent when merged arms mix instantiations.
    generic_arguments: Option<Vec<dir::GenericArgumentBinding>>,
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
    /// Whether the source field is optional.
    is_optional: bool,
    /// Whether the source field is readonly.
    is_readonly: bool,
}

impl BodyState<'_, '_> {
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
                    // bound references name declarations, never tagged cases
                    return Ok(Answer::Ready(None));
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
        let definition = self.definition(head.instance.symbol)?;
        match definition {
            // enum owners select their member by discriminant
            Some(dir::Definition::Enum(_)) => {
                return self.select_enum_member_pattern(
                    node,
                    origin,
                    head.owner,
                    &head.instance,
                    head.key,
                    fields,
                );
            }

            // derived newtypes carry their checked tagged definition
            Some(dir::Definition::Newtype(value)) if value.is_tagged() => {}

            // every other owner has no variant cases
            _ => return self.reject_pattern(node, origin, head.owner),
        }

        // select the requested owner from the matched input when it is visible
        let written_owner = head.owner;
        let input = answer!(self.node_type(node.into_any())?);
        let heads = answer!(self.tagged_heads_from_input(origin, input, &head)?);
        // otherwise let the written generic owner bind against the input
        let heads = if heads.is_empty() {
            let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
            let belongs =
                answer!(self.constrain_type(cause, Relation::Assignable, input, head.owner)?);
            let variant = self.format_variant_case(head.owner, head.key);
            if !belongs {
                self.report_pattern_variant_not_in_type(origin, variant, input)?;

                return self.commit_rejected_pattern(node);
            }

            vec![head]
        } else {
            heads
        };

        // select the requested case from every matched owner arm
        let Some(case) = answer!(self.tagged_case_selection_from_heads(origin, &heads)?) else {
            let key = self.format_static_key(&heads[0].key);
            self.report_pattern_variant_missing(origin, key, written_owner)?;

            return self.commit_rejected_pattern(node);
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
        let input = self.value_beneath_forms(origin, input)?;
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
        let mut generic_arguments = first.generic_arguments.clone();

        // merge payloads and field types at their projected positions
        for case in cases {
            payloads.push(case.payload);

            // mixed owner instantiations have no single argument list
            if generic_arguments != case.generic_arguments {
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
        expected_return: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<()>> {
        let module = origin.module();
        let source = self.origin_source_node(origin)?;
        let arguments = self.callable_arguments(module, argument_nodes)?;

        // name the case from the variant symbol's declared key
        let owner_head = answer!(self.reduce_type_head(origin, member.owner)?);
        let dir::Type::Instance(owner_instance) = self.ty(owner_head)? else {
            return self.reject_construct(site, node, origin, argument_nodes, &[]);
        };
        let Some(key) = self.tagged_variant_key(owner_instance.symbol, member.member)? else {
            return self.reject_construct(site, node, origin, argument_nodes, &[]);
        };

        // open the owner at the call site: the return expectation and
        //  the payload arguments bind its holes
        let owner_static = answer!(self.symbol_type(owner_instance.symbol)?);
        let Some(head) = answer!(self.tagged_pattern_head_from_owner(origin, owner_static, key)?)
        else {
            return self.reject_construct(site, node, origin, argument_nodes, &[]);
        };
        let Some(case) = answer!(self.tagged_case_selection(origin, &head)?) else {
            return self.reject_construct(site, node, origin, argument_nodes, &[]);
        };

        // explicit type arguments bind the opened owner holes directly
        let mut type_arguments = type_arguments;
        if !type_arguments.is_empty() {
            let dir::Type::Instance(owner_open) = self.ty(head.owner)? else {
                return self.reject_construct(site, node, origin, argument_nodes, &[]);
            };
            let holes = self
                .type_ids(head.owner.module_id, owner_open.arguments)?
                .to_vec();
            if holes.len() != type_arguments.len() {
                return self.reject_construct(site, node, origin, argument_nodes, &[]);
            }
            for (hole, argument) in holes.iter().zip(type_arguments) {
                if let Some(variable) = self.check.root_variable(*hole)? {
                    self.check.commit_solution(variable, *argument)?;
                }
            }
            type_arguments = &[];
        }

        // model the constructor: (payload) => owner
        let parameters = if case.fields.is_empty() {
            Vec::new()
        } else {
            vec![dir::FunctionParameterType {
                ty: case.payload,
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
        let attempt = self.match_signature(
            CandidatePass::Confirm,
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
            &arguments,
            expected_return,
        )?;
        let signature = match answer!(attempt) {
            CandidateOutcome::Accepted(signature) => signature,
            CandidateOutcome::Rejected(_) => {
                return self.reject_construct(site, node, origin, argument_nodes, &[]);
            }
        };

        // construction selects through one written owner instantiation
        let Some(generic_arguments) = case.generic_arguments else {
            return Err(CompilerError::Internal {
                message: format!("variant construction {node:?} merged mixed instantiations"),
            });
        };

        // commit the selected variant construction
        let target = dir::ConstructTarget::Variant(dir::VariantConstructCandidate {
            case: case.case,
            generic_arguments,
            discriminant: case.discriminant,
        });
        let resolution = dir::ConstructResolution::new(
            target,
            self.argument_bindings(module, argument_nodes, &signature.parameters),
            signature.return_type,
        );
        self.check_arguments(site, argument_nodes, &resolution.arguments)?;
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
            ..
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
        let Some(member) = self.member_head(ty)? else {
            return Ok(Answer::Ready(None));
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
                // generic owners open inference holes
                let arguments = if let Some(template) = self.symbol_template(reference.symbol)?
                    && !self.generic_template_parameters(template).is_empty()
                {
                    let parameters = self.generic_template_parameters(template);
                    let Some(substitution) = self.instantiate_parameter_arguments(
                        origin,
                        &parameters,
                        &[],
                        TypeSubstitution::default(),
                    )?
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

    /// Select one tagged variant.
    fn tagged_case_selection(
        &mut self,
        origin: Origin,
        head: &TaggedPatternHead,
    ) -> CompilerResult<Answer<Option<TaggedCaseSelection>>> {
        let Some(variant) = self.tagged_variant(head.instance.symbol, head.key)? else {
            return Ok(Answer::Ready(None));
        };
        // instantiate the backing leaf under the selected owner
        let substitution = self
            .instance_substitution(head.owner.module_id, &head.instance)?
            .with_receiver(head.owner);
        let leaf = self.substitute_type(origin.module(), variant.backing, &substitution)?;
        let leaf = answer!(self.reduce_type_head(origin, leaf)?);

        // project the runtime payload from the selected leaf
        let definition =
            self.definition(head.instance.symbol)?
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("tagged owner {:?} has no definition", head.instance.symbol),
                })?;
        let dir::Definition::Newtype(definition) = definition else {
            return Err(CompilerError::Internal {
                message: format!("tagged owner {:?} is not a newtype", head.instance.symbol),
            });
        };
        let discriminant = definition
            .discriminant
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "tagged owner {:?} has no discriminant",
                    head.instance.symbol
                ),
            })?;
        let discriminant_key = dir::StaticKey::Name(discriminant);
        let fields = answer!(self.tagged_payload_fields(origin, leaf, discriminant_key)?);
        let payload = self.tagged_payload_type(origin, &fields)?;
        let discriminant = dir::ScalarLiteral::String(variant.discriminant);
        let case = dir::VariantCase {
            owner: head.instance.symbol,
            key: variant.key,
            member: variant.symbol,
        };
        let head_arguments = self
            .type_ids(head.owner.module_id, head.instance.arguments)?
            .to_vec();
        let generic_arguments =
            self.symbol_generic_argument_bindings(head.instance.symbol, &head_arguments)?;

        Ok(Answer::Ready(Some(TaggedCaseSelection {
            case,
            generic_arguments: Some(generic_arguments),
            discriminant,
            payload,
            fields,
        })))
    }

    /// Return the compact payload fields declared by one tagged leaf.
    fn tagged_payload_fields(
        &mut self,
        origin: Origin,
        leaf: dir::GlobalTypeId,
        discriminant: dir::StaticKey,
    ) -> CompilerResult<Answer<Vec<TaggedPayloadField>>> {
        match self.ty(leaf)? {
            dir::Type::Instance(_) => {
                self.tagged_instance_payload_fields(origin, leaf, discriminant)
            }
            dir::Type::Shape(shape) => {
                let fields = self
                    .shape_fields(leaf.module_id, shape.fields)?
                    .iter()
                    .filter(|field| field.key != discriminant)
                    .map(|field| TaggedPayloadField {
                        key: field.key,
                        ty: field.ty,
                        is_optional: field.is_optional,
                        is_readonly: field.is_readonly,
                    })
                    .collect();

                Ok(Answer::Ready(fields))
            }
            _ => Err(CompilerError::Internal {
                message: format!("tagged variant has invalid leaf {leaf:?}"),
            }),
        }
    }

    /// Return the compact payload fields named by one nominal arm.
    fn tagged_instance_payload_fields(
        &mut self,
        origin: Origin,
        arm: dir::GlobalTypeId,
        tag_key: dir::StaticKey,
    ) -> CompilerResult<Answer<Vec<TaggedPayloadField>>> {
        let dir::Type::Instance(instance) = self.ty(arm)? else {
            return Err(CompilerError::Internal {
                message: format!("tagged nominal leaf {arm:?} is not an instance"),
            });
        };
        let Some(definition) = self.definition(instance.symbol)? else {
            return Err(CompilerError::Internal {
                message: format!("tagged nominal leaf {arm:?} has no definition"),
            });
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
                return Err(CompilerError::Internal {
                    message: format!("tagged payload field {:?} has no value type", field.symbol),
                });
            };

            payload.push(TaggedPayloadField {
                key: field.key,
                ty,
                is_optional: field.is_optional,
                is_readonly: field.is_readonly,
            });
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
                is_optional: field.is_optional,
                is_readonly: field.is_readonly,
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
                    )?;
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
                    let input = self.tagged_payload_field_type(module, payload_field)?;

                    if let Some(pattern) = pattern {
                        self.project_pattern_input(
                            flow,
                            scope,
                            input,
                            pattern.into_global_any(module),
                        )?;
                    } else if let Some(symbol) =
                        self.module(module).declaration_symbol((*field).into_any())
                    {
                        let input = self.pattern_binding_type(symbol, input)?;

                        self.bind_symbol_type(symbol, input)?;
                    }

                    projected.push(dir::PatternFieldResolution {
                        source,
                        projection: dir::Projection::FieldGet {
                            field: dir::ProjectionField::Key(key),
                            ty: input,
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
        &mut self,
        module: ModuleId,
        payload: dir::GlobalTypeId,
        fields: &[TaggedPayloadField],
        pattern: dir::LocalNodeId<dir::Pattern>,
        position: usize,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let pattern = self.module(module).view().get(pattern);
        if matches!(pattern, dir::Pattern::Object { .. }) {
            return Ok(Some(payload));
        }

        let ty = match fields.get(position) {
            Some(field) => Some(self.tagged_payload_field_type(module, field)?),
            None => None,
        };

        Ok(ty)
    }

    /// Return one tagged payload field's projected value type.
    fn tagged_payload_field_type(
        &mut self,
        module: ModuleId,
        field: &TaggedPayloadField,
    ) -> CompilerResult<dir::GlobalTypeId> {
        if !field.is_optional {
            return Ok(field.ty);
        }
        let undefined = self.intern_type(module, dir::Type::Undefined)?;

        self.normalized_union_type(module, [field.ty, undefined])
    }
}

impl CheckState<'_> {
    /// Return the case key declared by one Tagged variant symbol.
    pub(in crate::check) fn tagged_variant_key(
        &mut self,
        owner: dir::GlobalSymbolId,
        member: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::StaticKey>> {
        let Some(dir::Definition::Newtype(definition)) = self.definition(owner)? else {
            return Ok(None);
        };
        let key = definition
            .tagged_variant_by_symbol(member)
            .map(|variant| variant.key);

        Ok(key)
    }

    /// Return the variant declared by one Tagged owner and case key.
    pub(in crate::check) fn tagged_variant(
        &mut self,
        owner: dir::GlobalSymbolId,
        key: dir::StaticKey,
    ) -> CompilerResult<Option<dir::TaggedVariantDefinition>> {
        let Some(dir::Definition::Newtype(definition)) = self.definition(owner)? else {
            return Ok(None);
        };
        let variant = definition.tagged_variant_by_key(key).cloned();

        Ok(variant)
    }

    /// Return the selected case key for one Tagged discriminant.
    pub(in crate::check) fn tagged_case_key_from_discriminant(
        &self,
        owner: dir::GlobalSymbolId,
        discriminant: dir::ScalarLiteral,
    ) -> Option<dir::StaticKey> {
        let dir::ScalarLiteral::String(discriminant) = discriminant else {
            return None;
        };
        let Some(dir::Definition::Newtype(definition)) = self.loaded_definition(owner) else {
            return None;
        };

        definition
            .tagged_variant_by_discriminant(discriminant)
            .map(|variant| variant.key)
    }

    /// Return the selected case key for one Tagged owner type and discriminant.
    pub(in crate::check) fn tagged_case_key_from_type(
        &mut self,
        origin: Origin,
        owner: dir::GlobalTypeId,
        discriminant: dir::ScalarLiteral,
    ) -> CompilerResult<Answer<Option<dir::StaticKey>>> {
        let owner = answer!(self.reduce_type_head(origin, owner)?);
        let Some(instance) = self.tagged_domain_instance(owner)? else {
            return Ok(Answer::Ready(None));
        };
        let key = self.tagged_case_key_from_discriminant(instance.symbol, discriminant);

        Ok(Answer::Ready(key))
    }

    /// Return the finite discriminant domain of one Tagged newtype value.
    pub(in crate::check) fn tagged_discriminant_domain(
        &mut self,
        origin: Origin,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<Vec<dir::ScalarLiteral>>>> {
        let value = answer!(self.reduce_type_head(origin, value)?);
        let Some(instance) = self.tagged_domain_instance(value)? else {
            return Ok(Answer::Ready(None));
        };
        let Some(dir::Definition::Newtype(definition)) = self.definition(instance.symbol)? else {
            return Ok(Answer::Ready(None));
        };
        if !definition.is_tagged() {
            return Ok(Answer::Ready(None));
        }
        let domain = definition
            .tagged_variants()
            .map(|variant| dir::ScalarLiteral::String(variant.discriminant))
            .collect();

        Ok(Answer::Ready(Some(domain)))
    }

    /// Return the instance represented by one Tagged domain owner type.
    fn tagged_domain_instance(
        &mut self,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GenericInstance>> {
        let instance = match self.ty(value)? {
            dir::Type::Instance(instance) => instance,
            dir::Type::Reference(reference) => {
                if let Some(template) = self.symbol_template(reference.symbol)?
                    && !self.generic_template_parameters(template).is_empty()
                {
                    return Ok(None);
                }

                dir::GenericInstance {
                    symbol: reference.symbol,
                    arguments: dir::TypeListId::EMPTY,
                }
            }
            _ => return Ok(None),
        };

        Ok(Some(instance))
    }
}
