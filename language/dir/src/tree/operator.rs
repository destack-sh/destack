use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::{LanguageItem, TokenType};

/// One operator precedence level.
///
/// Ordered from weakest to strongest:
/// ```
/// = += -= *= /= %= **= <<= >>= >>>= &= ^= |= &&= ||= ??= // assignment
/// condition ? then : else          // conditional
/// extends implements               // type relation
/// ??                               // nullish coalescing
/// ||                               // logical or
/// &&                               // logical and
/// |                                // bitwise or
/// ^                                // bitwise xor
/// &                                // bitwise and
/// == != === !==                    // equality
/// .. ..=                           // range
/// < > <= >= in instanceof          // comparison
/// << >>                            // shift
/// + -                              // addition
/// * / %                            // multiplication
/// **                               // exponentiation
/// !x -x ~x *x &x ..x ++x --x       // prefix
/// x() x[] x{} x? x! x++ x--        // postfix
/// ```
#[derive(
    Debug,
    Copy,
    Clone,
    Default,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
    Serialize,
    Deserialize,
    Reflect,
)]
#[repr(u8)]
pub enum OperatorPrecedence {
    /// No enclosing operation.
    #[default]
    Lowest,
    /// Assignment-related binary operators.
    /// `= += -= *= /= %= **= <<= >>= >>>= &= ^= |= &&= ||= ??=`
    Assignment,
    /// Conditional expressions.
    /// `condition ? then : else`
    Conditional,
    /// Type relation operators.
    /// `extends implements`
    TypeRelation,
    /// Nullish-coalescing binary operator.
    /// `??`
    NullishCoalescing,
    /// Logical-or binary operator.
    /// `||`
    LogicalOr,
    /// Logical-and binary operator.
    /// `&&`
    LogicalAnd,
    /// Bitwise-or binary operator.
    /// `|`
    BitwiseOr,
    /// Bitwise-xor binary operator.
    /// `^`
    BitwiseXor,
    /// Bitwise-and binary operator.
    /// `&`
    BitwiseAnd,
    /// Equality-related binary operators.
    /// `== != === !==`
    Equality,
    /// Range expressions.
    /// `.. ..=`
    Range,
    /// Comparison-related binary operators.
    /// `< > <= >= in instanceof`
    Comparison,
    /// Shift-related binary operators.
    /// `<< >>`
    Shift,
    /// Addition-related binary operators.
    /// `+ -`
    Addition,
    /// Multiplication-related binary operators.
    /// `* / %`
    Multiplication,
    /// Exponentiation-related binary operators.
    /// `**`
    Exponentiation,
    /// Unary prefix operators.
    /// `!x -x ~x &x *x ..x ++x --x`
    Prefix,
    /// Unary postfix operators.
    /// `x() x[] x{} x? x! x++ x--`
    Postfix,
    /// Primary expressions.
    Primary,
}

impl OperatorPrecedence {
    /// Return whether operators at this precedence group from right to left.
    #[inline]
    pub const fn is_right_associative(self) -> bool {
        matches!(
            self,
            OperatorPrecedence::Assignment
                | OperatorPrecedence::Conditional
                | OperatorPrecedence::Exponentiation
        )
    }
}

/// The end-bound form of one range.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum RangeEnd {
    /// `..`, excluding the end when present.
    Open,
    /// `..=`, including the end.
    Inclusive,
}

/// One unary operator.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum UnaryOperator {
    /// `++`
    PostIncrement,
    /// `--`
    PostDecrement,
    /// `++`
    PreIncrement,
    /// `--`
    PreDecrement,
    /// `!`
    Not,
    /// `+`
    Plus,
    /// `-`
    Negate,
    /// `~`
    ElementwiseNot,
    /// `*`
    Dereference,
    /// `...`
    Spread,
}

impl UnaryOperator {
    /// Return the single language protocol selected by this operator.
    pub fn single_protocol(self) -> Option<LanguageItem> {
        match self {
            Self::Negate => Some(LanguageItem::Negate),
            Self::Plus => Some(LanguageItem::Plus),
            Self::ElementwiseNot => Some(LanguageItem::Not),
            Self::Dereference => Some(LanguageItem::Dereference),
            _ => None,
        }
    }

