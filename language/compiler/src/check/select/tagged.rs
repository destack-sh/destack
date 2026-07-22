use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    Answer, BodyState, CheckState, Decision, FlowPointId, FlowSite, Origin, SignatureMatch,
    TypeSubstitution, answer,
};
use crate::{CompilerError, CompilerResult};

/// One instantiated tagged owner.
#[derive(Debug, Clone)]
pub(in crate::check) struct VariantOwner {
    /// The variant owner type.
    pub(in crate::check) owner: dir::GlobalTypeId,
    /// The variant owner instance.
    pub(in crate::check) instance: dir::GenericApplication,
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
    /// Select one written variant pattern when its owner is a variant family.
    pub(in crate::check) fn select_variant_type_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        module: ModuleId,
        ty: dir::LocalNodeId<dir::TypeExpression>,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<Answer<bool>> {
        let Some((owner, symbol, key)) = answer!(self.variant_pattern_owner(origin, module, ty)?)
        else {
            return Ok(Answer::Ready(false));
        };
        let Some(case) = self.variant_case(symbol, key)? else {
            let key = self.format_static_key(&key);
            self.report_pattern_variant_missing(origin, key, owner)?;
            answer!(self.commit_rejected_pattern(node)?);

            return Ok(Answer::Ready(true));
        };
        answer!(self.select_variant_pattern(node, origin, flow, scope, case, fields)?);

        Ok(Answer::Ready(true))
    }

