use destack_dir as dir;

use crate::check::{BodyState, FlowPointId, Origin, Value, unary_operator_protocols};
use crate::{CompilerError, CompilerResult};

impl BodyState<'_, '_> {
    /// Select one dereference operation.
    pub(in crate::check) fn select_dereference(
        &mut self,
        origin: Origin,
        input: Value,
        access: dir::Access,
    ) -> CompilerResult<Option<dir::DereferenceResolution>> {
        // direct dereference projects physical pointer forms the access grants
        if let dir::Type::Form(form) = self.ty(input.ty)?
            && matches!(form.form, dir::Form::Borrowed(_) | dir::Form::Raw)
        {
            if let dir::Form::Borrowed(borrow) = form.form {
                // require the requested access from the selected borrow
                let held = self.check.type_borrow(input.ty.module_id, borrow)?.access;
                let requested =
                    self.intern_type(dir::Type::Memory(dir::MemoryLiteral::Access(access)))?;
                if !self
                    .check
                    .constrain_access_assignable(origin, held, requested)?
                {
                    return Ok(None);
                }
            }

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

        // protocol dereference handles smart pointer values
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
            let ty = self
                .operator_expression_type(operator_protocol.expression_result, call.return_type)?;

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
    pub(in crate::check) fn select_borrow_pattern(
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
        let access_type =
            self.intern_type(dir::Type::Memory(dir::MemoryLiteral::Access(access)))?;
        let lifetime = self.intern_type(dir::Type::Memory(dir::MemoryLiteral::Lifetime(
            dir::Lifetime::Frame,
        )))?;
        let form = self.intern_borrow(lifetime, access_type)?;
        let projected = self.intern_type(dir::Type::Form(dir::FormType { form, value: input }))?;

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
    pub(in crate::check) fn select_move_pattern(
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
    pub(in crate::check) fn select_dereference_pattern(
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
                place: None,
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
