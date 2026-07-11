use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{Keyword, TokenType};

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
    /// Atomic expressions.
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

/// The end-bound spelling of one range.
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
    /// `typeof`
    Typeof,
    /// `void`
    Void,
    /// `*`
    Dereference,
    /// `...`
    Spread,
}

impl UnaryOperator {
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
            UnaryOperator::Typeof => "typeof",
            UnaryOperator::Void => "void",
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
            | UnaryOperator::Typeof
            | UnaryOperator::Void
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

    /// Convert a prefix keyword to a UnaryOperator (if a direct mapping exists).
    #[inline]
    pub fn from_prefix_keyword(keyword: Keyword) -> Option<UnaryOperator> {
        match keyword {
            Keyword::Typeof => Some(UnaryOperator::Typeof),
            Keyword::Void => Some(UnaryOperator::Void),
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
    /// Return the binary operator used by this compound assignment operator.
    #[inline]
    pub fn binary_operator(self) -> Option<BinaryOperator> {
        match self {
            AssignOperator::AddAssign => Some(BinaryOperator::Add),
            AssignOperator::SubtractAssign => Some(BinaryOperator::Subtract),
            AssignOperator::MultiplyAssign => Some(BinaryOperator::Multiply),
            AssignOperator::DivideAssign => Some(BinaryOperator::Divide),
            AssignOperator::RemainderAssign => Some(BinaryOperator::Remainder),
            AssignOperator::ExponentAssign => Some(BinaryOperator::Exponent),
            AssignOperator::ShiftLeftAssign => Some(BinaryOperator::ShiftLeft),
            AssignOperator::ShiftRightAssign => Some(BinaryOperator::ShiftRight),
            AssignOperator::UnsignedShiftRightAssign => Some(BinaryOperator::UnsignedShiftRight),
            AssignOperator::ElementwiseAndAssign => Some(BinaryOperator::ElementwiseAnd),
            AssignOperator::ElementwiseXorAssign => Some(BinaryOperator::ElementwiseXor),
            AssignOperator::ElementwiseOrAssign => Some(BinaryOperator::ElementwiseOr),
            AssignOperator::Assign
            | AssignOperator::AndAssign
            | AssignOperator::OrAssign
            | AssignOperator::CoalesceAssign => None,
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
