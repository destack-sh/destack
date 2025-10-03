use dyst_ast::StringId;

use crate::{Argument, Block, MatchCase, Node, NodeId, NodeType, PathId};

/// A UnaryOperator is unary operator.
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
    /// `$`
    Virtual = 232,
    /// `..`
    Spread = 231,
}

/// A BinaryOperator is an infix binary operator.
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

/// An AssignOperator is assignment type.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum AssignOperator {
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

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// Unary operation (except reference/dereference, e.g., `-x`).
    Unary {
        operator: UnaryOperator,
        right: NodeId<Expression>,
    },
    /// Reference operation (e.g., `&x`).
    Reference { right: NodeId<Expression> },
    /// Dereference operation (e.g., `*x`).
    Dereference { right: NodeId<Expression> },
    /// Binary operation.
    Binary {
        left: NodeId<Expression>,
        operator: BinaryOperator,
        right: NodeId<Expression>,
    },
    /// Assignment (e.g., `x = y`).
    AssignDirect {
        left: NodeId<Expression>,
        right: NodeId<Expression>,
    },
    /// Assignment with operator (except direct assignment, e.g., `x += y`).
    AssignBinary {
        left: NodeId<Expression>,
        operator: AssignOperator,
        right: NodeId<Expression>,
    },
    /// Drop locals.
    Drop,
    /// Member access.
    Member {
        left: NodeId<Expression>,
        path: PathId,
    },
    /// Call to a function.
    Call {
        left: NodeId<Expression>,
        dynamic_arguments: Vec<NodeId<Argument>>,
    },
    /// Index into an array or slice.
    Index,
    /// Cast to a type.
    Cast,
    /// --------------------------------
    /// Literals.
    /// --------------------------------
    /// Scalar literal value.
    ScalarLiteral,
    /// Struct creation.
    StructLiteral {
        r#type: NodeId<Expression>,
        fields: Vec<NodeId<Argument>>,
    },
    /// Tuple creation.
    TupleLiteral { elements: Vec<NodeId<Argument>> },
    /// Array creation.
    ArrayLiteral { elements: Vec<NodeId<Expression>> },
    /// --------------------------------
    /// Control flow.
    /// --------------------------------
    /// If expression.
    If {
        condition: NodeId<Expression>,
        then_block: NodeId<Block>,
        else_block: Option<NodeId<Expression>>,
    },
    /// Loop expression.
    Loop {
        condition: NodeId<Expression>,
        body: NodeId<Block>,
        source: LoopSource,
    },
    /// Match expression.
    Match {
        value: NodeId<Expression>,
        cases: Vec<NodeId<MatchCase>>,
    },
    /// Break expression.
    Break {
        label: Option<StringId>,
        value: Option<NodeId<Expression>>,
    },
    /// Continue expression.
    Continue { label: Option<StringId> },
    /// Defer expression.
    Defer { body: NodeId<Expression> },
    /// Return expression.
    Return { value: Option<NodeId<Expression>> },

    /// Error expression.
    Error,
}

impl Node for Expression {
    const KIND: NodeType = NodeType::Expression;
}

/// A LoopSource is where the loop was lowered from.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum LoopSource {
    /// For loop.
    For,
    /// While loop.
    While,
    /// Loop loop.
    Loop,
}
