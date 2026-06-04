use destack_dir as dir;

use destack_core::StringPool;
use smallvec::{SmallVec, smallvec};

/// Type selected by one operator protocol step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum OperatorType {
    /// Use the selected method return type.
    MethodReturn,
    /// Use the builtin boolean type.
    Boolean,
}

/// Static generic argument required by one operator protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum OperatorProtocolArgument {
    /// Static access argument.
    Access(dir::Access),
}

/// Method required by one operator protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum OperatorMethod {
    /// Addition method.
    Add,
    /// Subtraction method.
    Subtract,
    /// Multiplication method.
    Multiply,
    /// Division method.
    Divide,
    /// Remainder method.
    Remainder,
    /// Exponentiation method.
    Power,
    /// Bitwise and method.
    And,
    /// Bitwise or method.
    Or,
    /// Bitwise xor method.
    Xor,
    /// Bitwise not method.
    Not,
    /// Unary plus method.
    Plus,
    /// Unary negation method.
    Negate,
    /// Left shift method.
    ShiftLeft,
    /// Right shift method.
    ShiftRight,
    /// Unsigned right shift method.
    ShiftRightUnsigned,
    /// Dereference method.
    Dereference,
    /// Equality method.
    Equal,
    /// Comparison method.
    Compare,
    /// Partial comparison method.
    PartialCompare,
}

impl OperatorMethod {
    /// Return the source member key for this protocol method.
    pub(in crate::check) fn key(self, strings: &StringPool) -> dir::StaticKey {
        dir::StaticKey::Name(strings.intern(self.name()))
    }

    /// Return the source member name for this protocol method.
    fn name(self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Subtract => "subtract",
            Self::Multiply => "multiply",
            Self::Divide => "divide",
            Self::Remainder => "remainder",
            Self::Power => "power",
            Self::And => "and",
            Self::Or => "or",
            Self::Xor => "xor",
            Self::Not => "not",
            Self::Plus => "plus",
            Self::Negate => "negate",
            Self::ShiftLeft => "shiftLeft",
            Self::ShiftRight => "shiftRight",
            Self::ShiftRightUnsigned => "shiftRightUnsigned",
            Self::Dereference => "dereference",
            Self::Equal => "equal",
            Self::Compare => "compare",
            Self::PartialCompare => "partialCompare",
        }
    }
}

/// Language item protocol used by one operator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) struct OperatorProtocol {
    /// The operator interface language item.
    pub(in crate::check) item: dir::LanguageItem,
    /// The required protocol arguments.
    pub(in crate::check) arguments: SmallVec<[OperatorProtocolArgument; 2]>,
    /// The required operator method.
    pub(in crate::check) method: OperatorMethod,
    /// The type required from the selected method.
    pub(in crate::check) method_return: OperatorType,
    /// The type produced by the operator expression.
    pub(in crate::check) expression_type: OperatorType,
}

impl OperatorProtocol {
    /// Return one protocol whose expression type is the selected method return.
    fn returning_method(item: dir::LanguageItem, method: OperatorMethod) -> Self {
        Self {
            item,
            arguments: SmallVec::new(),
            method,
            method_return: OperatorType::MethodReturn,
            expression_type: OperatorType::MethodReturn,
        }
    }
}

/// Return the ordered protocol candidates for one unary operator.
pub(in crate::check) fn unary_operator_protocols(
    operator: dir::UnaryOperator,
) -> SmallVec<[OperatorProtocol; 2]> {
    let protocol = match operator {
        dir::UnaryOperator::Negate => {
            OperatorProtocol::returning_method(dir::LanguageItem::Negate, OperatorMethod::Negate)
        }
        dir::UnaryOperator::Plus => {
            OperatorProtocol::returning_method(dir::LanguageItem::Plus, OperatorMethod::Plus)
        }
        dir::UnaryOperator::ElementwiseNot => {
            OperatorProtocol::returning_method(dir::LanguageItem::Not, OperatorMethod::Not)
        }
        dir::UnaryOperator::Dereference => OperatorProtocol {
            item: dir::LanguageItem::Dereference,
            arguments: smallvec![OperatorProtocolArgument::Access(dir::Access::Readonly)],
            method: OperatorMethod::Dereference,
            method_return: OperatorType::MethodReturn,
            expression_type: OperatorType::MethodReturn,
        },
        dir::UnaryOperator::PostIncrement
        | dir::UnaryOperator::PostDecrement
        | dir::UnaryOperator::PreIncrement
        | dir::UnaryOperator::PreDecrement
        | dir::UnaryOperator::Not
        | dir::UnaryOperator::Typeof
        | dir::UnaryOperator::Void
        | dir::UnaryOperator::Spread => return SmallVec::new(),
    };

    smallvec![protocol]
}

