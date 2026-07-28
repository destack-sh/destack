use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    Answer, BodyState, Cause, CauseKind, FlowPointId, Origin, Relation, VariantOwner, answer,
};
use crate::{CompilerError, CompilerResult};

impl BodyState<'_, '_> {
    /// Select one newtype pattern, unwrapping the substituted backing.
    pub(in crate::check) fn select_newtype_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        ty: dir::LocalNodeId<dir::TypeExpression>,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;
        self.check_pattern_bindings(module, fields)?;

        // reject invalid rest fields
        if !self.check_pattern_rest_fields(module, fields) {
            return self.commit_rejected_pattern(node);
        }

        // select owner.case patterns before ordinary newtype unwraps
        if answer!(
            self.select_variant_type_pattern(node, origin, flow, scope, module, ty, fields,)?
        ) {
            return Ok(Answer::Ready(()));
        }

        // resolve the written nominal tag like a construction head
        let tag = answer!(self.written_construct_tag(origin, module, ty)?);
        let tag = answer!(self.reduce_type_head(origin, tag)?);
        let Some(instance) = self.newtype_payload(origin, tag)? else {
            return self.reject_pattern(node, origin, tag);
        };
        let backing = instance.backing;
        let projection = instance.into_projection();

        // flow the backing into the wrapped hole
        let value = fields
            .first()
            .and_then(|field| match self.module(module).view().get(*field) {
                dir::PatternField::Positional { pattern } => Some(*pattern),
                _ => None,
            });
        if let Some(value) = value {
            answer!(self.check_pattern_projection(
                flow,
                scope,
                backing,
                value.into_global_any(module)
            )?);
        }

        self.commit_pattern(
            node,
            dir::PatternResolution::Project(Box::new(dir::PatternProjectionResolution {
                projection: projection.into(),
                pattern: value.map(|value| value.into_global_any(module)),
            })),
        )
    }

    /// Select one enum member pattern.
    pub(in crate::check) fn select_enum_member_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        case: dir::VariantCase,
        owners: &[VariantOwner],
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<Answer<()>> {
        // reject fields, enum members have no payload to destructure
        if !fields.is_empty() {
            return self.reject_pattern(node, origin, owners[0].owner);
        }

        // select the declared variant by the written key
        let variant = match self.definition(case.owner)? {
            Some(dir::Definition::Enum(definition)) => definition
                .variant_by_key(case.key)
                .map(|variant| (variant.symbol, variant.value)),
            _ => {
                return Err(CompilerError::Internal {
                    message: format!(
                        "enum member pattern received non-enum owner {:?}",
                        case.owner
                    ),
                });
            }
        };
        let Some((member, value)) = variant else {
            let key = self.format_static_key(&case.key);
            self.report_pattern_variant_missing(origin, key, owners[0].owner)?;

            return self.commit_rejected_pattern(node);
        };

        // test the discriminant and narrow to the variant's own type
        let discriminant = dir::ScalarLiteral::from(value);
        let module = origin.module();
        let mut narrowed = Vec::with_capacity(owners.len());
        for owner in owners {
            let member = self.intern_type(
                module,
                dir::Type::Variant(dir::VariantType {
                    owner: owner.owner,
                    variant: member,
                }),
            )?;
            narrowed.push(member);
        }
        let narrowed = self.normalized_union_type(module, narrowed)?;
        let carrier = self.normalized_union_type(module, owners.iter().map(|owner| owner.owner))?;
        let predicate = dir::Predicate::unary(
            dir::PredicateOperand::direct(carrier),
            dir::PredicateCondition::Literal(discriminant),
        )
        .with_narrowed(narrowed);

        self.commit_pattern(
            node,
            dir::PatternResolution::Variant(Box::new(dir::PatternVariantResolution {
                case,
                predicate,
                payload: None,
                fields: Vec::new(),
            })),
        )
    }

    /// Select one nominal object pattern, projecting declared fields.
    pub(in crate::check) fn select_nominal_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        ty: dir::LocalNodeId<dir::TypeExpression>,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;
        self.check_pattern_bindings(module, fields)?;

        if !self.check_pattern_rest_fields(module, fields) {
            return self.commit_rejected_pattern(node);
        }

        // select owner.case patterns before nominal fields
        if answer!(
            self.select_variant_type_pattern(node, origin, flow, scope, module, ty, fields,)?
        ) {
            return Ok(Answer::Ready(()));
        }

        // resolve the written nominal tag like a construction head
        let tag = answer!(self.written_construct_tag(origin, module, ty)?);
        let tag = answer!(self.reduce_type_head(origin, tag)?);
        let instance = match self.ty(tag)? {
            dir::Type::Application(instance) => instance,
            _ => return self.reject_pattern(node, origin, tag),
        };

        // bind the pattern instantiation from the matched input
        let input = self.require_node_type(node.into_any())?;
        let input = answer!(self.strip_form(origin, input)?);
        let mut matched = input;
        if let Some(instance) = self.decompose_newtype(origin, input)? {
            let backing = instance.backing;
            matched = answer!(self.reduce_type_head(origin, backing)?);
        }
        let arms: SmallVec<[dir::GlobalTypeId; 4]> = match self.ty(matched)? {
            dir::Type::Union(union) => {
                SmallVec::from_slice(self.type_ids(matched.module_id, union.elements)?)
            }
            _ => SmallVec::from_slice(&[matched]),
        };
        for arm in arms {
            let arm = answer!(self.reduce_type_head(origin, arm)?);
            if let dir::Type::Application(arm_instance) = self.ty(arm)?
                && arm_instance.symbol == instance.symbol
            {
                let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
                answer!(self.constrain_type(origin, cause, Relation::Equal, arm, tag)?);
                break;
            }
        }

        // project declared fields off the matched declaration
        let (fields, rest) =
            answer!(self.project_named_fields(node, origin, flow, scope, tag, fields)?);
        let arguments = self.type_ids(tag.module_id, instance.arguments)?.to_vec();
        let generic_arguments =
            self.symbol_generic_argument_bindings(instance.symbol, &arguments)?;
        self.commit_pattern(
            node,
            dir::PatternResolution::Destructure(Box::new(
                dir::PatternDestructureResolution::Nominal(
                    dir::PatternNominalDestructureResolution {
                        symbol: instance.symbol,
                        generic_arguments,
                        fields,
                        rest: rest.map(Box::new),
                    },
                ),
            )),
        )
    }
}