    /// Return the source text for this unary operator.
    #[inline]
    pub fn text(self) -> &'static str {
        match self {
            UnaryOperator::PostIncrement => "++",
            UnaryOperator::PostDecrement => "--",
            UnaryOperator::PreIncrement => "++",
            UnaryOperator::PreDecrement => "--",
            UnaryOperator::Not => "!",
            UnaryOperator::Plus => "+",
            UnaryOperator::Negate => "-",
            UnaryOperator::ElementwiseNot => "~",
            UnaryOperator::Dereference => "*",
            UnaryOperator::Spread => "...",
        }
    }

    /// Get the precedence of the unary operator.
    #[inline]
    pub fn precedence(self) -> OperatorPrecedence {
        if self.is_prefix() {
            OperatorPrecedence::Prefix
        } else {
            OperatorPrecedence::Postfix
        }
    }

    /// Whether the unary operator is a prefix operator.
    #[inline]
    pub fn is_prefix(&self) -> bool {
        match self {
            UnaryOperator::PreIncrement
            | UnaryOperator::PreDecrement
            | UnaryOperator::Not
            | UnaryOperator::Plus
            | UnaryOperator::Negate
            | UnaryOperator::ElementwiseNot
            | UnaryOperator::Dereference
            | UnaryOperator::Spread => true,
            UnaryOperator::PostIncrement | UnaryOperator::PostDecrement => false,
        }
    }

    /// Whether the unary operator is a postfix operator.
    #[inline]
    pub fn is_postfix(&self) -> bool {
        !self.is_prefix()
    }

    /// Convert a prefix token to a UnaryOperator (if a direct mapping exists).
    #[inline]
    pub fn from_prefix_token(token_type: TokenType) -> Option<UnaryOperator> {
        match token_type {
            TokenType::Increment => Some(UnaryOperator::PreIncrement),
            TokenType::Decrement => Some(UnaryOperator::PreDecrement),
            TokenType::Not => Some(UnaryOperator::Not),
            TokenType::Add => Some(UnaryOperator::Plus),
            TokenType::Subtract => Some(UnaryOperator::Negate),
            TokenType::Multiply => Some(UnaryOperator::Dereference),
            TokenType::ElementwiseNot => Some(UnaryOperator::ElementwiseNot),
            TokenType::Spread => Some(UnaryOperator::Spread),
            _ => None,
        }
    }

    /// Convert a postfix token to a UnaryOperator (if a direct mapping exists).
    #[inline]
    pub fn from_postfix_token(token_type: TokenType) -> Option<UnaryOperator> {
        match token_type {
            TokenType::Increment => Some(UnaryOperator::PostIncrement),
            TokenType::Decrement => Some(UnaryOperator::PostDecrement),
            _ => None,
        }
    }
}

/// One infix binary operator.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum BinaryOperator {
    // exponentiation
    /// `**`
    Exponent,
    // multiplication
    /// `*`
    Multiply,
    /// `/`
    Divide,
    /// `%`
    Remainder,

    // addition
    /// `+`
    Add,
    /// `-`
    Subtract,

    // shift
    /// `<<`
    ShiftLeft,
    /// `>>`
    ShiftRight,
    /// `>>>`
    UnsignedShiftRight,

    // bitwise
    /// `&`
    ElementwiseAnd,
    /// `^`
    ElementwiseXor,
    /// `|`
    ElementwiseOr,

    // comparison
    /// `==`
    Equal,
    /// `!=`
    NotEqual,
    /// `===`
    EqualStrict,
    /// `!==`
    NotEqualStrict,
    /// `<`
    LessThan,
    /// `<=`
    LessThanOrEqual,
    /// `>`
    GreaterThan,
    /// `>=`
    GreaterThanOrEqual,

    // boolean
    /// `&&`
    And,
    /// `||`
    Or,
    /// `??`
    Coalesce,

    // comparison
    /// `in`
    In,
}

impl BinaryOperator {
    /// Return the single language protocol selected by this operator.
    pub fn single_protocol(self) -> Option<LanguageItem> {
        match self {
            Self::Exponent => Some(LanguageItem::Power),
            Self::Multiply => Some(LanguageItem::Multiply),
            Self::Divide => Some(LanguageItem::Divide),
            Self::Remainder => Some(LanguageItem::Remainder),
            Self::Add => Some(LanguageItem::Add),
            Self::Subtract => Some(LanguageItem::Subtract),
            Self::ShiftLeft => Some(LanguageItem::ShiftLeft),
            Self::ShiftRight => Some(LanguageItem::ShiftRight),
            Self::UnsignedShiftRight => Some(LanguageItem::ShiftRightUnsigned),
            Self::ElementwiseAnd => Some(LanguageItem::And),
            Self::ElementwiseXor => Some(LanguageItem::Xor),
            Self::ElementwiseOr => Some(LanguageItem::Or),
            Self::Equal | Self::NotEqual => Some(LanguageItem::PartialEqual),
            Self::EqualStrict | Self::NotEqualStrict => Some(LanguageItem::StrictEqual),
            _ => None,
        }
    }

