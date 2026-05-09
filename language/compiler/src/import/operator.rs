use destack_dir::{AssignOperator, BinaryOperator, LanguageItem, UnaryOperator};

/// Resolve language items for operators.
pub trait OperatorLanguageItemExt {
    /// Return the language item for this operator, if one exists.
    fn language_item(&self) -> Option<LanguageItem>;
}

impl OperatorLanguageItemExt for BinaryOperator {
    fn language_item(&self) -> Option<LanguageItem> {
        match self {
            // arithmetic
            BinaryOperator::Add | BinaryOperator::WrappingAdd | BinaryOperator::SaturatingAdd => {
                Some(LanguageItem::Add)
            }
            BinaryOperator::Subtract
            | BinaryOperator::WrappingSubtract
            | BinaryOperator::SaturatingSubtract => Some(LanguageItem::Subtract),
            BinaryOperator::Multiply
            | BinaryOperator::WrappingMultiply
            | BinaryOperator::SaturatingMultiply => Some(LanguageItem::Multiply),
            BinaryOperator::Divide => Some(LanguageItem::Divide),
            BinaryOperator::Remainder => Some(LanguageItem::Remainder),
            BinaryOperator::Exponent
            | BinaryOperator::WrappingExponent
            | BinaryOperator::SaturatingExponent => Some(LanguageItem::Power),

            // shift
            BinaryOperator::ShiftLeft | BinaryOperator::SaturatingShiftLeft => {
                Some(LanguageItem::ShiftLeft)
            }
            BinaryOperator::ShiftRight => Some(LanguageItem::ShiftRight),
            BinaryOperator::UnsignedShiftRight => Some(LanguageItem::ShiftRightUnsigned),

            // elementwise/bitwise
            BinaryOperator::ElementwiseAnd => Some(LanguageItem::And),
            BinaryOperator::ElementwiseXor => Some(LanguageItem::Xor),
            BinaryOperator::ElementwiseOr => Some(LanguageItem::Or),

            // comparison
            BinaryOperator::Equal | BinaryOperator::NotEqual => Some(LanguageItem::Equal),
            BinaryOperator::EqualStrict | BinaryOperator::NotEqualStrict => None,
            BinaryOperator::LessThan
            | BinaryOperator::LessThanOrEqual
            | BinaryOperator::GreaterThan
            | BinaryOperator::GreaterThanOrEqual => Some(LanguageItem::Compare),

            // boolean
            BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Coalesce => None,

            // container
            BinaryOperator::In => None,
        }
    }
}

impl OperatorLanguageItemExt for UnaryOperator {
    fn language_item(&self) -> Option<LanguageItem> {
        match self {
            UnaryOperator::Negate | UnaryOperator::WrappingNegate => Some(LanguageItem::Negate),
            UnaryOperator::Plus => Some(LanguageItem::Plus),
            UnaryOperator::ElementwiseNot => Some(LanguageItem::Not),
            UnaryOperator::Dereference => Some(LanguageItem::ReadonlyDereference),

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

impl OperatorLanguageItemExt for AssignOperator {
    fn language_item(&self) -> Option<LanguageItem> {
        match self {
            // arithmetic
            AssignOperator::AddAssign
            | AssignOperator::WrappingAddAssign
            | AssignOperator::SaturatingAddAssign => Some(LanguageItem::Add),
            AssignOperator::SubtractAssign
            | AssignOperator::WrappingSubtractAssign
            | AssignOperator::SaturatingSubtractAssign => Some(LanguageItem::Subtract),
            AssignOperator::MultiplyAssign
            | AssignOperator::WrappingMultiplyAssign
            | AssignOperator::SaturatingMultiplyAssign => Some(LanguageItem::Multiply),
            AssignOperator::DivideAssign => Some(LanguageItem::Divide),
            AssignOperator::RemainderAssign => Some(LanguageItem::Remainder),
            AssignOperator::ExponentAssign
            | AssignOperator::WrappingExponentAssign
            | AssignOperator::SaturatingExponentAssign => Some(LanguageItem::Power),

            // shift
            AssignOperator::ShiftLeftAssign | AssignOperator::SaturatingShiftLeftAssign => {
                Some(LanguageItem::ShiftLeft)
            }
            AssignOperator::ShiftRightAssign => Some(LanguageItem::ShiftRight),
            AssignOperator::UnsignedShiftRightAssign => Some(LanguageItem::ShiftRightUnsigned),

            // elementwise/bitwise
            AssignOperator::ElementwiseAndAssign => Some(LanguageItem::And),
            AssignOperator::ElementwiseXorAssign => Some(LanguageItem::Xor),
            AssignOperator::ElementwiseOrAssign => Some(LanguageItem::Or),

            // boolean
            AssignOperator::AndAssign
            | AssignOperator::OrAssign
            | AssignOperator::CoalesceAssign => None,
        }
    }
}
