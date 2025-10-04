use dyst_ast::StringId;

use crate::{Argument, Block, Definition, Node, NodeId, NodeType, PathId, Pattern, Type};

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
    // Definition
    Definition(NodeId<Definition>),

    // Block
    Block(NodeId<Block>),

    // With
    WithDeclaration {
        declarations: Vec<NodeId<WithDeclaration>>,
    },

    // WithAssertion
    WithAssertion {
        declarations: Vec<NodeId<WithAssertion>>,
    },

    // Use
    Use {
        declarations: Vec<NodeId<UseItem>>,
    },

    /// Unary operation (except reference/dereference, e.g., `-x`).
    Unary {
        operator: UnaryOperator,
        right: NodeId<Expression>,
    },
    /// Reference operation (e.g., `&x`).
    Reference {
        right: NodeId<Expression>,
    },
    /// Dereference operation (e.g., `*x`).
    Dereference {
        right: NodeId<Expression>,
    },
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
    Index {
        left: NodeId<Expression>,
        right: NodeId<Expression>,
    },
    /// Cast to a type.
    Cast {
        value: NodeId<Expression>,
        ty: NodeId<Type>,
    },

    /// --------------------------------
    /// Literals.
    /// --------------------------------

    /// Scalar literal value.
    ScalarLiteral,
    /// Struct creation.
    StructLiteral {
        ty: NodeId<Type>,
        fields: Vec<NodeId<Argument>>,
    },
    /// Tuple creation.
    TupleLiteral {
        elements: Vec<NodeId<Argument>>,
    },
    /// Array creation.
    ArrayLiteral {
        elements: Vec<NodeId<Expression>>,
    },

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
    Continue {
        label: Option<StringId>,
    },
    /// Defer expression.
    Defer {
        body: NodeId<Expression>,
    },
    /// Return expression.
    Return {
        value: Option<NodeId<Expression>>,
    },

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

/// A UseItem is an item to use in a use clause.
#[derive(Debug, Clone, PartialEq)]
pub struct UseItem {
    /// The source of the item.
    pub source: PathId,
    /// The source name of the item (like `foo` in `foo as bar`)
    pub name: StringId,
    /// The alias to use for the item (like `bar` in `foo as bar`)
    pub alias: Option<StringId>,
}

impl Node for UseItem {
    const KIND: NodeType = NodeType::UseItem;
}

/// A WithDeclaration is a single clause in a with declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct WithDeclaration {
    /// The item to use (like `Foo.Bar` in `with Foo.Bar`)
    pub target: NodeId<Expression>,
}

impl Node for WithDeclaration {
    const KIND: NodeType = NodeType::WithDeclaration;
}

/// A WithAssertion is a single clause in a with declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct WithAssertion {
    /// The target to assert (like `T` in `with T: int32`)
    pub target: NodeId<Expression>,
    /// The assertion type (like `int32` in `with T: int32`)
    pub assertion: NodeId<Expression>,
}

impl Node for WithAssertion {
    const KIND: NodeType = NodeType::WithAssertion;
}

/// A MatchCase is a match case inside a Match expression.
/// MatchCases can be any Pattern and can have an optional `if` guard.
#[derive(Debug, Clone, PartialEq)]
pub enum MatchCase {
    /// A match case with an expression body.
    Expression {
        pattern: NodeId<Pattern>,
        body: NodeId<Expression>,
        guard: Option<NodeId<Expression>>,
    },
    /// A match case with a block body.
    Block {
        pattern: NodeId<Pattern>,
        body: NodeId<Block>,
        guard: Option<NodeId<Expression>>,
    },
}

impl Node for MatchCase {
    const KIND: NodeType = NodeType::MatchCase;
}
