use destack_workspace::CheckFailurePolicy;
use {destack_dir as dir, destack_mir as mir};

use crate::{LowerError, LowerResult, ScalarType};

use super::{FunctionLowerer, RUNTIME_CHECK_MESSAGES};

impl FunctionLowerer<'_> {
    /// Return true when overflow checks are enabled.
    pub(crate) fn overflow_checks_enabled(&self) -> bool {
        self.context.checks.overflow
    }

    /// Return true when bounds checks are enabled.
    pub(crate) fn bounds_checks_enabled(&self) -> bool {
        self.context.checks.bounds
    }

    /// Return true when null checks are enabled.
    pub(crate) fn null_checks_enabled(&self) -> bool {
        self.context.checks.null
    }

    /// Return true when division checks are enabled.
    pub(crate) fn division_checks_enabled(&self) -> bool {
        self.context.checks.division
    }

    /// Return true when shift checks are enabled.
    pub(crate) fn shift_checks_enabled(&self) -> bool {
        self.context.checks.shift
    }

    /// Emit a check terminator with a configured failure block.
    pub(crate) fn emit_check(
        &mut self,
        constraint: mir::CheckConstraint,
        message: &'static str,
    ) -> LowerResult<()> {
        // create the failure block before sealing the check
        let failure_block = self.check_failure_block(message)?;

        // create the success block for fallthrough
        let success_block = self.state.builder.block();

        // insert the check terminator
        self.state
            .builder
            .check(constraint, success_block, failure_block);

        // continue lowering in the success block
        self.state.builder.switch_to_block(success_block);

        Ok(())
    }

    /// Resolve integer scalar info for an expression when available.
    pub(crate) fn integer_scalar_info(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
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
        expression_id: dir::LocalNodeId<dir::Expression>,
        array_value: mir::Value,
        array_type: mir::LocalNodeId<mir::Type>,
        index_value: mir::Value,
        index_expression_id: dir::LocalNodeId<dir::Expression>,
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
            array_type = pointee.ty().ok_or_else(|| LowerError::Internal {
                module: self.context.module_id,
                message: "bounds check array reference type must be concrete".to_string(),
            })?;
        }

        // resolve the array length
        let mir::Type::Array { length, .. } = self.state.builder.tree().get(array_type) else {
            return Err(LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.context.module_id)
                    .into_anchored(Some(self.context.profile)),
                message: "bounds checks require a sized array type".to_string(),
            });
        };

        // resolve the index scalar info
        let Some((width, is_signed)) = self.integer_scalar_info(index_expression_id)? else {
            return Err(LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.context.module_id)
                    .into_anchored(Some(self.context.profile)),
                message: "bounds checks require an integer index".to_string(),
            });
        };

        // encode the length constant
        let length_value =
            i64::try_from(*length).map_err(|_| LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.context.module_id)
                    .into_anchored(Some(self.context.profile)),
                message: "array length exceeds bounds check limits".to_string(),
            })?;
        let width = u8::try_from(width).map_err(|_| LowerError::UnsupportedConstruct {
            node: expression_id
                .into_global_any(self.context.module_id)
                .into_anchored(Some(self.context.profile)),
            message: "index width exceeds bounds check limits".to_string(),
        })?;
        let length_const = self.state.builder.iconst(length_value, width, is_signed);

        // emit the bounds check
        let constraint = mir::CheckConstraint::Bounds {
            index: index_value.into(),
            length: length_const.into(),
            collection: array_value.into(),
            is_signed,
        };
        self.emit_check(constraint, RUNTIME_CHECK_MESSAGES.bounds_check)?;

        Ok(())
    }

    /// Emit a null check for a nullable reference when enabled.
    pub(crate) fn emit_null_check(
        &mut self,
        _expression_id: dir::LocalNodeId<dir::Expression>,
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

        // emit the null check
        let constraint = mir::CheckConstraint::Null {
            value: value.into(),
        };
        self.emit_check(constraint, RUNTIME_CHECK_MESSAGES.null_check)?;

        Ok(())
    }

    /// Create a failure block that respects the target check policy.
    fn check_failure_block(
        &mut self,
        message: &'static str,
    ) -> LowerResult<mir::LocalNodeId<mir::Block>> {
        // preserve the current insertion point
        let current_block = self.state.builder.current_block();

        // create a dedicated failure block
        let failure_block = self.state.builder.block();
        self.state.builder.switch_to_block(failure_block);

        // emit the configured failure behavior
        match self.context.checks.failure {
            CheckFailurePolicy::Trap | CheckFailurePolicy::Abort => {
                self.state.builder.trap_abort();
            }
            CheckFailurePolicy::Panic => {
                let (message_value, _) = self.string_literal_value(message)?;
                self.state.builder.trap_panic(message_value);
            }
        }

        // restore the previous insertion point
        self.state.builder.switch_to_block(current_block);

        Ok(failure_block)
    }
}
