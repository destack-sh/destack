use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{BodyState, CheckState, FlowPointId, Origin};
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
    /// The selected discriminator field.
    discriminator: dir::StaticKey,
    /// The runtime discriminant value.
    discriminant: dir::ScalarLiteral,
    /// The selected case payload.
    payload: Option<TaggedPayload>,
}

/// One instantiated Tagged case payload.
#[derive(Debug, Clone)]
struct TaggedPayload {
    /// The exact case backing projected at runtime.
    backing: dir::GlobalTypeId,
    /// The constructor argument fields.
    fields: Vec<dir::TypeProperty>,
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
    ) -> CompilerResult<bool> {
        let Some((owner, symbol, key)) = self.variant_pattern_owner(module, ty)? else {
            return Ok(false);
        };
        let Some(case) = self.variant_case(symbol, key)? else {
            let key = self.format_static_key(&key);
            self.report_pattern_variant_missing(origin, key, owner)?;
            self.commit_rejected_pattern(node)?;

            return Ok(true);
        };
        self.select_variant_pattern(node, origin, flow, scope, case, fields)?;

        Ok(true)
    }

    /// Return the variant family and key named by one pattern.
    fn variant_pattern_owner(
        &mut self,
        module: ModuleId,
        ty: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<Option<(dir::GlobalTypeId, dir::GlobalSymbolId, dir::StaticKey)>> {
        let source = ty.into_global_any(module);
        let expression = self.module(module).view().get(ty).clone();
        match expression {
            dir::TypeExpression::Member { left, name, .. } => {
                let owner = self.require_node_type(left.into_global_any(module))?;
                let key = dir::StaticKey::Name(name);

                self.variant_family(owner, key)
            }
            dir::TypeExpression::Reference { path, .. } => {
                let reference = self.module(module).resolved.references.get(source).cloned();
                let Some(dir::Reference::Projected {
                    base: dir::ReferenceTarget::Symbol(base),
                    from,
                }) = reference
                else {
                    // bound references name declarations, never variant cases
                    return Ok(None);
                };
                let Some(name) = path.segments.get(from as usize).copied() else {
                    return Ok(None);
                };
                let owner = self.symbol_type(base)?;
                let key = dir::StaticKey::Name(name);

                self.variant_family(owner, key)
            }
            _ => {
                let ty = self.require_node_type(source)?;
                let Some(member) = self.member_head(ty)? else {
                    return Ok(None);
                };

                self.variant_family(member.owner, member.key)
            }
        }
    }

    /// Return one variant family from its owner type and selected key.
    fn variant_family(
        &mut self,
        owner: dir::GlobalTypeId,
        key: dir::StaticKey,
    ) -> CompilerResult<Option<(dir::GlobalTypeId, dir::GlobalSymbolId, dir::StaticKey)>> {
        let symbol = match self.ty(owner)? {
            dir::Type::Application(instance) => instance.symbol,
            dir::Type::Reference(reference) => reference.symbol,
            _ => return Ok(None),
        };
        let is_variant_family = match self.definition(symbol)? {
            Some(dir::Definition::Enum(_)) => true,
            Some(dir::Definition::Newtype(definition)) => definition.is_tagged(),
            _ => false,
        };
        if is_variant_family {
            Ok(Some((owner, symbol, key)))
        } else {
            Ok(None)
        }
    }

    /// Return the variant case named by one expression pattern.
    pub(in crate::check) fn variant_expression_case(
        &mut self,
        module: ModuleId,
        value: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<dir::VariantCase>> {
        let dir::Expression::Member {
            left,
            name: Some(name),
            ..
        } = self.module(module).view().get(value).clone()
        else {
            return Ok(None);
        };

        // peel explicit application from the owner declaration reference
        let mut owner = left;
        while let dir::Expression::Instantiation { left, .. } =
            self.module(module).view().get(owner)
        {
            owner = *left;
        }
        let Some(owner) = self.reference_symbol(owner.into_global_any(module)) else {
            return Ok(None);
        };
        let case = self.variant_case(owner, dir::StaticKey::Name(name))?;

        Ok(case)
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
        let case = member.map(|variant| dir::VariantCase {
            owner,
            key,
            variant,
        });

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
    ) -> CompilerResult<()> {
        let input = self.require_node_type(node.into_any())?;
        let owners = self.variant_owners(origin, input, &case)?;
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

            // accept a derived newtype with its checked tagged definition
            Some(dir::Definition::Newtype(value)) if value.is_tagged() => {}

            // reject every other owner, which has no variant cases
            _ => return self.reject_pattern(node, origin, owners[0].owner),
        }

        // select the requested case from every matched owner arm
        let Some(selection) = self.select_tagged_case(origin, &case, &owners)? else {
            let key = self.format_static_key(&case.key);
            self.report_pattern_variant_missing(origin, key, owners[0].owner)?;

            return self.commit_rejected_pattern(node);
        };

        let owner = self.tagged_selection_owner(origin.module(), &owners)?;
        let tag_type = self.intern_type(dir::Type::from(&selection.discriminant))?;
        let narrowed =
            self.tagged_variant_type(origin.module(), &owners, selection.case.variant)?;
        let predicate = dir::Predicate::unary(
            dir::PredicateOperand::projected(dir::Projection::VariantTag {
                carrier: owner,
                discriminator: selection.discriminator,
                ty: tag_type,
            }),
            dir::PredicateCondition::Literal(selection.discriminant),
        )
        .with_narrowed(narrowed);

        // select the optional payload projection
        let (predicate, projected) = match selection.payload {
            Some(payload) => {
                let projected = self
                    .project_tagged_payload_patterns(node, origin, flow, scope, &payload, fields)?;
                let projection = dir::Projection::VariantPayload {
                    case: selection.case.clone(),
                    backing: payload.backing,
                    discriminator: selection.discriminator,
                    discriminant: selection.discriminant,
                    ty: payload.backing,
                };
                let predicate = predicate.with_projection(projection.clone());

                (predicate, projected)
            }
            None if fields.is_empty() => (predicate, (None, Vec::new())),
            None => {
                let key = self.format_static_key(&selection.case.key);
                self.report_pattern_field_missing(origin, owner, key)?;

                return self.commit_rejected_pattern(node);
            }
        };

        self.commit_pattern(
            node,
            dir::PatternDecision::Variant(Box::new(dir::PatternVariantResolution {
                case: selection.case,
                predicate,
                payload: projected.0,
                fields: projected.1,
            })),
        )
    }

    /// Return tagged owner instances visible in the matched input.
    fn variant_owners(
        &mut self,
        origin: Origin,
        input: dir::GlobalTypeId,
        case: &dir::VariantCase,
    ) -> CompilerResult<Vec<VariantOwner>> {
        let input = self.strip_form(origin, input)?;
        let mut owners = Vec::new();

        // collect every visible owner instance from the input
        match self.ty(input)? {
            dir::Type::Application(instance) if instance.symbol == case.owner => {
                owners.push(VariantOwner {
                    owner: input,
                    instance,
                });
            }
            dir::Type::Variant(variant) if variant.variant == case.variant => {
                let dir::Type::Application(instance) = self.ty(variant.owner)? else {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "variant type {input:?} has non-application owner {:?}",
                            variant.owner
                        ),
                    });
                };
                if instance.symbol == case.owner {
                    owners.push(VariantOwner {
                        owner: variant.owner,
                        instance,
                    });
                }
            }
            dir::Type::Union(union) => {
                let arms =
                    SmallVec::<[_; 4]>::from_slice(self.type_ids(input.module_id, union.elements)?);

                for arm in arms {
                    if let dir::Type::Application(instance) = self.ty(arm)?
                        && instance.symbol == case.owner
                    {
                        owners.push(VariantOwner {
                            owner: arm,
                            instance,
                        });
                    } else if let dir::Type::Variant(variant) = self.ty(arm)?
                        && variant.variant == case.variant
                    {
                        let dir::Type::Application(instance) = self.ty(variant.owner)? else {
                            return Err(CompilerError::Internal {
                                message: format!(
                                    "variant type {arm:?} has non-application owner {:?}",
                                    variant.owner
                                ),
                            });
                        };
                        if instance.symbol == case.owner {
                            owners.push(VariantOwner {
                                owner: variant.owner,
                                instance,
                            });
                        }
                    }
                }
            }
            _ => {}
        };

        Ok(owners)
    }

    /// Return one selected case from every matched tagged owner arm.
    fn select_tagged_case(
        &mut self,
        origin: Origin,
        case: &dir::VariantCase,
        owners: &[VariantOwner],
    ) -> CompilerResult<Option<TaggedCaseSelection>> {
        let mut selected = Vec::with_capacity(owners.len());
        for owner in owners {
            let Some(selected_case) = self.instantiate_tagged_case(owner, case)? else {
                return Err(CompilerError::Internal {
                    message: format!(
                        "tagged owner {:?} does not define resolved case {:?}",
                        owner.owner, case.variant,
                    ),
                });
            };
            selected.push(selected_case);
        }

        match selected.as_slice() {
            [] => Ok(None),
            [case] => Ok(Some(case.clone())),
            _ => self.merge_tagged_cases(origin, selected),
        }
    }

    /// Merge same-case selections from multiple generic owner arms.
    fn merge_tagged_cases(
        &mut self,
        _origin: Origin,
        cases: Vec<TaggedCaseSelection>,
    ) -> CompilerResult<Option<TaggedCaseSelection>> {
        let mut cases = cases.into_iter();
        let Some(first) = cases.next() else {
            return Err(CompilerError::Internal {
                message: "cannot merge an empty tagged case set".to_string(),
            });
        };
        let case_count = cases.len() + 1;
        let mut backings = Vec::with_capacity(case_count);
        let mut fields = Vec::<dir::TypeProperty>::new();
        let selected_case = first.case;
        let selected_discriminator = first.discriminator;
        let discriminant = first.discriminant;
        if let Some(payload) = first.payload {
            backings.push(payload.backing);
            fields.extend(payload.fields);
        }

        // retain fields available through every runtime payload arm
        for case in cases {
            if case.case != selected_case
                || case.discriminator != selected_discriminator
                || case.discriminant != discriminant
            {
                return Err(CompilerError::Internal {
                    message: "tagged owner arms selected different cases".to_string(),
                });
            }
            if let Some(payload) = case.payload {
                backings.push(payload.backing);
                let mut common = Vec::with_capacity(fields.len().min(payload.fields.len()));
                for existing in fields {
                    let Some(field) = payload
                        .fields
                        .iter()
                        .find(|field| field.key == existing.key)
                    else {
                        continue;
                    };

                    // join union arm payloads, keeping writes only when every arm accepts
                    let read = self
                        .normalized_union_type([existing.access.store(), field.access.store()])?;
                    let access = match existing.access.is_writable() && field.access.is_writable() {
                        true => dir::PropertyAccess::ReadWrite { read, write: read },
                        false => dir::PropertyAccess::Read(read),
                    };
                    common.push(dir::TypeProperty {
                        key: existing.key,
                        access,
                        is_optional: existing.is_optional || field.is_optional,
                    });
                }
                fields = common;
            }
        }

        let payload = if backings.is_empty() {
            None
        } else if backings.len() == case_count {
            Some(TaggedPayload {
                backing: self.normalized_union_type(backings)?,
                fields,
            })
        } else {
            return Err(CompilerError::Internal {
                message: "one tagged case mixed unit and payload-bearing definitions".to_string(),
            });
        };

        Ok(Some(TaggedCaseSelection {
            case: selected_case,
            discriminator: selected_discriminator,
            discriminant,
            payload,
        }))
    }

    /// Return the narrowed owner type selected by a tagged case.
    fn tagged_selection_owner(
        &mut self,
        _module: ModuleId,
        owners: &[VariantOwner],
    ) -> CompilerResult<dir::GlobalTypeId> {
        match owners {
            [owner] => Ok(owner.owner),
            _ => self.normalized_union_type(owners.iter().map(|owner| owner.owner)),
        }
    }

    /// Return the case-specific type selected across tagged owner instances.
    fn tagged_variant_type(
        &mut self,
        _module: ModuleId,
        owners: &[VariantOwner],
        member: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let mut variants = Vec::with_capacity(owners.len());
        for owner in owners {
            let variant = self.intern_type(dir::Type::Variant(dir::VariantType {
                owner: owner.owner,
                variant: member,
            }))?;
            variants.push(variant);
        }

        self.normalized_union_type(variants)
    }

    /// Select one tagged variant.
    fn instantiate_tagged_case(
        &mut self,
        owner: &VariantOwner,
        case: &dir::VariantCase,
    ) -> CompilerResult<Option<TaggedCaseSelection>> {
        let Some(dir::Definition::Newtype(definition)) = self.definition(owner.instance.symbol)?
        else {
            return Err(CompilerError::Internal {
                message: format!(
                    "tagged owner {:?} has no newtype definition",
                    owner.instance.symbol
                ),
            });
        };
        let Some(discriminator) = definition.discriminator else {
            return Err(CompilerError::Internal {
                message: format!(
                    "tagged owner {:?} has no discriminator",
                    owner.instance.symbol
                ),
            });
        };
        let Some(variant) = definition.tagged_variant_by_key(case.key).cloned() else {
            return Ok(None);
        };

        // instantiate the exact backing and constructor argument together
        let substitution = self
            .instance_substitution(owner.owner.module_id, &owner.instance)?
            .with_receiver(owner.owner);
        let payload = match variant.argument {
            Some(argument) => {
                let backing = self.substitute_type(variant.backing, &substitution)?;
                let argument = self.substitute_type(argument, &substitution)?;
                let (dir::Type::Shape(shape) | dir::Type::Object(shape)) = self.ty(argument)?
                else {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "tagged variant has invalid constructor argument {argument:?}"
                        ),
                    });
                };
                let fields = self
                    .shape_properties(argument.module_id, shape.properties)?
                    .to_vec();

                Some(TaggedPayload { backing, fields })
            }
            None => None,
        };
        let discriminant = dir::ScalarLiteral::String(variant.discriminant);
        Ok(Some(TaggedCaseSelection {
            case: case.clone(),
            discriminator,
            discriminant,
            payload,
        }))
    }

    /// Project written patterns from one Tagged case payload.
    fn project_tagged_payload_patterns(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        payload: &TaggedPayload,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<(
        Option<dir::GlobalNodeIdAny>,
        Vec<dir::PatternFieldResolution>,
    )> {
        let module = node.module_id;
        let mut payload_pattern = None;
        let mut projected = Vec::with_capacity(fields.len());
        let mut position = 0usize;
        for field in fields {
            let source = field.into_global_any(module);
            let field_node = self.module(module).view().get(*field).clone();
            match field_node {
                dir::PatternField::Positional { pattern } => {
                    let pattern_node = self.module(module).view().get(pattern);
                    if fields.len() == 1 && matches!(pattern_node, dir::Pattern::Object { .. }) {
                        self.check_pattern_projection(
                            flow,
                            scope,
                            payload.backing,
                            pattern.into_global_any(module),
                        )?;
                        payload_pattern = Some(pattern.into_global_any(module));
                        position += 1;

                        continue;
                    }

                    let Some(payload_field) = payload.fields.get(position) else {
                        let key = position.to_string();
                        self.report_pattern_field_missing(origin, payload.backing, key)?;
                        position += 1;

                        continue;
                    };
                    let pattern_type = self.tagged_payload_field_type(module, payload_field)?;

                    self.check_pattern_projection(
                        flow,
                        scope,
                        pattern_type,
                        pattern.into_global_any(module),
                    )?;
                    projected.push(dir::PatternFieldResolution {
                        source,
                        projection: dir::Projection::Field(dir::FieldResolution {
                            receiver: dir::MemberReceiver::direct(payload.backing),
                            target: dir::FieldTarget::Structural {
                                owner: payload.backing,
                                key: payload_field.key,
                            },
                            ty: pattern_type,
                        })
                        .into(),
                        pattern: Some(pattern.into_global_any(module)),
                    });
                    position += 1;
                }
                dir::PatternField::Named { name, pattern, .. } => {
                    let key = name.static_key();
                    let Some(payload_field) = payload.fields.iter().find(|field| field.key == key)
                    else {
                        let key = self.format_static_key(&key);
                        self.report_pattern_field_missing(origin, payload.backing, key)?;

                        continue;
                    };
                    let input = self.tagged_payload_field_type(module, payload_field)?;

                    if let Some(pattern) = pattern {
                        self.check_pattern_projection(
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
                        projection: dir::Projection::Field(dir::FieldResolution {
                            receiver: dir::MemberReceiver::direct(payload.backing),
                            target: dir::FieldTarget::Structural {
                                owner: payload.backing,
                                key,
                            },
                            ty: input,
                        })
                        .into(),
                        pattern: pattern.map(|pattern| pattern.into_global_any(module)),
                    });
                }
                dir::PatternField::Elision => {
                    position += 1;
                }
                dir::PatternField::Computed { .. } | dir::PatternField::Rest { .. } => {
                    self.report_pattern_source_not_object_shaped(origin, payload.backing)?;
                }
            }
        }

        Ok((payload_pattern, projected))
    }

    /// Return one tagged payload field's projected value type.
    fn tagged_payload_field_type(
        &mut self,
        _module: ModuleId,
        field: &dir::TypeProperty,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let ty = field.access.store();
        if !field.is_optional {
            return Ok(ty);
        }
        let undefined = self.intern_type(dir::Type::Undefined)?;

        self.normalized_union_type([ty, undefined])
    }
}

