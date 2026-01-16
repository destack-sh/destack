use destack_dir::{Expression, LocalNodeId, ScalarLiteral};
use destack_mir as mir;

use crate::{LowerError, LowerResult, ScalarType};

use crate::lower::emit::FunctionContext;

impl FunctionContext<'_> {
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
            ScalarLiteral::Integer(value) => match self.scalar_type_for_expression(expression_id) {
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
                    node: expression_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                    message: format!("unsupported scalar literal '{value:?}'"),
                })?,
            },
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
            ScalarLiteral::String(value) => {
                let literal = self.strings.get(*value);
                let value = self.builder.sconst(literal.to_string());
                let ty = self.mir_type_for_expression(expression_id).or_else(|_| {
                    self.type_lowerer
                        .string_type()
                        .ok_or(LowerError::UnsupportedConstruct {
                            node: expression_id
                                .into_global_any(self.module_id)
                                .into_anchored(Some(self.profile)),
                            message: "missing builtin String layout (load lib/native)".to_string(),
                        })
                })?;
                Ok((value, ty))
            }
            _ => Err(LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.module_id)
                    .into_anchored(Some(self.profile)),
                message: format!("unsupported scalar literal '{value:?}'"),
            })?,
        }
    }
}