    /// Return the source text for this binary operator.
    #[inline]
    pub fn text(self) -> &'static str {
        match self {
            BinaryOperator::Exponent => "**",
            BinaryOperator::Multiply => "*",
            BinaryOperator::Divide => "/",
            BinaryOperator::Remainder => "%",
            BinaryOperator::Add => "+",
            BinaryOperator::Subtract => "-",
            BinaryOperator::ShiftLeft => "<<",
            BinaryOperator::ShiftRight => ">>",
            BinaryOperator::UnsignedShiftRight => ">>>",
            BinaryOperator::ElementwiseAnd => "&",
            BinaryOperator::ElementwiseXor => "^",
            BinaryOperator::ElementwiseOr => "|",
            BinaryOperator::Equal => "==",
            BinaryOperator::NotEqual => "!=",
            BinaryOperator::EqualStrict => "===",
            BinaryOperator::NotEqualStrict => "!==",
            BinaryOperator::LessThan => "<",
            BinaryOperator::LessThanOrEqual => "<=",
            BinaryOperator::GreaterThan => ">",
            BinaryOperator::GreaterThanOrEqual => ">=",
            BinaryOperator::And => "&&",
            BinaryOperator::Or => "||",
            BinaryOperator::Coalesce => "??",
            BinaryOperator::In => "in",
        }
    }

    /// Return the precedence of this operator.
    #[inline]
    pub const fn precedence(self) -> OperatorPrecedence {
        match self {
            // exponentiation
            BinaryOperator::Exponent => OperatorPrecedence::Exponentiation,

            // multiplication
            BinaryOperator::Multiply => OperatorPrecedence::Multiplication,
            BinaryOperator::Divide => OperatorPrecedence::Multiplication,
            BinaryOperator::Remainder => OperatorPrecedence::Multiplication,

            // addition
            BinaryOperator::Add => OperatorPrecedence::Addition,
            BinaryOperator::Subtract => OperatorPrecedence::Addition,

            // shift
            BinaryOperator::ShiftLeft => OperatorPrecedence::Shift,
            BinaryOperator::ShiftRight => OperatorPrecedence::Shift,
            BinaryOperator::UnsignedShiftRight => OperatorPrecedence::Shift,

            // bitwise
            BinaryOperator::ElementwiseAnd => OperatorPrecedence::BitwiseAnd,
            BinaryOperator::ElementwiseXor => OperatorPrecedence::BitwiseXor,
            BinaryOperator::ElementwiseOr => OperatorPrecedence::BitwiseOr,

            // equality
            BinaryOperator::Equal => OperatorPrecedence::Equality,
            BinaryOperator::NotEqual => OperatorPrecedence::Equality,
            BinaryOperator::EqualStrict => OperatorPrecedence::Equality,
            BinaryOperator::NotEqualStrict => OperatorPrecedence::Equality,

            // comparison
            BinaryOperator::LessThan => OperatorPrecedence::Comparison,
            BinaryOperator::LessThanOrEqual => OperatorPrecedence::Comparison,
            BinaryOperator::GreaterThan => OperatorPrecedence::Comparison,
            BinaryOperator::GreaterThanOrEqual => OperatorPrecedence::Comparison,
            BinaryOperator::In => OperatorPrecedence::Comparison,

            // logical
            BinaryOperator::And => OperatorPrecedence::LogicalAnd,
            BinaryOperator::Or => OperatorPrecedence::LogicalOr,
            BinaryOperator::Coalesce => OperatorPrecedence::NullishCoalescing,
        }
    }

    /// Return whether this operator tests equality.
    #[inline]
    pub fn is_equality(self) -> bool {
        matches!(
            self,
            BinaryOperator::Equal
                | BinaryOperator::NotEqual
                | BinaryOperator::EqualStrict
                | BinaryOperator::NotEqualStrict
        )
    }

    /// Return whether this operator compares two values.
    #[inline]
    pub fn is_comparison(self) -> bool {
        self.is_equality()
            || matches!(
                self,
                BinaryOperator::LessThan
                    | BinaryOperator::LessThanOrEqual
                    | BinaryOperator::GreaterThan
                    | BinaryOperator::GreaterThanOrEqual
            )
    }

    /// Return the operator that preserves this comparison after swapping its operands.
    pub fn swapped(self) -> Option<Self> {
        let operator = match self {
            Self::Equal | Self::NotEqual | Self::EqualStrict | Self::NotEqualStrict => self,
            Self::LessThan => Self::GreaterThan,
            Self::LessThanOrEqual => Self::GreaterThanOrEqual,
            Self::GreaterThan => Self::LessThan,
            Self::GreaterThanOrEqual => Self::LessThanOrEqual,
            _ => return None,
        };

        Some(operator)
    }

    /// Return whether this operator tests overloadable value equality.
    #[inline]
    pub fn is_value_equality(self) -> bool {
        matches!(self, BinaryOperator::Equal | BinaryOperator::NotEqual)
    }

    /// Return whether this equality operator negates the relation.
    #[inline]
    pub fn is_negative_equality(self) -> bool {
        matches!(
            self,
            BinaryOperator::NotEqual | BinaryOperator::NotEqualStrict
        )
    }

    /// Return whether this operator tests strict identity equality.
    #[inline]
    pub fn is_strict_equality(self) -> bool {
        matches!(
            self,
            BinaryOperator::EqualStrict | BinaryOperator::NotEqualStrict
        )
    }

    /// Return whether this operator has builtin primitive numeric behavior.
    #[inline]
    pub fn has_numeric_builtin(self) -> bool {
        matches!(
            self,
            BinaryOperator::Exponent
                | BinaryOperator::Multiply
                | BinaryOperator::Divide
                | BinaryOperator::Remainder
                | BinaryOperator::Add
                | BinaryOperator::Subtract
                | BinaryOperator::ShiftLeft
                | BinaryOperator::ShiftRight
                | BinaryOperator::UnsignedShiftRight
                | BinaryOperator::ElementwiseAnd
                | BinaryOperator::ElementwiseXor
                | BinaryOperator::ElementwiseOr
                | BinaryOperator::Equal
                | BinaryOperator::NotEqual
                | BinaryOperator::LessThan
                | BinaryOperator::LessThanOrEqual
                | BinaryOperator::GreaterThan
                | BinaryOperator::GreaterThanOrEqual
        )
    }

    /// Return whether this numeric operator returns boolean.
    #[inline]
    pub fn returns_boolean_for_numeric_operands(self) -> bool {
        matches!(
            self,
            BinaryOperator::Equal
                | BinaryOperator::NotEqual
                | BinaryOperator::LessThan
                | BinaryOperator::LessThanOrEqual
                | BinaryOperator::GreaterThan
                | BinaryOperator::GreaterThanOrEqual
        )
    }

    /// Return whether this numeric operator only accepts integer operands.
    #[inline]
    pub fn requires_integer_numeric_operands(self) -> bool {
        matches!(
            self,
            BinaryOperator::ShiftLeft
                | BinaryOperator::ShiftRight
                | BinaryOperator::UnsignedShiftRight
                | BinaryOperator::ElementwiseAnd
                | BinaryOperator::ElementwiseXor
                | BinaryOperator::ElementwiseOr
        )
    }

    /// Return whether this operator is boolean short-circuit logic.
    #[inline]
    pub fn is_logical_boolean(self) -> bool {
        matches!(self, BinaryOperator::And | BinaryOperator::Or)
    }

    /// Convert a TokenType to a BinaryOperator (if a direct mapping exists).
    #[inline]
    pub fn from_token(token_str: &str, token_type: TokenType) -> Option<BinaryOperator> {
        match token_type {
            // multiplication
            TokenType::Multiply => Some(BinaryOperator::Multiply),
            TokenType::Exponent => Some(BinaryOperator::Exponent),
            TokenType::Divide => Some(BinaryOperator::Divide),
            TokenType::Remainder => Some(BinaryOperator::Remainder),

            // addition
            TokenType::Add => Some(BinaryOperator::Add),
            TokenType::Subtract => Some(BinaryOperator::Subtract),

            // shift
            TokenType::ShiftLeft => Some(BinaryOperator::ShiftLeft),
            TokenType::ShiftRight => Some(BinaryOperator::ShiftRight),
            TokenType::UnsignedShiftRight => Some(BinaryOperator::UnsignedShiftRight),

            // elementwise
            TokenType::ElementwiseAnd => Some(BinaryOperator::ElementwiseAnd),
            TokenType::ElementwiseXor => Some(BinaryOperator::ElementwiseXor),
            TokenType::ElementwiseOr => Some(BinaryOperator::ElementwiseOr),

            // comparison
            TokenType::Equal => Some(BinaryOperator::Equal),
            TokenType::EqualWide => Some(BinaryOperator::EqualStrict),
            TokenType::NotEqual => Some(BinaryOperator::NotEqual),
            TokenType::NotEqualWide => Some(BinaryOperator::NotEqualStrict),
            TokenType::LessThan => Some(BinaryOperator::LessThan),
            TokenType::LessThanOrEqual => Some(BinaryOperator::LessThanOrEqual),
            TokenType::GreaterThan => Some(BinaryOperator::GreaterThan),
            TokenType::GreaterThanOrEqual => Some(BinaryOperator::GreaterThanOrEqual),

            // logical
            TokenType::LogicalAnd => Some(BinaryOperator::And),
            TokenType::LogicalOr => Some(BinaryOperator::Or),
            TokenType::Coalesce => Some(BinaryOperator::Coalesce),

            // container
            TokenType::Identifier if token_str == "in" => Some(BinaryOperator::In),

            _ => None,
        }
    }
}

