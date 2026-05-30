use {destack_dir as dir, destack_mir as mir};

use crate::{CompilerError, CompilerResult, LowerError, ScalarType};

use crate::lower::FunctionLowerer;

impl FunctionLowerer<'_> {
    /// Lower a scalar literal expression.
    ///
    /// ```ds
    /// function one(): int32 {
    ///     return 1;
    /// }
    /// ```
    /// ->
    /// ```mir
    /// v0: int32 = const 1
    /// ```
    pub(crate) fn lower_scalar_literal(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        value: &dir::ScalarLiteral,
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        match value {
            dir::ScalarLiteral::Boolean(value) => {
                let value = self.state.builder.bconst(*value);
                Ok((value, self.context.type_lowerer.ty_bool))
            }
            dir::ScalarLiteral::Integer(value) => {
                match self.scalar_type_for_expression(expression_id) {
                    Some(ScalarType::Float { format }) => {
                        let value = self.state.builder.fconst(*value as f64, format);
                        let ty = self.context.type_lowerer.type_for_float_format(format);
                        Ok((value, ty))
                    }
                    Some(ScalarType::SignedInt { width }) => {
                        let value = self.state.builder.iconst(i128::from(*value), width, true);
                        let ty = if width == 64 {
                            self.context.type_lowerer.ty_i64
                        } else {
                            self.context.type_lowerer.ty_i32
                        };
                        Ok((value, ty))
                    }
                    _ => Err(CompilerError::from(LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(
                            expression_id
                                .into_global_any(self.context.module_id)
                                .into_anchored(Some(self.context.profile)),
                        ),
                        message: format!("unsupported scalar literal '{value:?}'"),
                    }))?,
                }
            }
            dir::ScalarLiteral::Float(value) => {
                let format = match self.scalar_type_for_expression(expression_id) {
                    Some(ScalarType::Float { format }) => format,
                    _ => mir::FloatType::Float64,
                };
                let value = self.state.builder.fconst(*value, format);
                let ty = self.context.type_lowerer.type_for_float_format(format);
                Ok((value, ty))
            }
            dir::ScalarLiteral::String(value) => {
                let anchor = expression_id
                    .into_global_any(self.context.module_id)
                    .into_anchored(Some(self.context.profile));
                let (value, fallback_ty) =
                    self.string_literal_value_for_id(*value, Some(anchor))?;
                let ty = match self.lower_type_for_expression(expression_id) {
                    Ok(ty) => ty,
                    Err(_) => fallback_ty,
                };
                Ok((value, ty))
            }
            _ => Err(CompilerError::from(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
                message: format!("unsupported scalar literal '{value:?}'"),
            }))?,
        }
    }
}
