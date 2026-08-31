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
        // read the scalar representation the cast targets
        let target = self.node_type(expression)?;
        let target = self.lower.scalar_type(&target)?;

        // materialize literal sources directly at the cast target
        if let dir::Type::Literal(literal) = self.node_type(value)? {
            return self.lower_constant(literal, target);
        }

        // hand the value through unchanged when it already carries the target
        let source = self.node_type(value)?;
        let source = self.lower.scalar_type(&source)?;
        let lowered = self.lower_expression(value)?;
        if source == target {
            return Ok(lowered);
        }

        // convert between the two scalar representations
        let operator = self.cast_operator(&source, &target)?;
        let target = self.builder.tree_mut().intern_type(target);

        Ok(self.builder.cast(operator, lowered, target))
    }

    /// Select the conversion between two concrete scalar representations.
    pub(in crate::lower) fn cast_operator(
        &self,
        source: &mir::Type,
        target: &mir::Type,
    ) -> CompilerResult<mir::CastOperator> {
        // resolve pointer-sized representations to their concrete widths
        let pointer_bits = self.builder.pointer_bits();
        let concrete = |ty: &mir::Type| match *ty {
            mir::Type::Isize => mir::Type::Int {
                width: pointer_bits,
                is_signed: true,
            },
            mir::Type::Usize => mir::Type::Int {
                width: pointer_bits,
                is_signed: false,
            },
            ref other => other.clone(),
        };
        let source = &concrete(source);
        let target = &concrete(target);

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

            // reject every other conversion
            (source, target) => {
                let source = self.scalar_name(source);
                let target = self.scalar_name(target);

                return Err(LowerError::Unsupported {
                    anchor: self.lower.module.into(),
                    construct: format!("a cast from {source} to {target}"),
                }
                .into());
            }
        })
    }

    /// Name one concrete scalar representation for diagnostics.
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
            _ => "a non-scalar representation".to_string(),
        }
    }

    /// Lower one expression whose value lives in its type.
    pub(in crate::lower) fn lower_const_expression(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        // skip the expressions already folded into their committed type
        if self.is_folded_expression(expression)? {
            return Ok(());
        }

        // evaluate every remaining const value outside its coercion
        self.lower_expression_value(expression)?;

        Ok(())
    }

    /// Return whether one expression's computation folded into its committed type.
    fn is_folded_expression(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<bool> {
        match self.source().tree().get(expression) {
            // fold literal spellings directly
            dir::Expression::Literal(_) => Ok(true),

            // read the fold recorded at operator selection
            dir::Expression::Binary { .. } | dir::Expression::Unary { .. } => {
                Ok(self.operator_decision(expression)?.is_folded())
            }

            // leave every other expression to run
            _ => Ok(false),
        }
    }
}
