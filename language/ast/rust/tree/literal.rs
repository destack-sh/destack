use crate::{Expression, FloatType, IntType, Node, NodeId, NodeType, StringId, Type};

/// A ScalarLiteral is literal scalar value node in the AST.
///
/// Examples:
/// ```
/// void
/// null
/// true
/// false
/// 1
/// 0x21
/// 1.0
/// "Hello, world!"
/// 'a'
/// b'a'
/// b"abc"
/// 0x1234
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum ScalarLiteral {
    /// Unknown / uninitialized value.
    Undefined,
    /// Void / empty / unit type.
    Void,
    /// Null value for optionals.
    Null,
    /// Boolean value.
    Boolean(bool),
    /// Byte value.
    Byte(u8),
    /// Integer value.
    Integer(i64, IntType),
    /// Float value.
    Float(f64, FloatType),
    /// Character value.
    Character(char),
    /// String value.
    String(StringId),
    /// Byte string value.
    ByteString(Vec<u8>),
}

impl Node for ScalarLiteral {
    const KIND: NodeType = NodeType::ScalarLiteral;
}

/// A RangeLiteral is range of an array or tuple node in the AST.
///
/// Examples:
/// ```
/// 1..3
/// 1..n
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct RangeLiteral {
    pub start: NodeId<Expression>,
    pub end: NodeId<Expression>,
    pub is_inclusive: bool,
}

impl Node for RangeLiteral {
    const KIND: NodeType = NodeType::RangeLiteral;
}

/// A TupleLiteral is literal tuple of heterogeneous elements node in the AST.
///
/// Examples:
/// ```
/// (1, 2, 3)
/// (1.0, 2.0, 3.0)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct TupleLiteral {
    pub elements: Vec<NodeId<Expression>>,
}

impl Node for TupleLiteral {
    const KIND: NodeType = NodeType::TupleLiteral;
}

/// An ArrayLiteral is literal array of homogeneous elements node in the AST.
///
/// Examples:
/// ```
/// [] // empty array
/// [1, 2, ] // trailing comma is allowed
/// // multi-line array with implicit comma
/// [
///   1 // comma is optional here
///   2 // comma is optional here too
/// ]
/// [10, false, "Hi"] // hetereogenous array is invalid but legal in AST
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum ArrayLiteral {
    Fixed { elements: Vec<NodeId<Expression>> },
}

impl Node for ArrayLiteral {
    const KIND: NodeType = NodeType::ArrayLiteral;
}

/// A StructLiteral is literal struct of heterogeneous fields node in the AST.
///
/// Examples:
/// ```
/// Vector2 { x: 1, y: 2 }
/// some_module.MyUnion.OptionB { a: true }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct StructLiteral {
    /// The type of the struct.
    pub r#type: NodeId<Type>,
    /// The fields of the struct.
    pub fields: Vec<NodeId<FieldLiteral>>,
}

impl Node for StructLiteral {
    const KIND: NodeType = NodeType::StructLiteral;
}

/// A FieldLiteral is a literal field value node in the AST.
///
/// Examples:
/// ```
/// x: 1,
/// y: 2,
/// z
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum FieldLiteral {
    /// The name of the field to bind.
    Named {
        name: StringId,
        value: NodeId<Expression>,
    },
    /// The name of the field to bind. Take the value from context.
    NamedShorthand { name: StringId },
}

impl Node for FieldLiteral {
    const KIND: NodeType = NodeType::FieldLiteral;
}