    /// Return the variant family and key named by one pattern.
    fn variant_pattern_owner(
        &mut self,
        origin: Origin,
        module: ModuleId,
        ty: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<Answer<Option<(dir::GlobalTypeId, dir::GlobalSymbolId, dir::StaticKey)>>>
    {
        let source = ty.into_global_any(module);
        let expression = self.module(module).view().get(ty).clone();
        match expression {
            dir::TypeExpression::Member { left, name, .. } => {
                let owner = answer!(self.node_type(left.into_global_any(module))?);
                let key = dir::StaticKey::Name(name);

                self.variant_family_from_type(origin, owner, key)
            }
            dir::TypeExpression::Reference { path, .. } => {
                let reference = self.module(module).resolved.references.get(source).cloned();
                let Some(dir::Reference::Projected { base, from }) = reference else {
                    // bound references name declarations, never variant cases
                    return Ok(Answer::Ready(None));
                };
                let Some(name) = path.segments.get(from as usize).copied() else {
                    return Ok(Answer::Ready(None));
                };
                let owner = answer!(self.symbol_type(base)?);
                let key = dir::StaticKey::Name(name);

                self.variant_family_from_type(origin, owner, key)
            }
            _ => {
                let ty = answer!(self.node_type(source)?);
                let Some(member) = self.member_head(ty)? else {
                    return Ok(Answer::Ready(None));
                };

                self.variant_family_from_type(origin, member.owner, member.key)
            }
        }
    }

    /// Return one variant family from its owner type and selected key.
    fn variant_family_from_type(
        &mut self,
        origin: Origin,
        owner: dir::GlobalTypeId,
        key: dir::StaticKey,
    ) -> CompilerResult<Answer<Option<(dir::GlobalTypeId, dir::GlobalSymbolId, dir::StaticKey)>>>
    {
        let owner = answer!(self.reduce_type_head(origin, owner)?);
        let symbol = match self.ty(owner)? {
            dir::Type::Application(instance) => instance.symbol,
            dir::Type::Reference(reference) => reference.symbol,
            _ => return Ok(Answer::Ready(None)),
        };
        let is_variant_family = match self.definition(symbol)? {
            Some(dir::Definition::Enum(_)) => true,
            Some(dir::Definition::Newtype(definition)) => definition.is_tagged(),
            _ => false,
        };
        if is_variant_family {
            Ok(Answer::Ready(Some((owner, symbol, key))))
        } else {
            Ok(Answer::Ready(None))
        }
    }

    /// Return the variant case named by one expression pattern.
    pub(in crate::check) fn variant_expression_case(
        &mut self,
        origin: Origin,
        module: ModuleId,
        value: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Answer<Option<dir::VariantCase>>> {
        let dir::Expression::Member {
            left,
            name: Some(name),
            ..
        } = self.module(module).view().get(value).clone()
        else {
            return Ok(Answer::Ready(None));
        };

        let owner = answer!(self.node_type(left.into_global_any(module))?);
        self.variant_case_from_owner(origin, owner, dir::StaticKey::Name(name))
    }

    /// Return one variant case from its owner type and key.
    fn variant_case_from_owner(
        &mut self,
        origin: Origin,
        owner: dir::GlobalTypeId,
        key: dir::StaticKey,
    ) -> CompilerResult<Answer<Option<dir::VariantCase>>> {
        let owner = answer!(self.reduce_type_head(origin, owner)?);
        let symbol = match self.ty(owner)? {
            dir::Type::Application(instance) => instance.symbol,
            dir::Type::Reference(reference) => reference.symbol,
            _ => return Ok(Answer::Ready(None)),
        };
        let case = self.variant_case(symbol, key)?;

        Ok(Answer::Ready(case))
    }

    /// Return one variant case from its declaration symbol and key.
    fn variant_case(
        &mut self,
        owner: dir::GlobalSymbolId,
        key: dir::StaticKey,
    ) -> CompilerResult<Option<dir::VariantCase>> {
        let member = match self.definition(owner)? {
            Some(dir::Definition::Enum(definition)) => {
                definition.variant_by_key(key).map(|variant| variant.symbol)
            }
            Some(dir::Definition::Newtype(definition)) if definition.is_tagged() => definition
                .tagged_variant_by_key(key)
                .map(|variant| variant.symbol),
            _ => None,
        };
        let case = member.map(|member| dir::VariantCase { owner, key, member });

        Ok(case)
    }

    /// Select one tagged variant pattern.
    pub(in crate::check) fn select_variant_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        case: dir::VariantCase,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<Answer<()>> {
        let input = answer!(self.node_type(node.into_any())?);
        let owners = answer!(self.variant_owners_from_input(origin, input, case.owner)?);
        if owners.is_empty() {
            let owner = self.format_symbol(case.owner);
            let key = self.format_static_key(&case.key);
            let variant = format!("{owner}.{key}");
            self.report_pattern_variant_not_in_type(origin, variant, input)?;

            return self.commit_rejected_pattern(node);
        }

        let definition = self.definition(case.owner)?;
        match definition {
            // enum owners select their member by discriminant
            Some(dir::Definition::Enum(_)) => {
                return self.select_enum_member_pattern(node, origin, case, &owners, fields);
            }

            // derived newtypes carry their checked tagged definition
            Some(dir::Definition::Newtype(value)) if value.is_tagged() => {}

            // every other owner has no variant cases
            _ => return self.reject_pattern(node, origin, owners[0].owner),
        }

        // select the requested case from every matched owner arm
        let Some(selection) =
            answer!(self.tagged_case_selection_from_owners(origin, &case, &owners)?)
        else {
            let key = self.format_static_key(&case.key);
            self.report_pattern_variant_missing(origin, key, owners[0].owner)?;

            return self.commit_rejected_pattern(node);
        };

        // project written fields out of the compact payload
        let projected = answer!(self.project_tagged_payload_fields(
            node,
            origin,
            flow,
            scope,
            selection.payload,
            &selection.fields,
            fields,
        )?);

        let tag_type =
            self.intern_type(origin.module(), dir::Type::from(&selection.discriminant))?;
        let projection = dir::Projection::VariantPayload {
            case: selection.case,
            generic_arguments: selection.generic_arguments,
            discriminant: selection.discriminant,
            ty: selection.payload,
        };
        let owner = self.tagged_selection_owner(origin.module(), &owners)?;
        let predicate = dir::Predicate::unary(
            dir::PredicateOperand::projected(dir::Projection::VariantTag { ty: tag_type }),
            dir::PredicateCondition::Literal(selection.discriminant),
        )
        .with_narrowed(owner)
        .with_projection(projection.clone());

        self.commit_pattern(
            node,
            dir::PatternResolution::Destructure(Box::new(
                dir::PatternDestructureResolution::Variant(Box::new(
                    dir::PatternVariantDestructureResolution {
                        predicate,
                        projection,
                        fields: projected,
                    },
                )),
            )),
        )
    }

    /// Return tagged owner instances visible in the matched input.
    fn variant_owners_from_input(
        &mut self,
        origin: Origin,
        input: dir::GlobalTypeId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Answer<Vec<VariantOwner>>> {
        let input = answer!(self.strip_form(origin, input)?);
        let mut owners = Vec::new();

        // collect every visible owner instance from the input
        match self.ty(input)? {
            dir::Type::Application(instance) if instance.symbol == symbol => {
                owners.push(VariantOwner {
                    owner: input,
                    instance,
                });
            }
            dir::Type::Union(union) => {
                let arms =
                    SmallVec::<[_; 4]>::from_slice(self.type_ids(input.module_id, union.elements)?);

                for arm in arms {
                    let arm = answer!(self.reduce_type_head(origin, arm)?);
                    if let dir::Type::Application(instance) = self.ty(arm)?
                        && instance.symbol == symbol
                    {
                        owners.push(VariantOwner {
                            owner: arm,
                            instance,
                        });
                    }
                }
            }
            _ => {}
        };

        Ok(Answer::Ready(owners))
    }

    /// Return one selected case from every matched tagged owner arm.
    fn tagged_case_selection_from_owners(
        &mut self,
        origin: Origin,
        case: &dir::VariantCase,
        owners: &[VariantOwner],
    ) -> CompilerResult<Answer<Option<TaggedCaseSelection>>> {
        let mut selected = Vec::with_capacity(owners.len());
        for owner in owners {
            let Some(case) = answer!(self.tagged_case_selection(origin, owner, case)?) else {
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
        owners: &[VariantOwner],
    ) -> CompilerResult<dir::GlobalTypeId> {
        match owners {
            [owner] => Ok(owner.owner),
            _ => self.normalized_union_type(module, owners.iter().map(|owner| owner.owner)),
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
        let dir::Type::Application(owner_instance) = self.ty(owner_head)? else {
            return self.reject_construct(site, node, origin, argument_nodes, &[]);
        };
        let Some(key) = self.tagged_variant_key(owner_instance.symbol, member.member)? else {
            return self.reject_construct(site, node, origin, argument_nodes, &[]);
        };

        // open the owner at the call site: the return expectation and
        //  the payload arguments bind its holes
        let owner_static = answer!(self.symbol_type(owner_instance.symbol)?);
        let Some(owner) = answer!(self.tagged_owner_from_type(origin, owner_static)?) else {
            return self.reject_construct(site, node, origin, argument_nodes, &[]);
        };
        let Some(variant) = self.variant_case(owner.instance.symbol, key)? else {
            return self.reject_construct(site, node, origin, argument_nodes, &[]);
        };
        let Some(case) = answer!(self.tagged_case_selection(origin, &owner, &variant)?) else {
            return self.reject_construct(site, node, origin, argument_nodes, &[]);
        };

        // explicit type arguments bind the opened owner holes directly
        let mut type_arguments = type_arguments;
        if !type_arguments.is_empty() {
            let dir::Type::Application(owner_open) = self.ty(owner.owner)? else {
                return self.reject_construct(site, node, origin, argument_nodes, &[]);
            };
            let holes = self
                .type_ids(owner.owner.module_id, owner_open.arguments)?
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
            return_type: Some(owner.owner),
            is_generator: false,
        };
        let attempt = self.match_signature(
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
        let (signature, rejection) = match answer!(attempt) {
            SignatureMatch::Selected(signature) | SignatureMatch::ReturnMismatch(signature) => {
                (signature, None)
            }
            SignatureMatch::Invalid {
                selection,
                rejection,
                variables,
            } => (selection, Some((rejection, variables))),
            SignatureMatch::Inapplicable(_) => {
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
        self.commit_decision(node, Decision::Construct(resolution))?;
        self.commit_node_type(node, signature.return_type)?;
        if let Some((rejection, variables)) = rejection {
            self.report_signature_rejection(origin, module, argument_nodes, rejection)?;
            self.check.poison_scope(variables)?;
        }

        Ok(Answer::Ready(()))
    }

    /// Instantiate one tagged owner type.
    fn tagged_owner_from_type(
        &mut self,
        origin: Origin,
        owner: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<VariantOwner>>> {
        let owner = answer!(self.reduce_type_head(origin, owner)?);
        let (owner, instance) = match self.ty(owner)? {
            dir::Type::Application(instance) => (owner, instance),
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

                let instance = dir::GenericApplication {
                    symbol: reference.symbol,
                    arguments,
                };
                let owner = self.intern_type(origin.module(), dir::Type::Application(instance))?;

                (owner, instance)
            }
            _ => return Ok(Answer::Ready(None)),
        };

        Ok(Answer::Ready(Some(VariantOwner { owner, instance })))
    }

    /// Select one tagged variant.
    fn tagged_case_selection(
        &mut self,
        origin: Origin,
        owner: &VariantOwner,
        case: &dir::VariantCase,
    ) -> CompilerResult<Answer<Option<TaggedCaseSelection>>> {
        let Some(variant) = self.tagged_variant(owner.instance.symbol, case.key)? else {
            return Ok(Answer::Ready(None));
        };
        // instantiate the backing leaf under the selected owner
        let substitution = self
            .instance_substitution(owner.owner.module_id, &owner.instance)?
            .with_receiver(owner.owner);
        let leaf = self.substitute_type(origin.module(), variant.backing, &substitution)?;
        let leaf = answer!(self.reduce_type_head(origin, leaf)?);

        // project the runtime payload from the selected leaf
        let definition =
            self.definition(owner.instance.symbol)?
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("tagged owner {:?} has no definition", owner.instance.symbol),
                })?;
        let dir::Definition::Newtype(definition) = definition else {
            return Err(CompilerError::Internal {
                message: format!("tagged owner {:?} is not a newtype", owner.instance.symbol),
            });
        };
        let discriminant = definition
            .discriminant
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "tagged owner {:?} has no discriminant",
                    owner.instance.symbol
                ),
            })?;
        let discriminant_key = dir::StaticKey::Name(discriminant);
        let fields = answer!(self.tagged_payload_fields(origin, leaf, discriminant_key)?);
        let payload = self.tagged_payload_type(origin, &fields)?;
        let discriminant = dir::ScalarLiteral::String(variant.discriminant);
        let owner_arguments = self
            .type_ids(owner.owner.module_id, owner.instance.arguments)?
            .to_vec();
        let generic_arguments =
            self.symbol_generic_argument_bindings(owner.instance.symbol, &owner_arguments)?;

        Ok(Answer::Ready(Some(TaggedCaseSelection {
            case: case.clone(),
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
            dir::Type::Application(_) => {
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
        let dir::Type::Application(instance) = self.ty(arm)? else {
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
    ) -> CompilerResult<Answer<Vec<dir::PatternFieldResolution>>> {
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

                    answer!(self.check_pattern_projection(
                        flow,
                        scope,
                        pattern_type,
                        pattern.into_global_any(module),
                    )?);
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
                        answer!(self.check_pattern_projection(
                            flow,
                            scope,
                            input,
                            pattern.into_global_any(module),
                        )?);
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

        Ok(Answer::Ready(projected))
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
    ) -> CompilerResult<Option<dir::GenericApplication>> {
        let instance = match self.ty(value)? {
            dir::Type::Application(instance) => instance,
            dir::Type::Reference(reference) => {
                if let Some(template) = self.symbol_template(reference.symbol)?
                    && !self.generic_template_parameters(template).is_empty()
                {
                    return Ok(None);
                }

                dir::GenericApplication {
                    symbol: reference.symbol,
                    arguments: dir::TypeListId::EMPTY,
                }
            }
            _ => return Ok(None),
        };

        Ok(Some(instance))
    }
}
