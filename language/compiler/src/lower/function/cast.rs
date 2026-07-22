use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::{CompilerResult, LowerError};

impl FunctionLowerer<'_, '_, '_> {
    /// Lower one explicit cast expression.
    pub(in crate::lower) fn lower_as(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        value: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::Value> {
        let target = self.lowerer.node_type(expression)?;
        let target = self.lowerer.lower_type(&target)?;

        // materialize literal sources directly at the cast target
        if let dir::Type::Literal(literal) = self.lowerer.node_type(value)? {
            return self.lower_constant(literal, target);
        }

        let source = self.lowerer.coerced_type(value)?;
        let source = self.lowerer.lower_type(&source)?;
        let lowered = self.lower_expression(value)?;
        if source == target {
            return Ok(lowered);
        }

        let operator = self.cast_operator(&source, &target)?;
        let target = self.builder.tree_mut().insert(target);

        Ok(self.builder.cast(operator, lowered, target))
    }

    /// Select the MIR conversion between two concrete scalar carriers.
    pub(in crate::lower) fn cast_operator(
        &self,
        source: &mir::Type,
        target: &mir::Type,
    ) -> CompilerResult<mir::CastOperator> {
        Ok(match (source, target) {
            // truncate, extend by source sign, or reinterpret between integers
            (
                mir::Type::Int {
                    width: from,
                    is_signed,
                },
                mir::Type::Int { width: to, .. },
            ) => match from.cmp(to) {
                std::cmp::Ordering::Greater => mir::CastOperator::Truncate,
                std::cmp::Ordering::Equal => mir::CastOperator::Bitcast,
                std::cmp::Ordering::Less => match is_signed {
                    true => mir::CastOperator::SignExtend,
                    false => mir::CastOperator::ZeroExtend,
                },
            },

            // convert integer to float by source sign
            (
                mir::Type::Int {
                    is_signed: true, ..
                },
                mir::Type::Float(_),
            ) => mir::CastOperator::SignedIntToFloat,
            (
                mir::Type::Int {
                    is_signed: false, ..
                },
                mir::Type::Float(_),
            ) => mir::CastOperator::UnsignedIntToFloat,

            // saturate float into the integer target range
            (
                mir::Type::Float(_),
                mir::Type::Int {
                    is_signed: true, ..
                },
            ) => mir::CastOperator::FloatToSignedIntSaturating,
            (
                mir::Type::Float(_),
                mir::Type::Int {
                    is_signed: false, ..
                },
            ) => mir::CastOperator::FloatToUnsignedIntSaturating,

            // truncate or extend between float widths
            (mir::Type::Float(from), mir::Type::Float(to)) => match from.width().cmp(&to.width()) {
                std::cmp::Ordering::Greater => mir::CastOperator::FloatTruncate,
                std::cmp::Ordering::Equal => mir::CastOperator::FloatConvert,
                std::cmp::Ordering::Less => mir::CastOperator::FloatExtend,
            },

            (source, target) => {
                let source = self.scalar_name(source);
                let target = self.scalar_name(target);

                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: format!("a cast from {source} to {target}"),
                }
                .into());
            }
        })
    }
    /// Name one concrete scalar carrier for diagnostics.
    fn scalar_name(&self, ty: &mir::Type) -> String {
        match ty {
            mir::Type::Void => "void".to_string(),
            mir::Type::Boolean => "boolean".to_string(),
            mir::Type::Isize => "isize".to_string(),
            mir::Type::Usize => "usize".to_string(),
            mir::Type::Int {
                width,
                is_signed: true,
            } => format!("int{width}"),
            mir::Type::Int {
                width,
                is_signed: false,
            } => format!("uint{width}"),
            mir::Type::Float(float) => float.label().to_string(),
            other => format!("{other:?}"),
        }
    }
}
