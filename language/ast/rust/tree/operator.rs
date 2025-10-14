use crate::TokenType;

/// The operator group (for precedence parsing).
///
/// Precedence:
/// ```
/// x() x[] x{} x? x!         // postfix
/// !x -x -%x ~x *x &x ..x    // prefix
/// * / % *% *|               // multiplication
/// + - +% -% +| -|           // addition
/// << >> <<|                 // shift
/// & ^ |                     // elementwise
/// == != < > <= >=           // comparison
/// && || ?? as               // logical
/// =                         // assignment
/// *= /= %= **= *%= *|=      // assignment multiplication
/// += -= +%= -%= +|= -|=     // assignment addition
/// <<= >>= <<|=              // assignment shift
/// &= ^= |=                  // assignment elementwise
/// &&= ||=                   // assignment logical
/// ```
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum OperatorPrecedence {
    /// Unary postfix operators.
    /// `x() x[] x{} x? x!`
    Postfix = 240,
    /// Unary prefix operators.
    /// `!x -x -%x ~x &x *x ..x`
    Prefix = 230,
    /// Multiplication-related binary operators.
    /// `* / % ** *% *|`
    Multiplication = 220,
    /// Addition-related binary operators.
    /// `+ - +% -% +| -|`
    Addition = 210,
    /// Shift-related binary operators.
    /// `<< >> <<|`
    Shift = 200,
    /// Elementwise-related binary operators.
    /// `& ^ |`
    Elementwise = 190,
    /// Comparison-related binary operators.
    /// `== != < > <= >=`
    Comparison = 180,
    /// Logical-related binary operators.
    /// `&& ||`
    Logical = 170,
    /// Assignment-related binary operators.
    /// `=`
    Assignment = 160,
    /// Assignment multiplication-related binary operators.
    /// `*= /= %= **= *%= *|=`
    AssignmentMultiplication = 150,
    /// Assignment addition-related binary operators.
    /// `+= -= +%= -%= +|= -|=`
    AssignmentAddition = 140,
    /// Assignment shift-related binary operators.
    /// `<<= >>= <<|=`
    AssignmentShift = 130,
    /// Assignment elementwise-related binary operators.
    /// `&= ^= |=`
    AssignmentElementwise = 120,
    /// Assignment logical-related binary operators.
    /// `&&= ||=`
    AssignmentLogical = 110,
}

/// A UnaryOperator is unary operator.
/// Relative order matches precedence. Also see OperatorPrecedence.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum UnaryOperator {
    /// `!`
    Not = 237,
    /// `-`
    Negate = 236,
    /// `-%`
    WrappingNegate = 235,
    /// `~`
    ElementwiseNot = 234,
    /// `*`
    Dereference = 233,
    /// `$`
    Virtual = 232,
    /// `..`
    Spread = 231,
}

impl UnaryOperator {
    /// Get the precedence of the unary operator.
    #[inline]
    pub fn precedence_group(&self) -> OperatorPrecedence {
        OperatorPrecedence::Prefix
    }

    /// Get the precedence of the unary operator.
    #[inline]
    pub fn precedence(self) -> u8 {
        // just transmute the enum value to an u8
        self as u8
    }

    /// Covnert a TokenType to a UnaryOperator (if a direct mapping exists).
    #[inline]
    pub fn from_token(token_type: TokenType) -> Option<UnaryOperator> {
        match token_type {
            TokenType::Not => Some(UnaryOperator::Not),
            TokenType::Subtract => Some(UnaryOperator::Negate),
            TokenType::WrappingSubtract => Some(UnaryOperator::WrappingNegate),
            TokenType::Multiply => Some(UnaryOperator::Dereference),
            TokenType::ElementwiseNot => Some(UnaryOperator::ElementwiseNot),
            TokenType::Virtual => Some(UnaryOperator::Virtual),
            TokenType::Range => Some(UnaryOperator::Spread),
            TokenType::RangeWide => Some(UnaryOperator::Spread),
            _ => None,
        }
    }

    /// Convert a UnaryOperator to a TokenType (if a direct mapping exists).
    #[inline]
    pub fn as_token(&self) -> TokenType {
        match self {
            UnaryOperator::Not => TokenType::Not,
            UnaryOperator::Negate => TokenType::Subtract,
            UnaryOperator::WrappingNegate => TokenType::WrappingSubtract,
            UnaryOperator::ElementwiseNot => TokenType::ElementwiseNot,
            UnaryOperator::Dereference => TokenType::Multiply,
            UnaryOperator::Virtual => TokenType::Virtual,
            UnaryOperator::Spread => TokenType::Range,
        }
    }
}

