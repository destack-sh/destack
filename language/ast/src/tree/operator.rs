use serde::{Deserialize, Serialize};

use crate::{Keyword, TokenType};

/// The operator group (for precedence).
///
/// Precedence:
/// ```
/// x() x[] x{} x? x! x++ x--            // postfix
/// !x -x -%x ~x *x &x ..x ++x --x       // prefix
/// type readonly typeof keyof           // type unary operator
/// * / % *% *|                          // multiplication
/// + - +% -% +| -|                      // addition
/// << >> <<|                            // shift
/// & ^ |                                // elementwise
/// == != < > <= >=                      // comparison
/// && || ??                             // boolean
/// in of                                // container
/// in extends implements                // type binary operator
/// =                                    // assignment
/// *= /= %= **= *%= *|=                 // assignment multiplication
/// += -= +%= -%= +|= -|=                // assignment addition
/// <<= >>= <<|=                         // assignment shift
/// &= ^= |=                             // assignment elementwise
/// &&= ||=                              // assignment logical
/// ```
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum OperatorPrecedence {
    /// Unary postfix operators.
    /// `x() x[] x{} x? x! x++ x--`
    Postfix = 2000,
    /// Unary prefix operators.
    /// `!x -x -%x ~x &x *x ..x ++x --x`
    Prefix = 1900,
    /// Type unary operators.
    /// `type readonly typeof keyof as comptime`
    TypeUnary = 1800,
    /// Multiplication-related binary operators.
    /// `* / % ** *% *| **% **|`
    Multiplication = 1700,
    /// Addition-related binary operators.
    /// `+ - +% -% +| -|`
    Addition = 1600,
    /// Shift-related binary operators.
    /// `<< >> <<|`
    Shift = 1500,
    /// Elementwise-related binary operators.
    /// `& ^ |`
    Elementwise = 1400,
    /// Comparison-related binary operators.
    /// `== != < > <= >=`
    Comparison = 1300,
    /// Boolean-logical binary operators.
    /// `&& ||`
    Boolean = 1200,
    /// Container operators.
    /// `in` `of`
    Container = 1100,
    /// Type binary operators.
    /// `in extends implements`
    TypeBinary = 1000,
    /// Assignment-related binary operators.
    /// `=`
    Assignment = 800,
    /// Assignment multiplication-related binary operators.
    /// `*= /= %= **= *%= *|=`
    AssignmentMultiplication = 700,
    /// Assignment addition-related binary operators.
    /// `+= -= +%= -%= +|= -|=`
    AssignmentAddition = 600,
    /// Assignment shift-related binary operators.
    /// `<<= >>= <<|=`
    AssignmentShift = 500,
    /// Assignment elementwise-related binary operators.
    /// `&= ^= |=`
    AssignmentElementwise = 400,
    /// Assignment logical-related binary operators.
    /// `&&= ||= ??=`
    AssignmentBoolean = 300,
}

/// A TypeUnaryOperator is a type unary operator.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum TypeUnaryOperator {
    /// `!T`
    Not = 1811,
    /// `T!`
    Must = 1809,
    /// `newtype`
    Newtype = 1808,
    /// `type`
    Type = 1807,
    /// `readonly`
    Readonly = 1806,
    /// `typeof`
    Typeof = 1804,
    /// `keyof`
    Keyof = 1803,
    /// `as comptime`
    AsComptime = 1802,
}

impl TypeUnaryOperator {
    /// Get the precedence of the type unary operator.
    #[inline]
    pub fn precedence_group(&self) -> OperatorPrecedence {
        OperatorPrecedence::TypeUnary
    }

    /// Get the precedence of the type unary operator.
    #[inline]
    pub fn precedence(self) -> u16 {
        // just transmute the enum value to an u16
        self as u16
    }

