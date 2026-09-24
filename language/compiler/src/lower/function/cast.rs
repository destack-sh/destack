use destack_dir as dir;
use destack_mir as mir;

use crate::CompilerResult;
use crate::lower::FunctionLowerer;

impl FunctionLowerer<'_, '_, '_> {
    /// Lower one explicit cast expression.
    pub(in crate::lower) fn lower_as(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        value: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::Value> {
        // materialize a literal source directly at the cast target
        if let dir::Type::Literal(literal) = self.node_type(value)?
            && self.coercion(value).is_none()
        {
            let target = self.lower_type(self.node_type_id(expression)?)?;

            return self.lower_constant(literal, target);
        }

        // apply the adjustment sema recorded on the operand
        self.lower_value(value)
    }

    /// Select the conversion between two concrete scalar representations.
    pub(in crate::lower) fn cast_operator(
        &self,
        source: &mir::Type,
        target: &mir::Type,
    ) -> CompilerResult<mir::CastOperator> {
        let pointer_bits = self.builder.pointer_bits();
        let source_integer = source.integer(pointer_bits).is_some();
        let target_integer = target.integer(pointer_bits).is_some();
        let source_float = matches!(source, mir::Type::Float(_));
        let target_float = matches!(target, mir::Type::Float(_));
        let operator = match (source_integer, source_float, target_integer, target_float) {
            (true, _, true, _) => mir::CastOperator::IntToInt,
            (true, _, _, true) => mir::CastOperator::IntToFloat,
            (_, true, true, _) => mir::CastOperator::FloatToInt,
            (_, true, _, true) => mir::CastOperator::FloatToFloat,
            _ => {
                let source = self.scalar_name(source);
                let target = self.scalar_name(target);

                return Err(self.internal(format!("a cast from {source} to {target}")));
            }
        };

        Ok(operator)
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

        // run every remaining const expression for its effects, its value living in its type
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
