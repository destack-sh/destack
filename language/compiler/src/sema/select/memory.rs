use destack_dir as dir;

use crate::sema::{CheckState, FlowPointId, Origin, Value, unary_operator_protocols};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
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
                let borrow = self.type_borrow(input.ty.module_id, borrow)?;
                let requested = self.access_literal(access)?;
                if !self
                    .constrain_access_assignable(origin, borrow.access, requested)?
                    .holds()
                {
                    return Ok(None);
                }
            }

            // read the owned object behind the pointer
            let owned = self.intern_type(dir::Type::Form(dir::FormType {
                form: dir::Form::Owned,
                value: form.value,
            }))?;
            let ty = self.reduce_redundant_forms(origin, owned)?;
            let dereference = dir::Dereference {
                receiver: input.ty,
                protocol: None,
                ty,
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
            let Ok(call) = self.select_protocol_call(
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
                protocol: Some(Box::new(call)),
                ty,
            };

            return Ok(Some(dir::OperationResolution::One(dereference)));
        }

        Ok(None)
    }

    /// Return whether one dereference reads through an explicit form.
    pub(in crate::sema) fn is_form_dereference(
        &self,
        dereference: &dir::Dereference,
    ) -> CompilerResult<bool> {
        Ok(matches!(
            self.ty(self.shallow_resolve(dereference.receiver)?)?,
            dir::Type::Form(form) if matches!(form.form, dir::Form::Readonly | dir::Form::Owned)
        ))
    }

    /// Select one borrow pattern projection.
    pub(in crate::sema) fn select_borrow_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        input: dir::GlobalTypeId,
        pattern: dir::LocalNodeId<dir::Pattern>,
        access: Option<dir::Access>,
    ) -> CompilerResult<()> {
        let module = node.module_id;
        let access = access.unwrap_or(dir::Access::BARE);
        let access_type = self.access_literal(access)?;
        let lifetime = self.lifetime_literal(dir::Lifetime::Frame)?;

        // borrow in the space the object stores in
        let origin = Origin::Node(node.into_any(), scope);
        let place = self.space_term(origin, input)?;
        let region = self.intern_region(lifetime, place)?;
        let form = self.intern_borrow(region, access_type)?;
        let projected = self.intern_type(dir::Type::Form(dir::FormType { form, value: input }))?;

        // check the wrapped pattern against the borrowed value
        self.check_pattern_projection(flow, scope, projected, pattern.into_global_any(module))?;

        // commit the borrow projection
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

        // check the wrapped pattern against the moved value
        self.check_pattern_projection(flow, scope, input, pattern.into_global_any(module))?;

        // commit the move projection
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

        // commit the dereference projection
        self.commit_pattern(
            node,
            dir::PatternDecision::Project(Box::new(dir::PatternProjectionResolution {
                projection: selection.into(),
                pattern: Some(pattern.into_global_any(module)),
            })),
        )
    }
}
