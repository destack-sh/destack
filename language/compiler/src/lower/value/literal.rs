use destack_dir::{Expression, LocalNodeId, ScalarLiteral};
use destack_mir as mir;

use crate::{LowerError, LowerResult};

use super::super::{BlockLowerer, ScalarType};

impl BlockLowerer<'_, '_> {
    /// Lower a scalar literal expression.
    pub(crate) fn lower_scalar_literal(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        value: &ScalarLiteral,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        match value {
            ScalarLiteral::Boolean(value) => {
                let value = self.builder.bconst(*value);
                Ok((value, self.type_lowerer.ty_bool))
            }
            ScalarLiteral::Integer(value) => {
                match self.scalar_type_for_expression(expression_id) {
                    Some(ScalarType::Float { width }) => {
                        let value = self.builder.fconst(*value as f64, width as u8);
                        let ty = if width == 32 {
                            self.type_lowerer.ty_f32
                        } else {
                            self.type_lowerer.ty_f64
                        };
                        Ok((value, ty))
                    }
                    Some(ScalarType::SignedInt { width }) => {
                        let value = self.builder.iconst(*value, width as u8, true);
                        let ty = if width == 64 {
                            self.type_lowerer.ty_i64
                        } else {
                            self.type_lowerer.ty_i32
                        };
                        Ok((value, ty))
                    }
                    _ => Err(LowerError::UnsupportedConstruct {
                        node: expression_id.into_global_any(self.module_id),
                        message: format!("unsupported scalar literal '{value:?}'"),
                    })?,
                }
            }
            ScalarLiteral::Float(value) => {
                let width = match self.scalar_type_for_expression(expression_id) {
                    Some(ScalarType::Float { width }) => width,
                    _ => 64,
                };
                let value = self.builder.fconst(*value, width as u8);
                let ty = if width == 32 {
                    self.type_lowerer.ty_f32
                } else {
                    self.type_lowerer.ty_f64
                };
                Ok((value, ty))
            }
            _ => Err(LowerError::UnsupportedConstruct {
                node: expression_id.into_global_any(self.module_id),
                message: format!("unsupported scalar literal '{value:?}'"),
            })?,
        }
    }
}