    /// Whether the type unary operator is a prefix operator.
    #[inline]
    pub fn is_prefix(&self) -> bool {
        match self {
            TypeUnaryOperator::Not
            | TypeUnaryOperator::Newtype
            | TypeUnaryOperator::Type
            | TypeUnaryOperator::Readonly
            | TypeUnaryOperator::Typeof
            | TypeUnaryOperator::Keyof => true,
            TypeUnaryOperator::Must | TypeUnaryOperator::AsComptime => false,
        }
    }

    /// Whether the type unary operator is a postfix operator.
    #[inline]
    pub fn is_postfix(&self) -> bool {
        !self.is_prefix()
    }

    /// Convert a TokenType to a TypeUnaryOperator (if a direct mapping exists).
    #[inline]
    pub fn from_prefix_token(token_str: &str, _token_type: TokenType) -> Option<TypeUnaryOperator> {
        match token_str {
            // NOTE: newtype | type / readonly are disambiguated separately
            "typeof" => Some(TypeUnaryOperator::Typeof),
            "keyof" => Some(TypeUnaryOperator::Keyof),
            _ => None,
        }
    }

    /// Convert a TokenType to a TypeUnaryOperator (if a direct mapping exists).
    #[inline]
    pub fn from_postfix_token(
        token_str: &str,
        next_token_str: &str,
        _token_type: TokenType,
    ) -> Option<TypeUnaryOperator> {
        match (token_str, next_token_str) {
            ("as", "comptime") => Some(TypeUnaryOperator::AsComptime),
            _ => None,
        }
    }
}

/// A UnaryOperator is unary operator.
/// Relative order matches precedence. Also see OperatorPrecedence.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum UnaryOperator {
    /// `++`
    PostIncrement = 2010,
    /// `--`
    PostDecrement = 2009,
    /// `++`
    PreIncrement = 1909,
    /// `--`
    PreDecrement = 1908,
    /// `!`
    Not = 1907,
    /// `+`
    Plus = 1906,
    /// `-`
    Negate = 1905,
    /// `-%`
    WrappingNegate = 1904,
    /// `~`
    ElementwiseNot = 1903,
    /// `typeof`
    Typeof = 1900,
    /// `void`
    Void = 1899,
    /// `*`
    Dereference = 1902,
    /// `...`
    Spread = 1901,
}

impl UnaryOperator {
    /// Get the precedence of the unary operator.
    #[inline]
    pub fn precedence_group(&self) -> OperatorPrecedence {
        OperatorPrecedence::Prefix
    }

    /// Get the precedence of the unary operator.
    #[inline]
    pub fn precedence(self) -> u16 {
        // just transmute the enum value to an u16
        self as u16
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
            | UnaryOperator::WrappingNegate
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
            TokenType::WrappingSubtract => Some(UnaryOperator::WrappingNegate),
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

/// A TypeBinaryOperator is a type binary operator.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum TypeBinaryOperator {
    /// `in`
    In = 1006,
    /// `extends`
    Extends = 1002,
    /// `implements`
    Implements = 1001,
}

impl TypeBinaryOperator {
    /// Get the precedence of the type binary operator.
    #[inline]
    pub fn precedence_group(&self) -> OperatorPrecedence {
        OperatorPrecedence::TypeBinary
    }

    /// Get the precedence of the type binary operator.
    #[inline]
    pub fn precedence(self) -> u16 {
        // just transmute the enum value to an u16
        self as u16
    }

    /// Convert a TokenType to a TypeBinaryOperator (if a direct mapping exists).
    #[inline]
    pub fn from_token(token_str: &str, _token_type: TokenType) -> Option<TypeBinaryOperator> {
        match token_str {
            "in" => Some(TypeBinaryOperator::In),
            "extends" => Some(TypeBinaryOperator::Extends),
            "implements" => Some(TypeBinaryOperator::Implements),
            _ => None,
        }
    }
}

/// A BinaryOperator is an infix binary operator.
/// Relative order matches precedence. Also see OperatorPrecedence.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum BinaryOperator {
    // multiplication
    /// `*`
    Multiply = 1708,
    /// `*%`
    WrappingMultiply = 1707,
    /// `*|`
    SaturatingMultiply = 1706,
    /// `**`
    Exponent = 1705,
    /// `**%`
    WrappingExponent = 1704,
    /// `**|`
    SaturatingExponent = 1703,
    /// `/`
    Divide = 1702,
    /// `%`
    Remainder = 1701,