/// A BinaryOperator is an infix binary operator.
/// Relative order matches precedence. Also see OperatorPrecedence.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum BinaryOperator {
    // multiplication
    /// `*`
    Multiply = 224,
    /// `*%`
    WrappingMultiply = 223,
    /// `*|`
    SaturatingMultiply = 222,
    /// `/`
    Divide = 221,
    /// `%`
    Remainder = 220,

    // addition
    /// `+`
    Add = 215,
    /// `+%`
    WrappingAdd = 214,
    /// `+|`
    SaturatingAdd = 213,
    /// `-`
    Subtract = 212,
    /// `-%`
    WrappingSubtract = 211,
    /// `-|`
    SaturatingSubtract = 210,

    // shift
    /// `<<`
    ShiftLeft = 202,
    /// `<<|`
    SaturatingShiftLeft = 201,
    /// `>>`
    ShiftRight = 200,

    // elementwise
    /// `&`
    ElementwiseAnd = 192,
    /// `^`
    ElementwiseXor = 191,
    /// `|`
    ElementwiseOr = 190,

    // comparison
    /// `==`
    Equal = 185,
    /// `!=`
    NotEqual = 184,
    /// `<`
    LessThan = 183,
    /// `<=`
    LessThanOrEqual = 182,
    /// `>`
    GreaterThan = 181,
    /// `>=`
    GreaterThanOrEqual = 180,

    // logical
    /// `&&`
    And = 173,
    /// `||`
    Or = 172,
    /// `??`
    Coalesce = 171,
    /// `as`
    Cast = 170,
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

            // elementwise
            BinaryOperator::ElementwiseAnd => OperatorPrecedence::Elementwise,
            BinaryOperator::ElementwiseXor => OperatorPrecedence::Elementwise,
            BinaryOperator::ElementwiseOr => OperatorPrecedence::Elementwise,

            // comparison
            BinaryOperator::Equal => OperatorPrecedence::Comparison,
            BinaryOperator::NotEqual => OperatorPrecedence::Comparison,
            BinaryOperator::LessThan => OperatorPrecedence::Comparison,
            BinaryOperator::LessThanOrEqual => OperatorPrecedence::Comparison,
            BinaryOperator::GreaterThan => OperatorPrecedence::Comparison,
            BinaryOperator::GreaterThanOrEqual => OperatorPrecedence::Comparison,

            // logical
            BinaryOperator::And => OperatorPrecedence::Logical,
            BinaryOperator::Or => OperatorPrecedence::Logical,
            BinaryOperator::Coalesce => OperatorPrecedence::Logical,
            BinaryOperator::Cast => OperatorPrecedence::Logical,
        }
    }

    /// Get the precedence of the binary operator.
    pub fn precedence(self) -> u8 {
        // just transmute the enum value to an u8
        self as u8
    }

    /// Convert a TokenType to a BinaryOperator (if a direct mapping exists).
    #[inline]
    pub fn from_token(token_str: &str, token_type: TokenType) -> Option<BinaryOperator> {
        match token_type {
            // multiplication
            TokenType::Multiply => Some(BinaryOperator::Multiply),
            TokenType::WrappingMultiply => Some(BinaryOperator::WrappingMultiply),
            TokenType::SaturatingMultiply => Some(BinaryOperator::SaturatingMultiply),
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

            // elementwise
            TokenType::ElementwiseAnd => Some(BinaryOperator::ElementwiseAnd),
            TokenType::ElementwiseXor => Some(BinaryOperator::ElementwiseXor),
            TokenType::ElementwiseOr => Some(BinaryOperator::ElementwiseOr),

            // comparison
            TokenType::Equal => Some(BinaryOperator::Equal),
            TokenType::EqualWide => Some(BinaryOperator::Equal),
            TokenType::NotEqual => Some(BinaryOperator::NotEqual),
            TokenType::NotEqualWide => Some(BinaryOperator::NotEqual),
            TokenType::LessThan => Some(BinaryOperator::LessThan),
            TokenType::LessThanOrEqual => Some(BinaryOperator::LessThanOrEqual),
            TokenType::GreaterThan => Some(BinaryOperator::GreaterThan),
            TokenType::GreaterThanOrEqual => Some(BinaryOperator::GreaterThanOrEqual),

            // logical
            TokenType::LogicalAnd => Some(BinaryOperator::And),
            TokenType::LogicalOr => Some(BinaryOperator::Or),
            TokenType::Coalesce => Some(BinaryOperator::Coalesce),
            TokenType::Identifier if token_str == "as" => Some(BinaryOperator::Cast),
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
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum AssignOperator {
    /// `=`
    Assign = 160,

    // assignment multiplication
    /// `*=`
    MultiplyAssign = 154,
    /// `*%=`
    WrappingMultiplyAssign = 153,
    /// `*|=`
    SaturatingMultiplyAssign = 152,
    /// `/=`
    DivideAssign = 151,
    /// `%=`
    RemainderAssign = 150,

    // assignment addition
    /// `+=`
    AddAssign = 145,
    /// `+%=`
    WrappingAddAssign = 144,
    /// `+|=`
    SaturatingAddAssign = 143,
    /// `-=`
    SubtractAssign = 142,
    /// `-%=`
    WrappingSubtractAssign = 141,
    /// `-|=`
    SaturatingSubtractAssign = 140,

    // assignment shift
    /// `<<=`
    ShiftLeftAssign = 132,
    /// `<<|=`
    SaturatingShiftLeftAssign = 131,
    /// `>>=`
    ShiftRightAssign = 130,

    // assignment elementwise
    /// `&=`
    ElementwiseAndAssign = 122,
    /// `^=`
    ElementwiseXorAssign = 121,
    /// `|=`
    ElementwiseOrAssign = 120,

    // assignment logical
    /// `&&=`
    AndAssign = 111,
    /// `||=`
    OrAssign = 110,
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
            | AssignOperator::ShiftRightAssign => OperatorPrecedence::AssignmentShift,

            // assignment elementwise
            AssignOperator::ElementwiseAndAssign
            | AssignOperator::ElementwiseXorAssign
            | AssignOperator::ElementwiseOrAssign => OperatorPrecedence::AssignmentElementwise,

            // assignment logical
            AssignOperator::AndAssign | AssignOperator::OrAssign => {
                OperatorPrecedence::AssignmentLogical
            }
        }
    }

    /// Get the precedence of the assignment type.
    #[inline]
    pub fn precedence(self) -> u8 {
        // just transmute the enum value to an u8
        self as u8
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
            TokenType::DivideAssign => Some(AssignOperator::DivideAssign),
            TokenType::RemainderAssign => Some(AssignOperator::RemainderAssign),

            // shift
            TokenType::ShiftLeftAssign => Some(AssignOperator::ShiftLeftAssign),
            TokenType::SaturatingShiftLeftAssign => Some(AssignOperator::SaturatingShiftLeftAssign),
            TokenType::ShiftRightAssign => Some(AssignOperator::ShiftRightAssign),

            // elementwise
            TokenType::ElementwiseAndAssign => Some(AssignOperator::ElementwiseAndAssign),
            TokenType::ElementwiseOrAssign => Some(AssignOperator::ElementwiseOrAssign),
            TokenType::ElementwiseXorAssign => Some(AssignOperator::ElementwiseXorAssign),

            // logical
            TokenType::LogicalAndAssign => Some(AssignOperator::AndAssign),
            TokenType::LogicalOrAssign => Some(AssignOperator::OrAssign),

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
            AssignOperator::DivideAssign => TokenType::DivideAssign,
            AssignOperator::RemainderAssign => TokenType::RemainderAssign,

            // shift
            AssignOperator::ShiftLeftAssign => TokenType::ShiftLeftAssign,
            AssignOperator::SaturatingShiftLeftAssign => TokenType::SaturatingShiftLeftAssign,
            AssignOperator::ShiftRightAssign => TokenType::ShiftRightAssign,

            // elementwise
            AssignOperator::ElementwiseAndAssign => TokenType::ElementwiseAndAssign,
            AssignOperator::ElementwiseOrAssign => TokenType::ElementwiseOrAssign,
            AssignOperator::ElementwiseXorAssign => TokenType::ElementwiseXorAssign,

            // logical
            AssignOperator::AndAssign => TokenType::LogicalAndAssign,
            AssignOperator::OrAssign => TokenType::LogicalOrAssign,
        }
    }
}

/// An InfixOperator is an umbrella for either a binary or assignment operator.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum InfixOperator {
    /// A binary operator.
    Binary(BinaryOperator),
    /// An assignment operator.
    Assign(AssignOperator),
}

impl InfixOperator {
    /// Get the precedence of the infix operator.
    #[inline]
    pub fn precedence_group(&self) -> OperatorPrecedence {
        match self {
            InfixOperator::Binary(binary_operator) => binary_operator.precedence_group(),
            InfixOperator::Assign(assign_operator) => assign_operator.precedence_group(),
        }
    }

    /// Get the precedence of the infix operator.
    #[inline]
    pub fn precedence(self) -> u8 {
        match self {
            InfixOperator::Binary(binary_operator) => binary_operator.precedence(),
            InfixOperator::Assign(assign_operator) => assign_operator.precedence(),
        }
    }
}
