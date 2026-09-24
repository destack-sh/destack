use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::lower::function::place::PlaceProjection;
use crate::lower::function::union::UnionDispatch;
use crate::{CompilerError, CompilerResult};

/// One value store dispatched over a union member's arms.
pub(in crate::lower) enum UnionMemberStore {
    /// Store the assigned value.
    Assign(mir::Value),
    /// Combine the current value with the assigned value, then store.
    Compound(mir::BinaryOperator, mir::Value),
    /// Step the current value by one, then store.
    Update(mir::BinaryOperator),
}

impl FunctionLowerer<'_, '_, '_> {
    /// Lower one assignment statement.
    pub(in crate::lower) fn lower_assign(
        &mut self,
        statement: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::AssignPattern>,
        operator: dir::AssignOperator,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<mir::Value>> {
        // require a place assignment
        let dir::AssignPatternDecision::Place = self.assign_resolution(left)? else {
            return Err(self.unsupported("a destructuring assignment"));
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

            self.lower_accessor_write(expression, operator, &call, right)?;

            Ok(None)
        }
        // write through the index set protocol call when one is selected
        else if let dir::WriteResolution::Subscript(dir::OperationResolution::One(subscript)) =
            &resolution.write
            && let dir::SubscriptTarget::Call(call) = &subscript.target
        {
            let call = call.clone();

            self.lower_subscript_write(expression, operator, &call, right)?;

            Ok(None)
        }
        // dispatch a union member write over its recorded arms
        else if let dir::WriteResolution::Member(member) = &resolution.write
            && !matches!(member, dir::OperationResolution::One(_))
        {
            let arms = member.arms().to_vec();
            let value = self.lower_value(right)?;
            let store = match operator {
                dir::AssignOperator::Assign => UnionMemberStore::Assign(value),
                _ => {
                    let resolution = self.operator_decision(statement)?;
                    let dir::OperationResolution::One(dir::OperatorApplication::Binary {
                        operator,
                        target: dir::OperatorTarget::Builtin(_),
                        ..
                    }) = resolution
                    else {
                        return Err(self.unsupported("a protocol compound assignment"));
                    };

                    UnionMemberStore::Compound(self.binary_operator(operator)?, value)
                }
            };

            self.lower_union_member_write(expression, store, &arms)?;

            Ok(None)
        }
        // store the right value directly for plain assignment
        else if operator == dir::AssignOperator::Assign {
            let place = self.place(&resolution)?;
            let value = self.lower_value(right)?;
            self.write_place(&place, value)?;

            Ok(Some(value))
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
                return Err(self.unsupported("a protocol compound assignment"));
            };

            // read, combine, and write the place back
            let current = self.read_place(&place)?;
            let operator = self.binary_operator(operator)?;
            let value = self.lower_value(right)?;
            let value = self.builder.binary(operator, current, value);
            self.write_place(&place, value)?;

            Ok(Some(value))
        }
    }

    /// Lower one member write through its union receiver's recorded arms.
    pub(in crate::lower) fn lower_union_member_write(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        store: UnionMemberStore,
        arms: &[dir::MemberAccess],
    ) -> CompilerResult<()> {
        // route the arms and walk the shared steps onto the union storage place
        let UnionDispatch {
            chains,
            prefix,
            targets,
        } = self.union_dispatch(arms)?;
        let dir::Expression::Member { left, .. } = *self.source().tree().get(expression) else {
            return Err(CompilerError::Internal {
                message: "a union member write outside a member place".to_string(),
            });
        };
        let place = self.receiver_place(left)?;
        let place = self.project_place_adjustments(place, &chains[0][..prefix])?;

        // dispatch on the union discriminant at its place
        self.switch_place(&place, None, targets.clone())?;

        // store through each arm's downcast place and rejoin
        let exit = self.builder.block();
        for ((arm, chain), &(case, block)) in arms.iter().zip(&chains).zip(&targets) {
            self.builder.switch_to_block(block);

            // project the dispatched case and the arm's remaining steps
            let payload = self.lower_type(chain[prefix].ty())?;
            let mut arm_place = place.clone();
            arm_place
                .path
                .push(PlaceProjection::Downcast { case, ty: payload });
            let mut arm_place = self.project_place_adjustments(arm_place, &chain[prefix + 1..])?;

            // project the written field on this arm
            let dir::MemberTarget::Field(field) = &arm.target else {
                return Err(self.unsupported("a union member write outside field storage"));
            };
            let index = self.member_field_index(field)?;
            let field_type = self.lower_type(field.ty)?;
            arm_place.path.push(PlaceProjection::Field {
                field: index,
                ty: field_type,
            });

            // combine with the current value for compound and update stores
            let stored = match store {
                UnionMemberStore::Assign(value) => value,
                UnionMemberStore::Compound(operator, value) => {
                    let current = self.read_place(&arm_place)?;

                    self.builder.binary(operator, current, value)
                }
                UnionMemberStore::Update(operator) => {
                    let current = self.read_place(&arm_place)?;
                    let one = self.one_value(field_type)?;

                    self.builder.binary(operator, current, one)
                }
            };

            // write the stored value back and join the other arms
            self.write_place(&arm_place, stored)?;
            self.builder.jump(exit);
        }

        // continue lowering at the exit block
        self.builder.switch_to_block(exit);

        Ok(())
    }

    /// Lower one index assignment through its selected index set call.
    fn lower_subscript_write(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        operator: dir::AssignOperator,
        call: &dir::Call,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        // reject a compound assignment through an index set
        if operator != dir::AssignOperator::Assign {
            return Err(self.unsupported("a compound assignment through an index set"));
        }

        // require a statically selected index set
        let dir::CallableTarget::Symbol {
            function,
            dispatch: dir::FunctionDispatch::Direct,
        } = &call.target
        else {
            return Err(self.unsupported("a dynamic index write"));
        };

        // read the receiver the index set is called on
        let dir::Expression::Index { left, .. } = *self.source().tree().get(expression) else {
            return Err(CompilerError::Internal {
                message: "an index write outside an index place".to_string(),
            });
        };

        // call the index set with the assigned value
        self.lower_function_target_call(left, call, function, Some(right), false)?;

        Ok(())
    }

    /// Lower one assignment through a selected setter call.
    fn lower_accessor_write(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        operator: dir::AssignOperator,
        call: &dir::Call,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        // reject a compound assignment through an accessor
        if operator != dir::AssignOperator::Assign {
            return Err(self.unsupported("a compound assignment through an accessor"));
        }

        // require a statically selected setter
        let dir::CallableTarget::Symbol {
            function,
            dispatch: dir::FunctionDispatch::Direct,
        } = &call.target
        else {
            return Err(self.unsupported("a dynamic property write"));
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
