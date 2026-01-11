use destack_dir::{Expression, LocalNodeId};
use {destack_dir as dir, destack_mir as mir};

use crate::{LowerError, LowerResult, ScalarType};

use super::FunctionContext;

impl FunctionContext<'_> {
    /// Lower a cast operator into a MIR cast operator.
    pub(crate) fn lower_cast_operator(
        &self,
        expression_id: LocalNodeId<Expression>,
        operator: dir::CastOperator,
        value_id: LocalNodeId<Expression>,
    ) -> LowerResult<Option<mir::CastOperator>> {
        // map no op casts to None
        if operator == dir::CastOperator::Identity {
            return Ok(None);
        }

        // read the source scalar kind when needed
        let source_scalar_type = self.scalar_type_for_expression(value_id);

        // read the target scalar kind when needed
        let target_scalar_type = self.scalar_type_for_expression(expression_id);

        // map dir operators to mir operators
        let cast_operator = match operator {
            dir::CastOperator::IntWiden => match source_scalar_type {
                Some(ScalarType::SignedInt { .. }) => mir::CastOperator::SignExtend,
                Some(ScalarType::UnsignedInt { .. }) => mir::CastOperator::ZeroExtend,
                _ => {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.module_id)
                            .into_anchored(Some(self.profile)),
                        message: "unsupported int widen cast".to_string(),
                    })?;
                }
            },
            dir::CastOperator::IntNarrow => mir::CastOperator::Truncate,
            dir::CastOperator::IntSignChange => mir::CastOperator::Bitcast,
            dir::CastOperator::FloatWiden => mir::CastOperator::FloatExtend,
            dir::CastOperator::FloatNarrow => mir::CastOperator::FloatTruncate,
            dir::CastOperator::IntToFloat => match source_scalar_type {
                Some(ScalarType::SignedInt { .. }) => mir::CastOperator::SignedIntToFloat,
                Some(ScalarType::UnsignedInt { .. }) => mir::CastOperator::UnsignedIntToFloat,
                _ => {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.module_id)
                            .into_anchored(Some(self.profile)),
                        message: "unsupported int to float cast".to_string(),
                    })?;
                }
            },
            dir::CastOperator::FloatToInt => match target_scalar_type {
                Some(ScalarType::SignedInt { .. }) => mir::CastOperator::FloatToSignedInt,
                Some(ScalarType::UnsignedInt { .. }) => mir::CastOperator::FloatToUnsignedInt,
                _ => {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.module_id)
                            .into_anchored(Some(self.profile)),
                        message: "unsupported float to int cast".to_string(),
                    })?;
                }
            },
            dir::CastOperator::PointerToInt => mir::CastOperator::PointerToInt,
            dir::CastOperator::IntToPointer => mir::CastOperator::IntToPointer,
            dir::CastOperator::PointerCast => mir::CastOperator::Bitcast,
            _ => {
                return Err(LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                    message: format!("unsupported cast operator '{operator:?}'"),
                })?;
            }
        };

        Ok(Some(cast_operator))
    }
}
