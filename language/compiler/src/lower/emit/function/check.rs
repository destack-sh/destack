use destack_dir::{Expression, LocalNodeId};
use destack_mir as mir;
use destack_workspace::CheckFailurePolicy;

use crate::{LowerError, LowerResult, ScalarType};

use super::FunctionContext;

// check failure messages
const BOUNDS_CHECK_MESSAGE: &str = "bounds check failed";
const NULL_CHECK_MESSAGE: &str = "null check failed";

impl FunctionContext<'_> {
    /// Return true when overflow checks are enabled.
    pub(crate) fn overflow_checks_enabled(&self) -> bool {
        self.env.runtime_checks.overflow
    }

    /// Return true when bounds checks are enabled.
    pub(crate) fn bounds_checks_enabled(&self) -> bool {
        self.env.runtime_checks.bounds
    }

    /// Return true when null checks are enabled.
    pub(crate) fn null_checks_enabled(&self) -> bool {
        self.env.runtime_checks.null
    }

    /// Return true when division checks are enabled.
    pub(crate) fn division_checks_enabled(&self) -> bool {
        self.env.runtime_checks.division
    }

    /// Return true when shift checks are enabled.
    pub(crate) fn shift_checks_enabled(&self) -> bool {
        self.env.runtime_checks.shift
    }

    /// Emit a check terminator with a configured failure block.
    pub(crate) fn emit_check(
        &mut self,
        condition: mir::Value,
        constraint: mir::CheckConstraint,
        message: &'static str,
    ) {
        // create the failure block before sealing the check
        let failure_block = self.check_failure_block(message);

        // create the success block for fallthrough
        let success_block = self.state.builder.create_block();

        // insert the check terminator
        self.state
            .builder
            .check(condition, constraint, success_block, failure_block);

        // continue lowering in the success block
        self.state.builder.switch_to_block(success_block);
    }

    /// Resolve integer scalar info for an expression when available.
    pub(crate) fn integer_scalar_info(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> LowerResult<Option<(u16, bool)>> {
        // resolve scalar type for the expression
        let scalar_type = self
            .scalar_type_for_expression(expression_id)
            .ok_or_else(|| self.missing_type_error(expression_id))?;

        // return integer width and signedness when applicable
        let info = match scalar_type {
            ScalarType::SignedInt { width } => Some((width, true)),
            ScalarType::UnsignedInt { width } => Some((width, false)),
            _ => None,
        };

        Ok(info)
    }

    /// Emit a bounds check for an array access when enabled.
    pub(crate) fn emit_bounds_check(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        array_value: mir::Value,
        array_type: mir::LocalNodeId<mir::Type>,
        index_value: mir::Value,
        index_expression_id: LocalNodeId<Expression>,
    ) -> LowerResult<()> {
        // skip when bounds checks are disabled
        if !self.bounds_checks_enabled() {
            return Ok(());
        }

        // unwrap reference types to get the array payload
        let mut array_type = array_type;
        loop {
            let mir_type = self.state.builder.tree().get(array_type);
            let mir::Type::Reference { pointee, .. } = mir_type else {
                break;
            };
            array_type = *pointee;
        }

        // resolve the array length
        let mir::Type::Array { length, .. } = self.state.builder.tree().get(array_type) else {
            return Err(LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                message: "bounds checks require a sized array type".to_string(),
            });
        };

        // resolve the index scalar info
        let Some((width, is_signed)) = self.integer_scalar_info(index_expression_id)? else {
            return Err(LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                message: "bounds checks require an integer index".to_string(),
            });
        };

        // encode the length constant
        let length_value =
            i64::try_from(*length).map_err(|_| LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                message: "array length exceeds bounds check limits".to_string(),
            })?;
        let width = u8::try_from(width).map_err(|_| LowerError::UnsupportedConstruct {
            node: expression_id
                .into_global_any(self.env.module_id)
                .into_anchored(Some(self.env.profile)),
            message: "index width exceeds bounds check limits".to_string(),
        })?;
        let length_const = self.state.builder.iconst(length_value, width, is_signed);

        // build the bounds check condition
        let condition = if is_signed {
            let zero = self.state.builder.iconst(0, width, true);
            let non_negative = self.state.builder.binary_op(
                mir::BinaryOperator::SignedGreaterEqual,
                index_value,
                zero,
            );
            let in_range = self.state.builder.binary_op(
                mir::BinaryOperator::SignedLessThan,
                index_value,
                length_const,
            );
            self.state
                .builder
                .binary_op(mir::BinaryOperator::And, non_negative, in_range)
        } else {
            self.state.builder.binary_op(
                mir::BinaryOperator::UnsignedLessThan,
                index_value,
                length_const,
            )
        };

        // emit the bounds check
        let constraint = mir::CheckConstraint::Bounds {
            index: index_value,
            length: length_const,
            collection: array_value,
            is_signed,
        };
        self.emit_check(condition, constraint, BOUNDS_CHECK_MESSAGE);

        Ok(())
    }

    /// Emit a null check for a nullable reference when enabled.
    pub(crate) fn emit_null_check(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        value: mir::Value,
        value_type: mir::LocalNodeId<mir::Type>,
    ) -> LowerResult<()> {
        // skip when null checks are disabled
        if !self.null_checks_enabled() {
            return Ok(());
        }

        // skip non nullable references
        let mir::Type::Reference { is_nullable, .. } = self.state.builder.tree().get(value_type)
        else {
            return Ok(());
        };
        if !*is_nullable {
            return Ok(());
        }

        // convert to integer for null comparison
        let pointer_bits = u16::from(self.env.type_lowerer.pointer_bytes()) * 8;
        let pointer_bits =
            u8::try_from(pointer_bits).map_err(|_| LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                message: "pointer width exceeds null check limits".to_string(),
            })?;
        let pointer_value = self.state.builder.cast(
            mir::CastOperator::PointerToInt,
            value,
            self.env.type_lowerer.ty_usize,
        );
        let zero = self.state.builder.iconst(0, pointer_bits, false);
        let condition =
            self.state
                .builder
                .binary_op(mir::BinaryOperator::NotEqual, pointer_value, zero);

        // emit the null check
        let constraint = mir::CheckConstraint::Null { value };
        self.emit_check(condition, constraint, NULL_CHECK_MESSAGE);

        Ok(())
    }

    /// Create a failure block that respects the target check policy.
    fn check_failure_block(&mut self, message: &'static str) -> mir::LocalNodeId<mir::Block> {
        // preserve the current insertion point
        let current_block = self.state.builder.current_block();

        // create a dedicated failure block
        let failure_block = self.state.builder.create_block();
        self.state.builder.switch_to_block(failure_block);

        // emit the configured failure behavior
        match self.env.runtime_checks.failure {
            CheckFailurePolicy::Trap => {}
            CheckFailurePolicy::Abort => {
                self.state
                    .builder
                    .intrinsic_void(mir::Intrinsic::Abort, Vec::new());
            }
            CheckFailurePolicy::Panic => {
                let message_value = self.state.builder.sconst(message);
                self.state
                    .builder
                    .intrinsic_void(mir::Intrinsic::Panic, vec![message_value]);
            }
        }

        // mark the block as unreachable
        let block = self.state.builder.current_block();
        let block_data = self.state.builder.tree_mut().get_mut(block);
        block_data.terminator = mir::Terminator::Unreachable;

        // restore the previous insertion point
        self.state.builder.switch_to_block(current_block);

        failure_block
    }
}
