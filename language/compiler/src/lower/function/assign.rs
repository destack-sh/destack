use destack_dir as dir;

use crate::lower::FunctionLowerer;
use crate::{CompilerError, CompilerResult, LowerError};

impl FunctionLowerer<'_, '_, '_> {
    /// Lower one assignment statement.
    pub(in crate::lower) fn lower_assign(
        &mut self,
        statement: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::AssignPattern>,
        operator: dir::AssignOperator,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        // require a place assignment
        let dir::AssignPatternDecision::Place = self.assign_resolution(left)? else {
            return Err(LowerError::Unsupported {
                anchor: self.lower.module.into(),
                construct: "a destructuring assignment".to_string(),
            }
            .into());
        };
        let dir::AssignPattern::Place { expression } = *self.source().tree().get(left) else {
            return Err(CompilerError::Internal {
                message: "a non-place pattern resolved as a place".to_string(),
            });
        };

        // resolve how the place is written
        let resolution = self.assignment_decision(expression)?;

        // write through the setter member when one is selected
        if let dir::WriteResolution::Member(dir::OperationResolution::One(access)) =
            &resolution.write
            && let dir::MemberTarget::Call(call) = &access.target
        {
            let call = call.clone();

            self.lower_accessor_write(expression, operator, &call, right)
        }
        // store the right value directly for plain assignment
        else if operator == dir::AssignOperator::Assign {
            let place = self.place(&resolution)?;
            let value = self.lower_expression(right)?;
            self.write_place(&place, value)?;

            Ok(())
        }
        // apply the builtin operation for compound assignment
        else {
            let place = self.place(&resolution)?;
            let resolution = self.operator_decision(statement)?;
            let dir::OperationResolution::One(dir::OperatorApplication::Binary {
                operator,
                target: dir::OperatorTarget::Builtin(_),
                ..
            }) = resolution
            else {
                return Err(LowerError::Unsupported {
                    anchor: self.lower.module.into(),
                    construct: "a protocol compound assignment".to_string(),
                }
                .into());
            };

            // read, combine, and write the place back
            let current = self.read_place(&place)?;
            let operator = self.binary_operator(operator)?;
            let value = self.lower_expression(right)?;
            let value = self.builder.binary(operator, current, value);
            self.write_place(&place, value)?;

            Ok(())
        }
    }

    /// Lower one assignment into a setter member through its call.
    fn lower_accessor_write(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        operator: dir::AssignOperator,
        call: &dir::Call,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        // reject a compound assignment through an accessor
        if operator != dir::AssignOperator::Assign {
            return Err(LowerError::Unsupported {
                anchor: self.lower.module.into(),
                construct: "a compound assignment through an accessor".to_string(),
            }
            .into());
        }

        // require a statically selected setter
        let dir::CallableTarget::Symbol {
            function,
            dispatch: dir::FunctionDispatch::Direct,
        } = &call.target
        else {
            return Err(LowerError::Unsupported {
                anchor: self.lower.module.into(),
                construct: "a dynamic property write".to_string(),
            }
            .into());
        };

        // read the receiver the setter is called on
        let dir::Expression::Member { left, .. } = *self.source().tree().get(expression) else {
            return Err(CompilerError::Internal {
                message: "an accessor write outside a member place".to_string(),
            });
        };

        // call the setter with the assigned value
        self.lower_function_target_call(left, call, function, Some(right), false)?;

        Ok(())
    }
}