impl CheckState<'_> {
    /// Return the selected case key for one Tagged discriminant.
    pub(in crate::check) fn tagged_case_key_from_discriminant(
        &self,
        owner: dir::GlobalSymbolId,
        discriminant: dir::ScalarLiteral,
    ) -> Option<dir::StaticKey> {
        let dir::ScalarLiteral::String(discriminant) = discriminant else {
            return None;
        };
        let Some(dir::Definition::Newtype(definition)) = self.definition_maybe(owner) else {
            return None;
        };

        definition
            .tagged_variant_by_discriminant(discriminant)
            .map(|variant| variant.key)
    }

    /// Return the selected case key for one Tagged owner type and discriminant.
    pub(in crate::check) fn tagged_case_key_from_type(
        &mut self,
        owner: dir::GlobalTypeId,
        discriminant: dir::ScalarLiteral,
    ) -> CompilerResult<Option<dir::StaticKey>> {
        let Some(instance) = self.tagged_domain_instance(owner)? else {
            return Ok(None);
        };
        let key = self.tagged_case_key_from_discriminant(instance.symbol, discriminant);

        Ok(key)
    }

    /// Return the finite discriminant domain of one Tagged newtype value.
    pub(in crate::check) fn tagged_discriminant_domain(
        &mut self,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Option<Vec<dir::ScalarLiteral>>> {
        let Some(instance) = self.tagged_domain_instance(value)? else {
            return Ok(None);
        };
        let Some(dir::Definition::Newtype(definition)) = self.definition(instance.symbol)? else {
            return Ok(None);
        };
        if !definition.is_tagged() {
            return Ok(None);
        }
        let domain = definition
            .tagged_variants()
            .map(|variant| dir::ScalarLiteral::String(variant.discriminant))
            .collect();

        Ok(Some(domain))
    }

    /// Return the instance represented by one Tagged domain owner type.
    fn tagged_domain_instance(
        &mut self,
        mut value: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GenericApplication>> {
        if let dir::Type::Variant(variant) = self.ty(value)? {
            value = variant.owner;
        }

        let instance = match self.ty(value)? {
            dir::Type::Application(instance) => instance,
            dir::Type::Reference(reference) => {
                if let Some(template) = self.symbol_template(reference.symbol)? {
                    let parameters = self.generic_template_parameters(template)?;
                    if !parameters.is_empty() {
                        return Ok(None);
                    }
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