/// An AssignOperator is an assignment type.
/// All assignment operators share one right-associative precedence.
///
/// Examples:
/// ```
/// x = 1
/// x += 1
/// x >>= 1
/// x &= 1
/// x |= 1
/// ```
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum AssignOperator {
    /// `=`
    Assign,

    // multiplication assignment
    /// `*=`
    MultiplyAssign,
    /// `**=`
    ExponentAssign,
    /// `/=`
    DivideAssign,
    /// `%=`
    RemainderAssign,

    // addition assignment
    /// `+=`
    AddAssign,
    /// `-=`
    SubtractAssign,

    // shift assignment
    /// `<<=`
    ShiftLeftAssign,
    /// `>>=`
    ShiftRightAssign,
    /// `>>>=`
    UnsignedShiftRightAssign,

    // elementwise assignment
    /// `&=`
    ElementwiseAndAssign,
    /// `^=`
    ElementwiseXorAssign,
    /// `|=`
    ElementwiseOrAssign,

    // logical assignment
    /// `&&=`
    AndAssign,
    /// `||=`
    OrAssign,
    /// `??=`
    CoalesceAssign,
}

impl AssignOperator {
    /// Return the single language protocol selected by this assignment operator.
    pub fn single_protocol(self) -> Option<LanguageItem> {
        self.try_into()
            .ok()
            .and_then(BinaryOperator::single_protocol)
    }

