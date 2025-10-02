use dyst_source::PathId;
use dyst_token::TokenType;

use crate::{
    Argument, ArrayLiteral, Block, Break, Call, Continue, Defer, Enum, For, Function, If,
    Implement, Index, Let, Loop, Match, Module, Node, NodeId, NodeType, RangeLiteral, Return,
    ScalarLiteral, ScopedMutability, Struct, StructLiteral, Trait, Try, TupleLiteral, TypeLiteral,
    Union, Use, While, With,
};

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
/// && ||                     // logical
/// ?? as                     // coalesce
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
    And = 171,
    /// `||`
    Or = 170,
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

/// An InfixOperator is an umbrella for either a binary or assignment operator.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum InfixOperator {
    /// A binary operator.
    Binary(BinaryOperator),
    /// An assignment operator.
    Assign(AssignOperator),
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
        }
    }

    /// Get the precedence of the binary operator.
    pub fn precedence(self) -> u8 {
        // just transmute the enum value to an u8
        self as u8
    }

    /// Convert a TokenType to a BinaryOperator (if a direct mapping exists).
    #[inline]
    pub fn from_token_type(token_type: TokenType) -> Option<BinaryOperator> {
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
            TokenType::NotEqual => Some(BinaryOperator::NotEqual),
            TokenType::LessThan => Some(BinaryOperator::LessThan),
            TokenType::LessThanOrEqual => Some(BinaryOperator::LessThanOrEqual),
            TokenType::GreaterThan => Some(BinaryOperator::GreaterThan),
            TokenType::GreaterThanOrEqual => Some(BinaryOperator::GreaterThanOrEqual),

            // logical
            TokenType::LogicalAnd => Some(BinaryOperator::And),
            TokenType::LogicalOr => Some(BinaryOperator::Or),

            _ => None,
        }
    }
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
    pub fn from_token_type(token_type: TokenType) -> Option<UnaryOperator> {
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
    pub fn as_token_type(&self) -> TokenType {
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
    pub fn from_token_type(token_type: TokenType) -> Option<AssignOperator> {
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

/// A Cast is an `as` infallible type cast or transmutation.
///
/// Examples:
/// ```
/// x as int32
/// x as Vector2
/// y() as Mesh<Dims: 2>
/// ```
///
#[derive(Debug, Clone, PartialEq)]
pub struct Cast {
    /// The receiver of the cast (including expression to cast).
    pub receiver: NodeId<Expression>,
    /// The type to cast to.
    pub r#type: NodeId<Expression>,
}

impl Node for Cast {
    const KIND: NodeType = NodeType::Cast;
}

/// A Coalesce is an `??` coalesce operation.
///
/// Examples:
/// ```
/// x ?? 0
/// x ?? false
/// y() ?? 0
/// ```
///
#[derive(Debug, Clone, PartialEq)]
pub struct Coalesce {
    /// The receiver of the coalesce (including expression to coalesce).
    pub receiver: NodeId<Expression>,
    /// The default value to return if the expression is `null`.
    pub default: NodeId<Expression>,
}

impl Node for Coalesce {
    const KIND: NodeType = NodeType::Coalesce;
}

/// An Expression is a generic container for value-producing forms.
///
/// Some Expressions are "place Expressions" and can be read from and written to,
///  that is, they have a place in memory we can point to and get the address of.
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// Module definition (used as an Expression, see Module).
    Module(NodeId<Module>),
    /// Struct definition (used as an Expression, see Struct).
    Struct(NodeId<Struct>),
    /// Enum definition (used as an Expression, see Enum).
    Enum(NodeId<Enum>),
    /// Union definition (used as an Expression, see Union).
    Union(NodeId<Union>),
    /// Trait definition (used as an Expression, see Trait).
    Trait(NodeId<Trait>),
    /// Implement definition (used as an Expression, see Implement).
    Implement(NodeId<Implement>),
    /// Function definition (used as an Expression, see Function).
    Function(NodeId<Function>),
    /// Block of Statements (as an Expression, see Block).
    Block(NodeId<Block>),

    /// With declaration for context management (see With).
    With(NodeId<With>),
    /// Use declaration for dependency management (see Use).
    Use(NodeId<Use>),
    /// Let or var binding (as an Expression, see Let).
    Let(NodeId<Let>),
    /// An If is an if/then/else expression (as an Expression, see If).
    If(NodeId<If>),
    /// A While is a while loop (as an Expression, see While).
    While(NodeId<While>),
    /// A For is a for loop (as an Expression, see For).
    For(NodeId<For>),
    /// A Loop is an unconditional loop (as an Expression, see Loop).
    Loop(NodeId<Loop>),
    /// A Try is try/catch statement (as an Expression, see Try).
    Try(NodeId<Try>),
    /// A Match is match expression (as an Expression, see Match).
    Match(NodeId<Match>),
    /// Break out of a scope (as an Expression, see Break).
    Break(NodeId<Break>),
    /// Continue to the next iteration of a scope (as an Expression, see Continue).
    Continue(NodeId<Continue>),
    /// Defer expression until scope exit (as an Expression, see Defer).
    Defer(NodeId<Defer>),
    /// Return expression (as an Expression, see Return).
    Return(NodeId<Return>),

    // NOTE #Incomplete: multiply parameterized Expression Paths?
    //  (like `Foo<int32, boolean>.Bar<Yes: true>`)
    /// Alias reference to some path, statically parameterized (as an Expression).
    Path {
        path: PathId,
        static_arguments: Option<Vec<NodeId<Argument>>>,
    },
    /// Literal scalar value (as an Expression, see ScalarLiteral).
    ScalarLiteral(NodeId<ScalarLiteral>),
    /// Type literal (as an Expression, see TypeLiteral).
    TypeLiteral(NodeId<TypeLiteral>),
    /// Range literal (as an Expression, see RangeLiteral).
    RangeLiteral(NodeId<RangeLiteral>),
    /// Array literal (as an Expression, see ArrayLiteral).
    ArrayLiteral(NodeId<ArrayLiteral>),
    /// Tuple literal (as an Expression, see TupleLiteral).
    TupleLiteral(NodeId<TupleLiteral>),
    /// Struct literal (as an Expression, see StructLiteral).
    StructLiteral(NodeId<StructLiteral>),

    /// Parenthesized expression (as an Expression).
    Parenthesized { expression: NodeId<Expression> },
    /// Unary operation (simple prefix as Expression).
    Unary {
        operator: UnaryOperator,
        right: NodeId<Expression>,
    },
    /// Reference operation (prefix as an Expression).
    Reference {
        mutability: ScopedMutability,
        right: NodeId<Expression>,
    },
    /// Member access (postfix as an Expression, see Member).
    Member {
        receiver: NodeId<Expression>,
        path: PathId,
    },
    /// Index access (postfix as an Expression, see Index).
    Index(NodeId<Index>),
    /// Call to a function (postfix as an Expression, see Call).
    Call(NodeId<Call>),
    /// Cast to a type (postfix as an Expression, see As).
    Cast(NodeId<Cast>),
    /// Maybe unwrap an expression with `?` and propagate (postfix as an Expression).
    Maybe(NodeId<Expression>),
    /// Force unwrap an expression with `!` and propagate (postfix as an Expression).
    Must(NodeId<Expression>),
    /// Coalesce an expression with `??` (postfix as an Expression).
    Coalesce(NodeId<Coalesce>),
    /// Binary operation (infix between Expressions, see BinaryOperator).
    Binary {
        left: NodeId<Expression>,
        operator: BinaryOperator,
        right: NodeId<Expression>,
    },
    /// Assignment operation (infix as an Expression, see AssignOperator).
    Assign {
        left: NodeId<Expression>,
        operator: AssignOperator,
        right: NodeId<Expression>,
    },

    /// Error placeholder.
    Error,
}

impl Expression {
    /// Map the expression to its inner node type (if any).
    pub fn to_wrapper_node_type(&self) -> Option<NodeType> {
        match self {
            // definitions
            Expression::Module(_) => Some(NodeType::Module),
            Expression::Struct(_) => Some(NodeType::Struct),
            Expression::Enum(_) => Some(NodeType::Enum),
            Expression::Union(_) => Some(NodeType::Union),
            Expression::Trait(_) => Some(NodeType::Trait),
            Expression::Implement(_) => Some(NodeType::Implement),
            Expression::Function(_) => Some(NodeType::Function),
            Expression::Block(_) => Some(NodeType::Block),

            // declarations
            Expression::With(_) => Some(NodeType::With),
            Expression::Use(_) => Some(NodeType::Use),
            Expression::Let(_) => Some(NodeType::Let),
            Expression::If(_) => Some(NodeType::If),
            Expression::While(_) => Some(NodeType::While),
            Expression::For(_) => Some(NodeType::For),
            Expression::Loop(_) => Some(NodeType::Loop),
            Expression::Try(_) => Some(NodeType::Try),
            Expression::Match(_) => Some(NodeType::Match),
            Expression::Break(_) => Some(NodeType::Break),
            Expression::Continue(_) => Some(NodeType::Continue),
            Expression::Defer(_) => Some(NodeType::Defer),
            Expression::Return(_) => Some(NodeType::Return),

            // literals
            Expression::Path { .. } => None,
            Expression::ScalarLiteral(_) => Some(NodeType::ScalarLiteral),
            Expression::TypeLiteral(_) => Some(NodeType::TypeLiteral),
            Expression::RangeLiteral(_) => Some(NodeType::RangeLiteral),
            Expression::TupleLiteral(_) => Some(NodeType::TupleLiteral),
            Expression::ArrayLiteral(_) => Some(NodeType::ArrayLiteral),
            Expression::StructLiteral(_) => Some(NodeType::StructLiteral),

            // unary operations
            Expression::Parenthesized { .. } => None,
            Expression::Unary { .. } => None,
            Expression::Reference { .. } => None,

            // postfix operations
            Expression::Member { .. } => None,
            Expression::Index(_) => Some(NodeType::Index),
            Expression::Call(_) => Some(NodeType::Call),
            Expression::Cast(_) => Some(NodeType::Cast),
            Expression::Maybe(_) => None,
            Expression::Must(_) => None,
            Expression::Coalesce(_) => Some(NodeType::Coalesce),

            // binary operations
            Expression::Binary { .. } => None,
            Expression::Assign { .. } => None,

            // error
            Expression::Error => None,
        }
    }
}

impl Node for Expression {
    const KIND: NodeType = NodeType::Expression;
}