/// Return the ordered protocol candidates for one binary operator.
pub(in crate::check) fn binary_operator_protocols(
    operator: dir::BinaryOperator,
) -> SmallVec<[OperatorProtocol; 2]> {
    match operator {
        dir::BinaryOperator::Add => {
            smallvec![OperatorProtocol::returning_method(
                dir::LanguageItem::Add,
                OperatorMethod::Add,
            )]
        }
        dir::BinaryOperator::Subtract => smallvec![OperatorProtocol::returning_method(
            dir::LanguageItem::Subtract,
            OperatorMethod::Subtract,
        )],
        dir::BinaryOperator::Multiply => smallvec![OperatorProtocol::returning_method(
            dir::LanguageItem::Multiply,
            OperatorMethod::Multiply,
        )],
        dir::BinaryOperator::Divide => {
            smallvec![OperatorProtocol::returning_method(
                dir::LanguageItem::Divide,
                OperatorMethod::Divide,
            )]
        }
        dir::BinaryOperator::Remainder => smallvec![OperatorProtocol::returning_method(
            dir::LanguageItem::Remainder,
            OperatorMethod::Remainder,
        )],
        dir::BinaryOperator::Exponent => {
            smallvec![OperatorProtocol::returning_method(
                dir::LanguageItem::Power,
                OperatorMethod::Power,
            )]
        }
        dir::BinaryOperator::ShiftLeft => smallvec![OperatorProtocol::returning_method(
            dir::LanguageItem::ShiftLeft,
            OperatorMethod::ShiftLeft,
        )],
        dir::BinaryOperator::ShiftRight => smallvec![OperatorProtocol::returning_method(
            dir::LanguageItem::ShiftRight,
            OperatorMethod::ShiftRight,
        )],
        dir::BinaryOperator::UnsignedShiftRight => smallvec![OperatorProtocol::returning_method(
            dir::LanguageItem::ShiftRightUnsigned,
            OperatorMethod::ShiftRightUnsigned,
        )],
        dir::BinaryOperator::ElementwiseAnd => {
            smallvec![OperatorProtocol::returning_method(
                dir::LanguageItem::And,
                OperatorMethod::And,
            )]
        }
        dir::BinaryOperator::ElementwiseXor => {
            smallvec![OperatorProtocol::returning_method(
                dir::LanguageItem::Xor,
                OperatorMethod::Xor,
            )]
        }
        dir::BinaryOperator::ElementwiseOr => {
            smallvec![OperatorProtocol::returning_method(
                dir::LanguageItem::Or,
                OperatorMethod::Or,
            )]
        }
        dir::BinaryOperator::Equal | dir::BinaryOperator::NotEqual => smallvec![OperatorProtocol {
            item: dir::LanguageItem::PartialEqual,
            arguments: SmallVec::new(),
            method: OperatorMethod::Equal,
            method_return: OperatorType::Boolean,
            expression_type: OperatorType::Boolean,
        }],
        dir::BinaryOperator::LessThan
        | dir::BinaryOperator::LessThanOrEqual
        | dir::BinaryOperator::GreaterThan
        | dir::BinaryOperator::GreaterThanOrEqual => smallvec![
            OperatorProtocol {
                item: dir::LanguageItem::Compare,
                arguments: SmallVec::new(),
                method: OperatorMethod::Compare,
                method_return: OperatorType::MethodReturn,
                expression_type: OperatorType::Boolean,
            },
            OperatorProtocol {
                item: dir::LanguageItem::PartialCompare,
                arguments: SmallVec::new(),
                method: OperatorMethod::PartialCompare,
                method_return: OperatorType::MethodReturn,
                expression_type: OperatorType::Boolean,
            },
        ],
        dir::BinaryOperator::EqualStrict
        | dir::BinaryOperator::NotEqualStrict
        | dir::BinaryOperator::And
        | dir::BinaryOperator::Or
        | dir::BinaryOperator::Coalesce
        | dir::BinaryOperator::In => SmallVec::new(),
    }
}
