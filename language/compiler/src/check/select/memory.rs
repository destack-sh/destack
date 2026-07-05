use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, Decision, FlowPointId, Origin, answer, unary_operator_protocols,
};

/// Dereference operation selected for one value.
pub(in crate::check) struct DereferenceSelection {
    /// The selected dereference operation.
    pub(in crate::check) operation: dir::DereferenceOperation,
    /// The projected value type.
    pub(in crate::check) ty: dir::GlobalTypeId,
}

impl CheckState<'_> {
    /// Select one dereference operation.
    pub(in crate::check) fn select_dereference(
        &mut self,
        origin: Origin,
        input: dir::GlobalTypeId,
        access: dir::Access,
    ) -> CompilerResult<Answer<Option<DereferenceSelection>>> {
        let module = origin.module();
        let input = answer!(self.reduce_type_head(origin, input)?);

        // direct dereference projects physical pointer forms
        if let dir::Type::Form(form) = self.ty(input)?
            && matches!(form.form, dir::Form::Borrowed { .. } | dir::Form::Raw)
        {
            return Ok(Answer::Ready(Some(DereferenceSelection {
                operation: dir::DereferenceOperation::Direct,
                ty: form.value,
            })));
        }

        // protocol dereference handles smart pointer values
        for operator_protocol in unary_operator_protocols(dir::UnaryOperator::Dereference, access) {
            let key = operator_protocol.method.key(&self.module(module).strings);
            let protocol = self.operator_protocol(origin, &operator_protocol, &[])?;
            let Some(call) = answer!(self.select_protocol_call(
                origin,
                input,
                input,
                key,
                &protocol,
                &[],
                &[],
            )?) else {
                continue;
            };
            let ty = answer!(self.operator_expression_type(
                origin,
                operator_protocol.expression_result,
                call.return_type,
            )?);

            return Ok(Answer::Ready(Some(DereferenceSelection {
                operation: dir::DereferenceOperation::Call(call.resolution),
                ty,
            })));
        }

        Ok(Answer::Ready(None))
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
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;
        let access = mutability
            .map(dir::Mutability::access)
            .unwrap_or(dir::Access::Mutable);
        let access_type = self.intern_type(
            module,
            dir::Type::Memory(dir::MemoryLiteral::Access(access)),
        )?;
        let lifetime = self.intern_type(
            module,
            dir::Type::Memory(dir::MemoryLiteral::Lifetime(dir::Lifetime::Frame)),
        )?;
        let projected = self.intern_type(
            module,
            dir::Type::Form(dir::FormType {
                form: dir::Form::Borrowed {
                    lifetime,
                    access: access_type,
                },
                value: input,
            }),
        )?;

        self.project_pattern_input(flow, scope, projected, pattern.into_global_any(module))?;

        self.commit_pattern(
            node,
            dir::PatternResolution::Project(dir::PatternProjectionResolution {
                projection: dir::Projection::Borrow {
                    access: Some(access),
                    ty: projected,
                },
                pattern: Some(pattern.into_global_any(module)),
            }),
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
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;
        let access = mutability.map(dir::Mutability::access);

        self.project_pattern_input(flow, scope, input, pattern.into_global_any(module))?;

        self.commit_pattern(
            node,
            dir::PatternResolution::Project(dir::PatternProjectionResolution {
                projection: dir::Projection::Move { access, ty: input },
                pattern: Some(pattern.into_global_any(module)),
            }),
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
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;
        let Some(selection) =
            answer!(self.select_dereference(origin, input, dir::Access::Readonly)?)
        else {
            self.commit_decision(node.into_any(), Decision::Rejected)?;

            return Ok(Answer::Ready(()));
        };
        self.project_pattern_input(flow, scope, selection.ty, pattern.into_global_any(module))?;

        self.commit_pattern(
            node,
            dir::PatternResolution::Project(dir::PatternProjectionResolution {
                projection: dir::Projection::Dereference {
                    read: selection.operation,
                    ty: selection.ty,
                },
                pattern: Some(pattern.into_global_any(module)),
            }),
        )
    }
}
