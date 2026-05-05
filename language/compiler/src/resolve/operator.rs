use destack_dir::{AssignOperator, BinaryOperator, LanguageSymbol, UnaryOperator};

/// Extension trait to get the LanguageSymbol for an operator.
pub trait OperatorLanguageSymbolExt {
    /// Get the LanguageSymbol for this operator, if one exists.
    fn language_symbol(&self) -> Option<LanguageSymbol>;
}

impl OperatorLanguageSymbolExt for BinaryOperator {
    fn language_symbol(&self) -> Option<LanguageSymbol> {
        match self {
            // arithmetic
            BinaryOperator::Add | BinaryOperator::WrappingAdd | BinaryOperator::SaturatingAdd => {
                Some(LanguageSymbol::Add)
            }
            BinaryOperator::Subtract
            | BinaryOperator::WrappingSubtract
            | BinaryOperator::SaturatingSubtract => Some(LanguageSymbol::Subtract),
            BinaryOperator::Multiply
            | BinaryOperator::WrappingMultiply
            | BinaryOperator::SaturatingMultiply => Some(LanguageSymbol::Multiply),
            BinaryOperator::Divide => Some(LanguageSymbol::Divide),
            BinaryOperator::Remainder => Some(LanguageSymbol::Remainder),
            BinaryOperator::Exponent
            | BinaryOperator::WrappingExponent
            | BinaryOperator::SaturatingExponent => Some(LanguageSymbol::Power),

            // shift
            BinaryOperator::ShiftLeft | BinaryOperator::SaturatingShiftLeft => {
                Some(LanguageSymbol::ShiftLeft)
            }
            BinaryOperator::ShiftRight => Some(LanguageSymbol::ShiftRight),
            BinaryOperator::UnsignedShiftRight => Some(LanguageSymbol::ShiftRightUnsigned),

            // elementwise/bitwise
            BinaryOperator::ElementwiseAnd => Some(LanguageSymbol::And),
            BinaryOperator::ElementwiseXor => Some(LanguageSymbol::Xor),
            BinaryOperator::ElementwiseOr => Some(LanguageSymbol::Or),

            // comparison
            BinaryOperator::Equal | BinaryOperator::NotEqual => Some(LanguageSymbol::Equal),
            BinaryOperator::EqualStrict | BinaryOperator::NotEqualStrict => None,
            BinaryOperator::LessThan
            | BinaryOperator::LessThanOrEqual
            | BinaryOperator::GreaterThan
            | BinaryOperator::GreaterThanOrEqual => Some(LanguageSymbol::Compare),

            // boolean
            BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Coalesce => None,

            // container
            BinaryOperator::In => None,
        }
    }
}

impl OperatorLanguageSymbolExt for UnaryOperator {
    fn language_symbol(&self) -> Option<LanguageSymbol> {
        match self {
            UnaryOperator::Negate | UnaryOperator::WrappingNegate => Some(LanguageSymbol::Negate),
            UnaryOperator::Plus => Some(LanguageSymbol::Plus),
            UnaryOperator::ElementwiseNot => Some(LanguageSymbol::Not),
            UnaryOperator::Dereference => Some(LanguageSymbol::ReadonlyDereference),

            // increment/decrement
            UnaryOperator::PostIncrement
            | UnaryOperator::PostDecrement
            | UnaryOperator::PreIncrement
            | UnaryOperator::PreDecrement => None,

            // logical not
            UnaryOperator::Not => None,

            // spread
            UnaryOperator::Spread => None,

            // keyword operators
            UnaryOperator::Typeof | UnaryOperator::Void => None,
        }
    }
}

impl OperatorLanguageSymbolExt for AssignOperator {
    fn language_symbol(&self) -> Option<LanguageSymbol> {
        match self {
            // arithmetic
            AssignOperator::AddAssign
            | AssignOperator::WrappingAddAssign
            | AssignOperator::SaturatingAddAssign => Some(LanguageSymbol::Add),
            AssignOperator::SubtractAssign
            | AssignOperator::WrappingSubtractAssign
            | AssignOperator::SaturatingSubtractAssign => Some(LanguageSymbol::Subtract),
            AssignOperator::MultiplyAssign
            | AssignOperator::WrappingMultiplyAssign
            | AssignOperator::SaturatingMultiplyAssign => Some(LanguageSymbol::Multiply),
            AssignOperator::DivideAssign => Some(LanguageSymbol::Divide),
            AssignOperator::RemainderAssign => Some(LanguageSymbol::Remainder),
            AssignOperator::ExponentAssign
            | AssignOperator::WrappingExponentAssign
            | AssignOperator::SaturatingExponentAssign => Some(LanguageSymbol::Power),

            // shift
            AssignOperator::ShiftLeftAssign | AssignOperator::SaturatingShiftLeftAssign => {
                Some(LanguageSymbol::ShiftLeft)
            }
            AssignOperator::ShiftRightAssign => Some(LanguageSymbol::ShiftRight),
            AssignOperator::UnsignedShiftRightAssign => Some(LanguageSymbol::ShiftRightUnsigned),

            // elementwise/bitwise
            AssignOperator::ElementwiseAndAssign => Some(LanguageSymbol::And),
            AssignOperator::ElementwiseXorAssign => Some(LanguageSymbol::Xor),
            AssignOperator::ElementwiseOrAssign => Some(LanguageSymbol::Or),

            // boolean
            AssignOperator::AndAssign
            | AssignOperator::OrAssign
            | AssignOperator::CoalesceAssign => None,
        }
    }
}
