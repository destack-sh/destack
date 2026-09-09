use destack_dir as dir;

use crate::sema::CheckState;
use crate::sema::derive::Derivation;
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Build `*this` reading the managed reference a class receiver holds.
    fn build_identity(
        &mut self,
        frame: &mut Derivation,
        value: dir::LocalNodeId<dir::Expression>,
        value_type: dir::GlobalTypeId,
    ) -> CompilerResult<dir::LocalNodeId<dir::Expression>> {
        // read the handle beneath a borrow, a handle itself standing as is
        let dir::Type::Form(form) = self.ty(value_type)? else {
            return Ok(value);
        };
        if !matches!(form.form, dir::Form::Borrowed(_) | dir::Form::Readonly) {
            return Ok(value);
        }

        // dereference to the handle and record the builtin operator
        let handle = form.value;
        let node = self.build_expression(
            frame,
            dir::Expression::Unary {
                operator: dir::UnaryOperator::Dereference,
                right: value,
            },
            handle,
        )?;
        self.commit_decision(
            node.into_global_any(frame.module),
            dir::Decision::Operator(dir::OperationResolution::One(
                dir::OperatorApplication::Unary {
                    operator: dir::UnaryOperator::Dereference,
                    target: dir::OperatorTarget::Builtin(dir::BuiltinOperand {
                        source: value.into_global(frame.module),
                        ty: value_type,
                        scalar_families: None,
                    }),
                    ty: handle,
                    is_folded: false,
                },
            )),
        )?;

        Ok(node)
    }

    /// Build `*this === *other` equating class instances by managed identity.
    pub(super) fn build_identity_equal_body(
        &mut self,
        frame: &mut Derivation,
    ) -> CompilerResult<dir::LocalNodeId<dir::Expression>> {
        // require the frame's receiver
        let boolean = self.intern_type(dir::Type::Primitive(dir::PrimitiveType::Boolean))?;
        let Some(this_type) = frame.this else {
            return Err(CompilerError::Internal {
                message: "a derived static body reading this".to_owned(),
            });
        };

        // read the handle behind the receiver and behind the peer
        let (_, _, peer_type) = frame.parameters[0];
        let this = self.build_this(frame)?;
        let left = self.build_identity(frame, this, this_type)?;
        let left_type = self.require_node_type(left.into_global_any(frame.module))?;
        let peer = self.build_parameter(frame, 0)?;
        let right = self.build_identity(frame, peer, peer_type)?;
        let right_type = self.require_node_type(right.into_global_any(frame.module))?;

        // compare the two handles by identity
        let node = self.build_builtin_binary(
            frame,
            dir::BinaryOperator::EqualStrict,
            (left, left_type),
            (right, right_type),
            boolean,
        )?;

        self.build_block(frame, Vec::new(), Some(node), boolean)
    }
}
