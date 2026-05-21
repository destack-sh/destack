use destack_dir as dir;

use crate::check::CheckModuleState;

impl CheckModuleState {
    /// Resolve a builtin binary operator once operand types are solved.
    pub(in crate::check) fn resolve_binary_operator_builtin(
        &mut self,
        operator: dir::BinaryOperator,
        left: dir::LocalTypeId,
        right: dir::LocalTypeId,
    ) -> Option<dir::CallResolution> {
        match operator {
            dir::BinaryOperator::Add => self.resolve_add_builtin(operator, left, right),
            dir::BinaryOperator::Subtract
            | dir::BinaryOperator::Multiply
            | dir::BinaryOperator::Divide
            | dir::BinaryOperator::Remainder
            | dir::BinaryOperator::Exponent => self.resolve_numeric_builtin(operator, left, right),
            dir::BinaryOperator::Equal
            | dir::BinaryOperator::NotEqual
            | dir::BinaryOperator::EqualStrict
            | dir::BinaryOperator::NotEqualStrict
            | dir::BinaryOperator::LessThan
            | dir::BinaryOperator::LessThanOrEqual
            | dir::BinaryOperator::GreaterThan
            | dir::BinaryOperator::GreaterThanOrEqual => {
                self.resolve_comparison_builtin(operator, left, right)
            }
            _ => None,
        }
    }

    /// Resolve builtin add.
    fn resolve_add_builtin(
        &mut self,
        operator: dir::BinaryOperator,
        left: dir::LocalTypeId,
        right: dir::LocalTypeId,
    ) -> Option<dir::CallResolution> {
        if self.is_string_like(left) || self.is_string_like(right) {
            let return_type = self.primitive_type_id(dir::PrimitiveType::String);

            return Some(builtin_resolution(
                operator,
                [
                    self.widen_builtin_operand_type(left),
                    self.widen_builtin_operand_type(right),
                ],
                return_type,
            ));
        }

        self.resolve_numeric_builtin(operator, left, right)
    }

    /// Resolve builtin numeric operators.
    fn resolve_numeric_builtin(
        &mut self,
        operator: dir::BinaryOperator,
        left: dir::LocalTypeId,
        right: dir::LocalTypeId,
    ) -> Option<dir::CallResolution> {
        if !self.is_numeric_like(left) || !self.is_numeric_like(right) {
            return None;
        }

        let left = self.widen_builtin_operand_type(left);
        let right = self.widen_builtin_operand_type(right);
        let return_type = self.join_numeric_builtin_type(left, right);

        Some(builtin_resolution(operator, [left, right], return_type))
    }

    /// Resolve builtin comparison operators.
    fn resolve_comparison_builtin(
        &mut self,
        operator: dir::BinaryOperator,
        left: dir::LocalTypeId,
        right: dir::LocalTypeId,
    ) -> Option<dir::CallResolution> {
        let return_type = self.primitive_type_id(dir::PrimitiveType::Boolean);

        Some(builtin_resolution(operator, [left, right], return_type))
    }

    /// Return the widened operand type used by builtin operators.
    fn widen_builtin_operand_type(&mut self, type_id: dir::LocalTypeId) -> dir::LocalTypeId {
        match self.get_type(type_id) {
            dir::Type::Literal(dir::ScalarLiteral::Integer(_)) => {
                self.primitive_type_id(dir::PrimitiveType::Integer(dir::IntegerType::Fixed {
                    width: 32,
                    is_signed: true,
                }))
            }
            dir::Type::Literal(dir::ScalarLiteral::Float(_)) => {
                self.primitive_type_id(dir::PrimitiveType::Float(dir::FloatType::Float64))
            }
            dir::Type::Literal(dir::ScalarLiteral::Bigint(_)) => {
                self.primitive_type_id(dir::PrimitiveType::Bigint)
            }
            dir::Type::Literal(dir::ScalarLiteral::String(_)) => {
                self.primitive_type_id(dir::PrimitiveType::String)
            }
            _ => type_id,
        }
    }

    /// Return the builtin numeric result type.
    fn join_numeric_builtin_type(
        &mut self,
        left: dir::LocalTypeId,
        right: dir::LocalTypeId,
    ) -> dir::LocalTypeId {
        if self.is_float_like(left) || self.is_float_like(right) {
            return self.primitive_type_id(dir::PrimitiveType::Float(dir::FloatType::Float64));
        }

        left
    }

    /// Return whether one type is string-like.
    fn is_string_like(&self, type_id: dir::LocalTypeId) -> bool {
        matches!(
            self.get_type(type_id),
            dir::Type::Primitive(dir::PrimitiveType::String)
                | dir::Type::Literal(dir::ScalarLiteral::String(_))
        )
    }

    /// Return whether one type is numeric-like.
    fn is_numeric_like(&self, type_id: dir::LocalTypeId) -> bool {
        matches!(
            self.get_type(type_id),
            dir::Type::Primitive(dir::PrimitiveType::Integer(_))
                | dir::Type::Primitive(dir::PrimitiveType::Float(_))
                | dir::Type::Literal(dir::ScalarLiteral::Integer(_))
                | dir::Type::Literal(dir::ScalarLiteral::Float(_))
        )
    }

    /// Return whether one type is float-like.
    fn is_float_like(&self, type_id: dir::LocalTypeId) -> bool {
        matches!(
            self.get_type(type_id),
            dir::Type::Primitive(dir::PrimitiveType::Float(_))
                | dir::Type::Literal(dir::ScalarLiteral::Float(_))
        )
    }
}

/// Build one builtin call resolution.
fn builtin_resolution(
    operator: dir::BinaryOperator,
    parameters: [dir::LocalTypeId; 2],
    return_type: dir::LocalTypeId,
) -> dir::CallResolution {
    dir::CallResolution::new(
        dir::CallTarget::Builtin(dir::BuiltinCall::BinaryOperator { operator }),
        parameters.to_vec(),
        Some(return_type),
    )
}