    // addition
    /// `+`
    Add = 1605,
    /// `+%`
    WrappingAdd = 1604,
    /// `+|`
    SaturatingAdd = 1603,
    /// `-`
    Subtract = 1602,
    /// `-%`
    WrappingSubtract = 1601,
    /// `-|`
    SaturatingSubtract = 1600,

    // shift
    /// `<<`
    ShiftLeft = 1502,
    /// `<<|`
    SaturatingShiftLeft = 1501,
    /// `>>`
    ShiftRight = 1500,
    /// `>>>`
    UnsignedShiftRight = 1503,

    // elementwise
    /// `&`
    ElementwiseAnd = 1402,
    /// `^`
    ElementwiseXor = 1401,
    /// `|`
    ElementwiseOr = 1400,

    // comparison
    /// `==`
    Equal = 1307,
    /// `!=`
    NotEqual = 1306,
    /// `===`
    EqualStrict = 1305,
    /// `!==`
    NotEqualStrict = 1304,
    /// `<`
    LessThan = 1303,
    /// `<=`
    LessThanOrEqual = 1302,
    /// `>`
    GreaterThan = 1301,
    /// `>=`
    GreaterThanOrEqual = 1300,

    // boolean
    /// `&&`
    And = 1202,
    /// `||`
    Or = 1201,
    /// `??`
    Coalesce = 1200,

    // container
    /// `in`
    In = 1102,
}

impl BinaryOperator {
    /// Get the precedence of the binary operator.
    #[inline]
    pub fn precedence_group(&self) -> OperatorPrecedence {
        match self {
            // multiplication
            BinaryOperator::Multiply => OperatorPrecedence::Multiplication,
            BinaryOperator::WrappingMultiply => OperatorPrecedence::Multiplication,
            BinaryOperator::SaturatingMultiply => OperatorPrecedence::Multiplication,
            BinaryOperator::Exponent => OperatorPrecedence::Multiplication,
            BinaryOperator::WrappingExponent => OperatorPrecedence::Multiplication,
            BinaryOperator::SaturatingExponent => OperatorPrecedence::Multiplication,
            BinaryOperator::Divide => OperatorPrecedence::Multiplication,
            BinaryOperator::Remainder => OperatorPrecedence::Multiplication,

            // addition
            BinaryOperator::Add => OperatorPrecedence::Addition,
            BinaryOperator::WrappingAdd => OperatorPrecedence::Addition,
            BinaryOperator::SaturatingAdd => OperatorPrecedence::Addition,
            BinaryOperator::Subtract => OperatorPrecedence::Addition,
            BinaryOperator::WrappingSubtract => OperatorPrecedence::Addition,
            BinaryOperator::SaturatingSubtract => OperatorPrecedence::Addition,

            // shift
            BinaryOperator::ShiftLeft => OperatorPrecedence::Shift,
            BinaryOperator::SaturatingShiftLeft => OperatorPrecedence::Shift,
            BinaryOperator::ShiftRight => OperatorPrecedence::Shift,
            BinaryOperator::UnsignedShiftRight => OperatorPrecedence::Shift,

            // elementwise
            BinaryOperator::ElementwiseAnd => OperatorPrecedence::Elementwise,
            BinaryOperator::ElementwiseXor => OperatorPrecedence::Elementwise,
            BinaryOperator::ElementwiseOr => OperatorPrecedence::Elementwise,

            // comparison
            BinaryOperator::Equal => OperatorPrecedence::Comparison,
            BinaryOperator::NotEqual => OperatorPrecedence::Comparison,
            BinaryOperator::EqualStrict => OperatorPrecedence::Comparison,
            BinaryOperator::NotEqualStrict => OperatorPrecedence::Comparison,
            BinaryOperator::LessThan => OperatorPrecedence::Comparison,
            BinaryOperator::LessThanOrEqual => OperatorPrecedence::Comparison,
            BinaryOperator::GreaterThan => OperatorPrecedence::Comparison,
            BinaryOperator::GreaterThanOrEqual => OperatorPrecedence::Comparison,

            // logical
            BinaryOperator::And => OperatorPrecedence::Boolean,
            BinaryOperator::Or => OperatorPrecedence::Boolean,
            BinaryOperator::Coalesce => OperatorPrecedence::Boolean,

            // container
            BinaryOperator::In => OperatorPrecedence::Container,
        }
    }

