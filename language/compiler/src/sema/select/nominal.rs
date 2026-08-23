use destack_dir as dir;
use smallvec::SmallVec;

use crate::sema::{BodyState, Cause, CauseKind, FlowPointId, Origin, Relation};
use crate::{CompilerError, CompilerResult};

impl BodyState<'_, '_> {
    /// Select one newtype pattern, unwrapping the substituted backing.
    pub(in crate::sema) fn select_newtype_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        ty: dir::LocalNodeId<dir::TypeExpression>,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<()> {
        let module = node.module_id;
        self.report_duplicate_pattern_bindings(module, fields)?;

        // reject invalid rest fields
        if !self.report_pattern_rest_fields(module, fields) {
            return self.commit_rejected_pattern(node);
        }

        // select owner.case patterns before ordinary newtype unwraps
        if self.select_variant_type_pattern(node, origin, module, ty, fields)? {
            return Ok(());
        }

        // resolve the written nominal tag like a construction head
        let tag = self.written_construct_tag(origin, module, ty)?;
        let Some(instance) = self.newtype_payload(origin, tag)? else {
            return self.report_rejected_pattern(node, origin, tag);
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
            self.check_pattern_projection(flow, scope, backing, value.into_global_any(module))?;
        }

        self.commit_pattern(
            node,
            dir::PatternDecision::Project(Box::new(dir::PatternProjectionResolution {
                projection: projection.into(),
                pattern: value.map(|value| value.into_global_any(module)),
            })),
        )
    }

    /// Select one enum member pattern.
    pub(in crate::sema) fn select_enum_member_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        case: dir::VariantCase,
        owners: &[dir::GlobalTypeId],
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<()> {
        // reject fields, enum members have no payload to destructure
        if !fields.is_empty() {
            return self.report_rejected_pattern(node, origin, owners[0]);
        }

        // read the selected variant from its enum definition
        let value = match self.definition(case.owner)? {
            Some(dir::Definition::Enum(definition)) => definition
                .variants()
                .find(|variant| variant.symbol == case.variant)
                .map(|variant| variant.value)
                .ok_or_else(|| CompilerError::Internal {
                    message: format!(
                        "enum {:?} lost selected variant {:?}",
                        case.owner, case.variant
                    ),
                })?,
            _ => {
                return Err(CompilerError::Internal {
                    message: format!(
                        "enum member pattern received non-enum owner {:?}",
                        case.owner
                    ),
                });
            }
        };

        // test the discriminant and narrow to the variant's own type
        let discriminant = dir::Literal::from(value);
        let mut narrowed = Vec::with_capacity(owners.len());
        for owner in owners {
            let member = self.intern_type(dir::Type::Variant(dir::VariantType {
                owner: *owner,
                variant: case.variant,
            }))?;
            narrowed.push(member);
        }
        let narrowed = self.normalized_union_type(narrowed)?;
        let carrier = self.normalized_union_type(owners.iter().copied())?;
        let predicate = dir::Predicate::unary(
            dir::PredicateOperand::direct(carrier),
            dir::PredicateCondition::Literal(discriminant),
        )
        .with_narrowed(narrowed);

        self.commit_pattern(
            node,
            dir::PatternDecision::Variant(Box::new(dir::PatternVariantResolution {
                case,
                predicate,
            })),
        )
    }

    /// Select one nominal object pattern, projecting declared fields.
    pub(in crate::sema) fn select_nominal_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        ty: dir::LocalNodeId<dir::TypeExpression>,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<()> {
        let module = node.module_id;
        self.report_duplicate_pattern_bindings(module, fields)?;

        if !self.report_pattern_rest_fields(module, fields) {
            return self.commit_rejected_pattern(node);
        }

        // select owner.case patterns before nominal fields
        if self.select_variant_type_pattern(node, origin, module, ty, fields)? {
            return Ok(());
        }

        // resolve the written nominal tag like a construction head
        let tag = self.written_construct_tag(origin, module, ty)?;
        let instance = match self.ty(tag)? {
            dir::Type::Application(instance) => instance,
            _ => return self.report_rejected_pattern(node, origin, tag),
        };

        // bind the pattern instantiation from the matched input
        let input = self.require_node_type(node.into_any())?;
        let input = self.strip_form(origin, input)?;
        let mut matched = input;
        if let Some(instance) = self.decompose_newtype(origin, input)? {
            let backing = instance.backing;
            matched = backing;
        }
        let arms: SmallVec<[dir::GlobalTypeId; 4]> = match self.ty(matched)? {
            dir::Type::Union(union) => {
                SmallVec::from_slice(self.type_ids(matched.module_id, union.elements)?)
            }
            _ => SmallVec::from_slice(&[matched]),
        };
        for arm in arms {
            if let dir::Type::Application(arm_instance) = self.ty(arm)?
                && arm_instance.symbol == instance.symbol
            {
                let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
                self.constrain_type(origin, cause, Relation::Equal, arm, tag)?;
                break;
            }
        }

        // project declared fields off the matched declaration
        let (fields, rest) = self.project_named_fields(node, origin, flow, scope, tag, fields)?;
        let arguments: SmallVec<[_; 8]> = self.type_ids(tag.module_id, instance.arguments)?.into();
        let generic_arguments =
            self.symbol_generic_argument_bindings(instance.symbol, &arguments)?;
        self.commit_pattern(
            node,
            dir::PatternDecision::Destructure(Box::new(
                dir::PatternDestructureResolution::Nominal(
                    dir::PatternNominalDestructureResolution {
                        selection: dir::Selection::new(instance.symbol, generic_arguments),
                        fields,
                        rest: rest.map(Box::new),
                    },
                ),
            )),
        )
    }
}