    /// Return the source text for this assignment operator.
    #[inline]
    pub fn text(self) -> &'static str {
        match self {
            AssignOperator::Assign => "=",
            AssignOperator::MultiplyAssign => "*=",
            AssignOperator::ExponentAssign => "**=",
            AssignOperator::DivideAssign => "/=",
            AssignOperator::RemainderAssign => "%=",
            AssignOperator::AddAssign => "+=",
            AssignOperator::SubtractAssign => "-=",
            AssignOperator::ShiftLeftAssign => "<<=",
            AssignOperator::ShiftRightAssign => ">>=",
            AssignOperator::UnsignedShiftRightAssign => ">>>=",
            AssignOperator::ElementwiseAndAssign => "&=",
            AssignOperator::ElementwiseXorAssign => "^=",
            AssignOperator::ElementwiseOrAssign => "|=",
            AssignOperator::AndAssign => "&&=",
            AssignOperator::OrAssign => "||=",
            AssignOperator::CoalesceAssign => "??=",
        }
    }

    /// Return the precedence shared by assignment operators.
    #[inline]
    pub const fn precedence(self) -> OperatorPrecedence {
        OperatorPrecedence::Assignment
    }

    /// Convert a TokenType to an AssignOperator (if a direct mapping exists).
    #[inline]
    pub fn from_token(token_type: TokenType) -> Option<AssignOperator> {
        match token_type {
            TokenType::Assign => Some(AssignOperator::Assign),

            // addition
            TokenType::AddAssign => Some(AssignOperator::AddAssign),
            TokenType::SubtractAssign => Some(AssignOperator::SubtractAssign),

            // multiplication
            TokenType::MultiplyAssign => Some(AssignOperator::MultiplyAssign),
            TokenType::ExponentAssign => Some(AssignOperator::ExponentAssign),
            TokenType::DivideAssign => Some(AssignOperator::DivideAssign),
            TokenType::RemainderAssign => Some(AssignOperator::RemainderAssign),

            // shift
            TokenType::ShiftLeftAssign => Some(AssignOperator::ShiftLeftAssign),
            TokenType::ShiftRightAssign => Some(AssignOperator::ShiftRightAssign),
            TokenType::UnsignedShiftRightAssign => Some(AssignOperator::UnsignedShiftRightAssign),

            // elementwise
            TokenType::ElementwiseAndAssign => Some(AssignOperator::ElementwiseAndAssign),
            TokenType::ElementwiseOrAssign => Some(AssignOperator::ElementwiseOrAssign),
            TokenType::ElementwiseXorAssign => Some(AssignOperator::ElementwiseXorAssign),

            // logical
            TokenType::LogicalAndAssign => Some(AssignOperator::AndAssign),
            TokenType::LogicalOrAssign => Some(AssignOperator::OrAssign),
            TokenType::CoalesceAssign => Some(AssignOperator::CoalesceAssign),

            _ => None,
        }
    }
}

