use destack_dir as dir;

use crate::sema::{BodyState, FlowPointId, Origin, Value, unary_operator_protocols};
use crate::{CompilerError, CompilerResult};

impl BodyState<'_, '_> {
    /// Select one dereference operation.
    pub(in crate::sema) fn select_dereference(
        &mut self,
        origin: Origin,
        input: Value,
        access: dir::Access,
    ) -> CompilerResult<Option<dir::DereferenceResolution>> {
        // project physical pointer forms directly when the access grants it
        if let dir::Type::Form(form) = self.ty(input.ty)?
            && matches!(form.form, dir::Form::Borrowed(_) | dir::Form::Raw)
        {
            if let dir::Form::Borrowed(borrow) = form.form {
                // require the requested access from the selected borrow
                let borrow = self.check.type_borrow(input.ty.module_id, borrow)?;
                let requested = self.access_literal(access)?;
                if !self
                    .check
                    .constrain_access_assignable(origin, borrow.access, requested)?
                    .holds()
                {
                    return Ok(None);
                }
            }

            // the referent's own type carries its places, the resolution its placement
            let dereference = dir::Dereference {
                receiver: input.ty,
                target: dir::DereferenceTarget::Direct,
                ty: form.value,
            };

            return Ok(Some(dir::OperationResolution::One(dereference)));
        }

        // union values select one exact dereference operation for every runtime arm
        if let Some(arms) = self.union_arms(origin, input.ty)? {
            let mut resolutions = Vec::with_capacity(arms.len());
            let mut types = Vec::with_capacity(arms.len());
            for arm in arms {
                let Some(resolution) =
                    self.select_dereference(origin, Value { ty: arm, ..input }, access)?
                else {
                    return Ok(None);
                };

                let dir::OperationResolution::One(dereference) = resolution else {
                    return Err(CompilerError::Internal {
                        message: "union dereference contains a nested union".to_string(),
                    });
                };
                types.push(dereference.ty);
                resolutions.push(dereference);
            }
            let ty = self.normalized_union_type(types)?;

            return Ok(Some(dir::OperationResolution::Union {
                arms: resolutions,
                ty,
            }));
        }

        // dereference smart pointer values through their protocol
        for operator_protocol in unary_operator_protocols(dir::UnaryOperator::Dereference, access) {
            let key = operator_protocol.method.key(self.strings());
            let protocol = self.operator_protocol(origin, &operator_protocol, &[])?;
            let Some(call) = self.select_protocol_call(
                origin,
                input,
                input.ty,
                dir::MemberSpace::Instance,
                key,
                &protocol,
                &[],
            )?
            else {
                continue;
            };
            let ty = self.operator_expression_type(
                origin,
                operator_protocol.expression_result,
                call.return_type,
            )?;

            let dir::OperationResolution::One(call) = call.resolution else {
                return Err(CompilerError::Internal {
                    message: "protocol dereference contains a nested union".to_string(),
                });
            };
            let dereference = dir::Dereference {
                receiver: input.ty,
                target: dir::DereferenceTarget::Call(Box::new(call)),
                ty,
            };

            return Ok(Some(dir::OperationResolution::One(dereference)));
        }

        Ok(None)
    }

    /// Select one borrow pattern projection.
    pub(in crate::sema) fn select_borrow_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        input: dir::GlobalTypeId,
        pattern: dir::LocalNodeId<dir::Pattern>,
        mutability: Option<dir::Mutability>,
    ) -> CompilerResult<()> {
        let module = node.module_id;
        let access = mutability
            .map(dir::Mutability::access)
            .unwrap_or(dir::Access::Mutable);
        let access_type = self.access_literal(access)?;
        let lifetime = self.lifetime_literal(dir::Lifetime::Frame)?;

        // borrow through a managed handle at the handle's place
        let (place, value) = match self.ty(input)? {
            dir::Type::Form(form) if let dir::Form::Managed { place } = form.form => {
                (place, form.value)
            }
            _ => (self.check.local_place()?, input),
        };
        let region = self.check.intern_region(lifetime, place)?;
        let form = self.intern_borrow(region, access_type)?;
        let projected = self.intern_type(dir::Type::Form(dir::FormType { form, value }))?;

        self.check_pattern_projection(flow, scope, projected, pattern.into_global_any(module))?;

        self.commit_pattern(
            node,
            dir::PatternDecision::Project(Box::new(dir::PatternProjectionResolution {
                projection: dir::Projection::Borrow {
                    access: Some(access),
                    ty: projected,
                }
                .into(),
                pattern: Some(pattern.into_global_any(module)),
            })),
        )
    }

    /// Select one move pattern projection.
    pub(in crate::sema) fn select_move_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        input: dir::GlobalTypeId,
        pattern: dir::LocalNodeId<dir::Pattern>,
        mutability: Option<dir::Mutability>,
    ) -> CompilerResult<()> {
        let module = node.module_id;
        let access = mutability.map(dir::Mutability::access);

        self.check_pattern_projection(flow, scope, input, pattern.into_global_any(module))?;

        self.commit_pattern(
            node,
            dir::PatternDecision::Project(Box::new(dir::PatternProjectionResolution {
                projection: dir::Projection::Move { access, ty: input }.into(),
                pattern: Some(pattern.into_global_any(module)),
            })),
        )
    }

    /// Select one dereference pattern projection.
    pub(in crate::sema) fn select_dereference_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        input: dir::GlobalTypeId,
        pattern: dir::LocalNodeId<dir::Pattern>,
    ) -> CompilerResult<()> {
        let module = node.module_id;
        let Some(selection) = self.select_dereference(
            origin,
            Value {
                ty: input,
                node: None,
                place: None,
                is_fresh: false,
            },
            dir::Access::Readonly,
        )?
        else {
            return self.commit_rejected_pattern(node);
        };
        self.check_pattern_projection(
            flow,
            scope,
            selection.ty(),
            pattern.into_global_any(module),
        )?;

        self.commit_pattern(
            node,
            dir::PatternDecision::Project(Box::new(dir::PatternProjectionResolution {
                projection: selection.into(),
                pattern: Some(pattern.into_global_any(module)),
            })),
        )
    }
}
