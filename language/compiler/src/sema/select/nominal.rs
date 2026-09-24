use destack_dir as dir;
use smallvec::SmallVec;

use crate::sema::{Cause, CauseKind, CheckState, FlowPointId, Origin, Relation};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
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

        // require a named type for the nominal pattern
        let head = self.construct_type(origin, module, ty, None)?;
        let Some(instance) = self.decompose_newtype(origin, head)? else {
            return self.report_rejected_pattern(node, origin, head);
        };

        // deny an unwrap the backing's declared visibility rejects
        self.check_backing_access(origin, instance.symbol)?;

        // read the backing the newtype wraps
        let backing = instance.backing;
        let projection = instance.into_projection();

        // check the backing against the wrapped pattern
        let value = fields
            .first()
            .and_then(|field| match self.module(module).view().get(*field) {
                dir::PatternField::Positional { pattern } => Some(*pattern),
                _ => None,
            });
        if let Some(value) = value {
            self.check_pattern_projection(flow, scope, backing, value.into_global_any(module))?;
        }

        // commit the newtype projection
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
        // reject fields, an enum member matches by discriminant alone
        if !fields.is_empty() {
            return self.report_rejected_pattern(node, origin, owners[0]);
        }

        // read the selected variant from its enum definition
        let value = match self.definition(case.owner)?.as_deref() {
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
        let owner_union = self.normalized_union_type(owners.iter().copied())?;
        let predicate = dir::Predicate::unary(
            dir::PredicateOperand::direct(owner_union),
            dir::PredicateCondition::Literal(discriminant),
        )
        .with_narrowed(narrowed);

        // commit the selected variant case
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

        // reject invalid rest fields
        if !self.report_pattern_rest_fields(module, fields) {
            return self.commit_rejected_pattern(node);
        }

        // select owner.case patterns before nominal fields
        if self.select_variant_type_pattern(node, origin, module, ty, fields)? {
            return Ok(());
        }

        // require a named type for the nominal pattern
        let head = self.construct_type(origin, module, ty, None)?;
        let instance = match self.ty(head)? {
            dir::Type::Application(instance) => instance,
            _ => return self.report_rejected_pattern(node, origin, head),
        };

        // bind the pattern instantiation from the matched input, its fields through its borrow
        let input = self.require_node_type(node.into_any())?;
        let binding = self.pattern_binding_form(origin, input)?;
        let (payload, _) = self.project_newtype_receiver(origin, input)?;
        let arms: SmallVec<[dir::GlobalTypeId; 4]> = match self.union_arms(origin, payload)? {
            Some(arms) => arms,
            None => SmallVec::from_slice(&[payload]),
        };

        // equate the arm naming the written declaration with the written head
        let mut adjustments = Vec::new();
        let mut matched = head;
        for arm in arms {
            let base = self.form_chain(origin, arm)?.base();
            if let dir::Type::Application(arm_instance) = self.ty(base)?
                && arm_instance.symbol == instance.symbol
            {
                let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
                self.constrain_type(origin, cause, Relation::Equal, base, head)?;
                adjustments = self.project_narrowed_receiver(origin, input, arm)?;
                matched = arm;
                break;
            }
        }

        // project declared fields off the matched arm, in its own forms
        let (fields, rest) =
            self.project_named_fields(node, origin, flow, scope, matched, fields, binding)?;
        let arguments: SmallVec<[_; 8]> = self.type_ids(head.module_id, instance.arguments)?.into();
        let generic_arguments =
            self.symbol_generic_argument_bindings(instance.symbol, &arguments)?;
        self.commit_pattern(
            node,
            dir::PatternDecision::Destructure(Box::new(
                dir::PatternDestructureResolution::Nominal(
                    dir::PatternNominalDestructureResolution {
                        adjustments,
                        key: dir::InstanceKey::new(instance.symbol, generic_arguments),
                        fields,
                        rest: rest.map(Box::new),
                    },
                ),
            )),
        )
    }
}