    /// Get the precedence of the binary operator.
    pub fn precedence(self) -> u16 {
        // just transmute the enum value to an u8
        self as u16
    }

    /// Convert a TokenType to a BinaryOperator (if a direct mapping exists).
    #[inline]
    pub fn from_token(token_str: &str, token_type: TokenType) -> Option<BinaryOperator> {
        match token_type {
            // multiplication
            TokenType::Multiply => Some(BinaryOperator::Multiply),
            TokenType::WrappingMultiply => Some(BinaryOperator::WrappingMultiply),
            TokenType::SaturatingMultiply => Some(BinaryOperator::SaturatingMultiply),
            TokenType::Exponent => Some(BinaryOperator::Exponent),
            TokenType::WrappingExponent => Some(BinaryOperator::WrappingExponent),
            TokenType::SaturatingExponent => Some(BinaryOperator::SaturatingExponent),
            TokenType::Divide => Some(BinaryOperator::Divide),
            TokenType::Remainder => Some(BinaryOperator::Remainder),

            // addition
            TokenType::Add => Some(BinaryOperator::Add),
            TokenType::WrappingAdd => Some(BinaryOperator::WrappingAdd),
            TokenType::SaturatingAdd => Some(BinaryOperator::SaturatingAdd),
            TokenType::Subtract => Some(BinaryOperator::Subtract),
            TokenType::WrappingSubtract => Some(BinaryOperator::WrappingSubtract),
            TokenType::SaturatingSubtract => Some(BinaryOperator::SaturatingSubtract),

            // shift
            TokenType::ShiftLeft => Some(BinaryOperator::ShiftLeft),
            TokenType::SaturatingShiftLeft => Some(BinaryOperator::SaturatingShiftLeft),
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

/// An AssignOperator is assignment type.
/// Relative order matches precedence. Also see OperatorPrecedence.
///
/// Examples:
/// ```
/// x = 1
/// x += 1
/// x >>= 1
/// x &= 1
/// x |= 1
/// ```
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum AssignOperator {
    /// `=`
    Assign = 800,

    // assignment multiplication
    /// `*=`
    MultiplyAssign = 708,
    /// `*%=`
    WrappingMultiplyAssign = 707,
    /// `*|=`
    SaturatingMultiplyAssign = 706,
    /// `**=`
    ExponentAssign = 705,
    /// `**%=`
    WrappingExponentAssign = 704,
    /// `**|`
    SaturatingExponentAssign = 703,
    /// `/=`
    DivideAssign = 702,
    /// `%=`
    RemainderAssign = 701,

    // assignment addition
    /// `+=`
    AddAssign = 606,
    /// `+%=`
    WrappingAddAssign = 605,
    /// `+|=`
    SaturatingAddAssign = 604,
    /// `-=`
    SubtractAssign = 603,
    /// `-%=`
    WrappingSubtractAssign = 602,
    /// `-|=`
    SaturatingSubtractAssign = 601,

    // assignment shift
    /// `<<=`
    ShiftLeftAssign = 503,
    /// `<<|=`
    SaturatingShiftLeftAssign = 502,
    /// `>>=`
    ShiftRightAssign = 501,
    /// `>>>=`
    UnsignedShiftRightAssign = 500,

    // assignment elementwise
    /// `&=`
    ElementwiseAndAssign = 403,
    /// `^=`
    ElementwiseXorAssign = 402,
    /// `|=`
    ElementwiseOrAssign = 401,

    // assignment logical
    /// `&&=`
    AndAssign = 303,
    /// `||=`
    OrAssign = 302,
    /// `??=`
    CoalesceAssign = 301,
}

impl AssignOperator {
    /// Get the precedence of the assignment type.
    #[inline]
    pub fn precedence_group(&self) -> OperatorPrecedence {
        match self {
            // assignment
            AssignOperator::Assign => OperatorPrecedence::Assignment,

            // assignment multiplication
            AssignOperator::MultiplyAssign
            | AssignOperator::WrappingMultiplyAssign
            | AssignOperator::SaturatingMultiplyAssign
            | AssignOperator::ExponentAssign
            | AssignOperator::WrappingExponentAssign
            | AssignOperator::SaturatingExponentAssign
            | AssignOperator::DivideAssign
            | AssignOperator::RemainderAssign => OperatorPrecedence::AssignmentMultiplication,

            // assignment addition
            AssignOperator::AddAssign
            | AssignOperator::WrappingAddAssign
            | AssignOperator::SaturatingAddAssign
            | AssignOperator::SubtractAssign
            | AssignOperator::WrappingSubtractAssign
            | AssignOperator::SaturatingSubtractAssign => OperatorPrecedence::AssignmentAddition,

            // assignment shift
            AssignOperator::ShiftLeftAssign
            | AssignOperator::SaturatingShiftLeftAssign
            | AssignOperator::ShiftRightAssign
            | AssignOperator::UnsignedShiftRightAssign => OperatorPrecedence::AssignmentShift,

            // assignment elementwise
            AssignOperator::ElementwiseAndAssign
            | AssignOperator::ElementwiseXorAssign
            | AssignOperator::ElementwiseOrAssign => OperatorPrecedence::AssignmentElementwise,

            // assignment logical
            AssignOperator::AndAssign
            | AssignOperator::OrAssign
            | AssignOperator::CoalesceAssign => OperatorPrecedence::AssignmentBoolean,
        }
    }

    /// Get the precedence of the assignment type.
    #[inline]
    pub fn precedence(self) -> u16 {
        // just transmute the enum value to an u16
        self as u16
    }

    /// Convert a TokenType to an AssignOperator (if a direct mapping exists).
    #[inline]
    pub fn from_token(token_type: TokenType) -> Option<AssignOperator> {
        match token_type {
            TokenType::Assign => Some(AssignOperator::Assign),

            // addition
            TokenType::AddAssign => Some(AssignOperator::AddAssign),
            TokenType::WrappingAddAssign => Some(AssignOperator::WrappingAddAssign),
            TokenType::SaturatingAddAssign => Some(AssignOperator::SaturatingAddAssign),
            TokenType::SubtractAssign => Some(AssignOperator::SubtractAssign),
            TokenType::WrappingSubtractAssign => Some(AssignOperator::WrappingSubtractAssign),
            TokenType::SaturatingSubtractAssign => Some(AssignOperator::SaturatingSubtractAssign),

            // multiplication
            TokenType::MultiplyAssign => Some(AssignOperator::MultiplyAssign),
            TokenType::WrappingMultiplyAssign => Some(AssignOperator::WrappingMultiplyAssign),
            TokenType::SaturatingMultiplyAssign => Some(AssignOperator::SaturatingMultiplyAssign),
            TokenType::ExponentAssign => Some(AssignOperator::ExponentAssign),
            TokenType::WrappingExponentAssign => Some(AssignOperator::WrappingExponentAssign),
            TokenType::SaturatingExponentAssign => Some(AssignOperator::SaturatingExponentAssign),
            TokenType::DivideAssign => Some(AssignOperator::DivideAssign),
            TokenType::RemainderAssign => Some(AssignOperator::RemainderAssign),

            // shift
            TokenType::ShiftLeftAssign => Some(AssignOperator::ShiftLeftAssign),
            TokenType::SaturatingShiftLeftAssign => Some(AssignOperator::SaturatingShiftLeftAssign),
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

    /// Convert an AssignOperator to a TokenType (if a direct mapping exists).
    #[inline]
    pub fn as_token_type(&self) -> TokenType {
        match self {
            AssignOperator::Assign => TokenType::Assign,

            // addition
            AssignOperator::AddAssign => TokenType::AddAssign,
            AssignOperator::WrappingAddAssign => TokenType::WrappingAddAssign,
            AssignOperator::SaturatingAddAssign => TokenType::SaturatingAddAssign,
            AssignOperator::SubtractAssign => TokenType::SubtractAssign,
            AssignOperator::WrappingSubtractAssign => TokenType::WrappingSubtractAssign,
            AssignOperator::SaturatingSubtractAssign => TokenType::SaturatingSubtractAssign,

            // multiplication
            AssignOperator::MultiplyAssign => TokenType::MultiplyAssign,
            AssignOperator::WrappingMultiplyAssign => TokenType::WrappingMultiplyAssign,
            AssignOperator::SaturatingMultiplyAssign => TokenType::SaturatingMultiplyAssign,
            AssignOperator::ExponentAssign => TokenType::ExponentAssign,
            AssignOperator::WrappingExponentAssign => TokenType::WrappingExponentAssign,
            AssignOperator::SaturatingExponentAssign => TokenType::SaturatingExponentAssign,
            AssignOperator::DivideAssign => TokenType::DivideAssign,
            AssignOperator::RemainderAssign => TokenType::RemainderAssign,

            // shift
            AssignOperator::ShiftLeftAssign => TokenType::ShiftLeftAssign,
            AssignOperator::SaturatingShiftLeftAssign => TokenType::SaturatingShiftLeftAssign,
            AssignOperator::ShiftRightAssign => TokenType::ShiftRightAssign,
            AssignOperator::UnsignedShiftRightAssign => TokenType::UnsignedShiftRightAssign,

            // elementwise
            AssignOperator::ElementwiseAndAssign => TokenType::ElementwiseAndAssign,
            AssignOperator::ElementwiseOrAssign => TokenType::ElementwiseOrAssign,
            AssignOperator::ElementwiseXorAssign => TokenType::ElementwiseXorAssign,

            // logical
            AssignOperator::AndAssign => TokenType::LogicalAndAssign,
            AssignOperator::OrAssign => TokenType::LogicalOrAssign,
            AssignOperator::CoalesceAssign => TokenType::CoalesceAssign,
        }
    }
}

/// An infix operator umbrella for binary, type-binary, and assignment operators.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum InfixOperator {
    /// A binary operator.
    Binary(BinaryOperator),
    /// A type binary operator.
    TypeBinary(TypeBinaryOperator),
    /// An assignment operator.
    Assign(AssignOperator),
}

impl InfixOperator {
    /// Get the precedence of the infix operator.
    #[inline]
    pub fn precedence_group(&self) -> OperatorPrecedence {
        match self {
            InfixOperator::Binary(binary_operator) => binary_operator.precedence_group(),
            InfixOperator::TypeBinary(type_binary_operator) => {
                type_binary_operator.precedence_group()
            }
            InfixOperator::Assign(assign_operator) => assign_operator.precedence_group(),
        }
    }

    /// Get the precedence of the infix operator.
    #[inline]
    pub fn precedence(self) -> u16 {
        match self {
            InfixOperator::Binary(binary_operator) => binary_operator.precedence(),
            InfixOperator::TypeBinary(type_binary_operator) => type_binary_operator.precedence(),
            InfixOperator::Assign(assign_operator) => assign_operator.precedence(),
        }
    }
}