impl TryFrom<BinaryOperator> for AssignOperator {
    type Error = ();

    /// Convert a binary operator to its compound assignment operator.
    fn try_from(operator: BinaryOperator) -> Result<Self, Self::Error> {
        match operator {
            BinaryOperator::Exponent => Ok(AssignOperator::ExponentAssign),
            BinaryOperator::Multiply => Ok(AssignOperator::MultiplyAssign),
            BinaryOperator::Divide => Ok(AssignOperator::DivideAssign),
            BinaryOperator::Remainder => Ok(AssignOperator::RemainderAssign),
            BinaryOperator::Add => Ok(AssignOperator::AddAssign),
            BinaryOperator::Subtract => Ok(AssignOperator::SubtractAssign),
            BinaryOperator::ShiftLeft => Ok(AssignOperator::ShiftLeftAssign),
            BinaryOperator::ShiftRight => Ok(AssignOperator::ShiftRightAssign),
            BinaryOperator::UnsignedShiftRight => Ok(AssignOperator::UnsignedShiftRightAssign),
            BinaryOperator::ElementwiseAnd => Ok(AssignOperator::ElementwiseAndAssign),
            BinaryOperator::ElementwiseXor => Ok(AssignOperator::ElementwiseXorAssign),
            BinaryOperator::ElementwiseOr => Ok(AssignOperator::ElementwiseOrAssign),
            BinaryOperator::Equal
            | BinaryOperator::NotEqual
            | BinaryOperator::EqualStrict
            | BinaryOperator::NotEqualStrict
            | BinaryOperator::LessThan
            | BinaryOperator::LessThanOrEqual
            | BinaryOperator::GreaterThan
            | BinaryOperator::GreaterThanOrEqual
            | BinaryOperator::And
            | BinaryOperator::Or
            | BinaryOperator::Coalesce
            | BinaryOperator::In => Err(()),
        }
    }
}

impl TryFrom<AssignOperator> for BinaryOperator {
    type Error = ();

    /// Convert a compound assignment operator to its binary operator.
    fn try_from(operator: AssignOperator) -> Result<Self, Self::Error> {
        match operator {
            AssignOperator::AddAssign => Ok(BinaryOperator::Add),
            AssignOperator::SubtractAssign => Ok(BinaryOperator::Subtract),
            AssignOperator::MultiplyAssign => Ok(BinaryOperator::Multiply),
            AssignOperator::DivideAssign => Ok(BinaryOperator::Divide),
            AssignOperator::RemainderAssign => Ok(BinaryOperator::Remainder),
            AssignOperator::ExponentAssign => Ok(BinaryOperator::Exponent),
            AssignOperator::ShiftLeftAssign => Ok(BinaryOperator::ShiftLeft),
            AssignOperator::ShiftRightAssign => Ok(BinaryOperator::ShiftRight),
            AssignOperator::UnsignedShiftRightAssign => Ok(BinaryOperator::UnsignedShiftRight),
            AssignOperator::ElementwiseAndAssign => Ok(BinaryOperator::ElementwiseAnd),
            AssignOperator::ElementwiseXorAssign => Ok(BinaryOperator::ElementwiseXor),
            AssignOperator::ElementwiseOrAssign => Ok(BinaryOperator::ElementwiseOr),
            AssignOperator::Assign
            | AssignOperator::AndAssign
            | AssignOperator::OrAssign
            | AssignOperator::CoalesceAssign => Err(()),
        }
    }
}
