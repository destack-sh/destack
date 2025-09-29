use dyst_source::PathId;

use crate::{
    Argument, ArrayLiteral, Block, Break, Call, Continue, Defer, Enum, For, Function, If,
    Implement, Index, Let, Loop, Match, Module, Node, NodeId, NodeType, RangeLiteral, Return,
    ScalarLiteral, ScopedMutability, Struct, StructLiteral, Trait, Try, TupleLiteral, Type, Union,
    Use, While, With,
};

/// The operator group (for precedence parsing).
///
/// Precedence:
/// ```
/// !x -x -%x ~x *x &x            // prefix
/// x() x[] x{} x as y x? x ?? y  // postfix
/// * / % ** *% *|                // multiplication
/// + - +% -% +| -|               // addition
/// << >> <<|                     // shift
/// & ^ |                         // elementwise
/// == != < > <= >=               // comparison
/// && ||                         // logical
/// =                             // assignment
/// *= /= %= **= *%= *|=          // assignment multiplication
/// += -= +%= -%= +|= -|=         // assignment addition
/// <<= >>= <<|=                  // assignment shift
/// &= ^= |=                      // assignment elementwise
/// &&= ||=                       // assignment logical
/// ```
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum OperatorPrecedence {
    /// Unary prefix operators.
    /// `!x -x -%x ~x &x *x`
    Prefix = 240,
    /// Unary postfix operators.
    /// `x() x[] x{} x as y x? x ?? y``
    Postfix = 230,
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
    Not = 246,
    /// `-`
    Negate = 245,
    /// `-%`
    WrappingNegate = 244,
    /// `~`
    ElementwiseNot = 243,
    /// `*`
    Dereference = 242,
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
    pub r#type: NodeId<Type>,
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

    /// Alias reference to some path, statically parameterized (as an Expression).
    Path {
        path: PathId,
        static_arguments: Option<Vec<NodeId<Argument>>>,
    },
    /// Literal scalar value (as an Expression, see ScalarLiteral).
    ScalarLiteral(NodeId<ScalarLiteral>),
    /// Range literal (as an Expression, see RangeLiteral).
    RangeLiteral(NodeId<RangeLiteral>),
    /// Array literal (as an Expression, see ArrayLiteral).
    ArrayLiteral(NodeId<ArrayLiteral>),
    /// Tuple literal (as an Expression, see TupleLiteral).
    TupleLiteral(NodeId<TupleLiteral>),
    /// Struct literal (as an Expression, see StructLiteral).
    StructLiteral(NodeId<StructLiteral>),

    /// Unary operation (prefix as Expression).
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
    /// A Call is call to a function (postfix as an Expression, see Call).
    Call(NodeId<Call>),
    /// As casting (postfix as an Expression, see As).
    Cast(NodeId<Cast>),
    /// Maybe unwrap an expression with `?` and propagate (postfix as an Expression).
    Maybe(NodeId<Expression>),
    /// Must unwrap an expression with `!` and propagate (postfix as an Expression).
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

impl Node for Expression {
    const KIND: NodeType = NodeType::Expression;
}
