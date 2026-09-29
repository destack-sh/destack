use tspp_dir as dir;

use crate::sema::derive::Derivation;
use crate::sema::{CheckState, Origin};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Build `this(a, b)` or `new this(a, b)` over the member's parameters.
    pub(super) fn build_invoke_body(
        &mut self,
        frame: &mut Derivation,
    ) -> CompilerResult<dir::LocalNodeId<dir::Expression>> {
        // pass every parameter on in declaration order
        let callee = self.build_this(frame)?;
        let mut arguments = Vec::with_capacity(frame.parameters.len());
        let mut sources = Vec::with_capacity(frame.parameters.len());
        for index in 0..frame.parameters.len() {
            let value = self.build_parameter(frame, index)?;
            arguments.push(self.build_node(
                frame.module,
                frame.span,
                dir::Argument::Positional { value },
            ));
            sources.push(dir::ArgumentSource::Provided(
                value.into_global_any(frame.module),
            ));
        }

        // bind the arguments against the receiver's own signature
        let origin = Origin::Node(callee.into_global_any(frame.module), None);
        let Some((ty, signature)) = self.callable_signature_type(origin, frame.receiver)? else {
            return Err(CompilerError::Internal {
                message: "an invoked receiver without a signature".to_owned(),
            });
        };
        let parameters = self.signature_parameters(ty.module_id, signature.parameters)?;
        let bound = self
            .bind_arguments(origin, parameters, &sources)?
            .ok_or_else(|| CompilerError::Internal {
                message: "an invoked receiver rejects its forwarded parameters".to_owned(),
            })?;
        let call = dir::Call {
            target: dir::CallableTarget::Expression {
                generic_arguments: Vec::new(),
            },
            callable_type: frame.receiver,
            arguments: bound,
            return_type: frame.result,
            regions: Vec::new(),
        };

        // invoke the receiver by role
        let expression = match frame.member {
            dir::StaticKey::New => dir::Expression::New {
                left: callee,
                generic_arguments: Vec::new(),
                arguments,
            },
            dir::StaticKey::Call => dir::Expression::Call {
                position: dir::PostfixPosition::Direct,
                left: callee,
                generic_arguments: Vec::new(),
                arguments,
                is_optional: false,
            },
            dir::StaticKey::Name(_) | dir::StaticKey::Index(_) => {
                return Err(CompilerError::Internal {
                    message: "an invoked member under a non-role key".to_owned(),
                });
            }
        };
        let node = self.build_expression(frame, expression, frame.result)?;
        self.commit_decision(
            node.into_global_any(frame.module),
            dir::Decision::Call(dir::OperationResolution::One(call)),
        )?;

        Ok(node)
    }
}
