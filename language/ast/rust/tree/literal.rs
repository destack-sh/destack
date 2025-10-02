use crate::{Expression, Node, NodeId, NodeType, StringId};

/// A ScalarLiteral is literal scalar value node.
///
/// Examples:
/// ```
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
    /// Boolean value.
    Boolean(bool),
    /// Byte value.
    Byte(u8),
    /// Integer value.
    Integer(i64),
    /// Float value.
    Float(f64),
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

/// A TypeLiteral is literal type node.
/// Some types are also their literal scalar values (like `null`).
///
/// Examples:
/// ```
/// !
/// $
/// _
/// undefined
/// void
/// null
/// int2
/// float64
/// boolean
/// Self
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum TypeLiteral {
    /// Never type `!`.
    Never,
    /// Any type `$`.
    Any,
    /// Infer type `_`.
    Infer,
    /// Unknown / uninitialized type and value.
    Undefined,
    /// Void / empty / unit type.
    Void,
    /// Null type and value.
    Null,
    /// Boolean type.
    Boolean,
    /// Character type.
    Character,
    /// String type (unsized).
    String,
    /// "Number" type (alias).
    Number,
    /// Integer type.
    Int(IntType),
    /// Float type.
    Float(FloatType),
    /// Composite type.
    Composite(CompositeType),
    /// Self type.
    Self_,
}

impl Node for TypeLiteral {
    const KIND: NodeType = NodeType::TypeLiteral;
}

/// An IntType represents arbitrary width integer with signedness.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct IntType {
    /// Bit width. May be omitted in AST for better diagnostics.
    pub width: Option<u16>,
    /// Whether the integer is signed (`int*` or `uint*`).
    pub is_signed: bool,
}

/// A FloatType represents IEEE-754 float.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct FloatType {
    /// Float width. May be omitted in AST for better diagnostics.
    pub width: Option<u16>,
}

impl IntType {
    #[inline]
    pub fn as_str(self) -> String {
        if self.is_signed {
            // int
            if let Some(width) = self.width {
                format!("int{width}")
            } else {
                "int".to_string()
            }
        } else {
            // uint
            if let Some(width) = self.width {
                format!("uint{width}")
            } else {
                "uint".to_string()
            }
        }
    }
}

impl FloatType {
    #[inline]
    pub fn as_str(self) -> String {
        if let Some(width) = self.width {
            format!("float{width}")
        } else {
            "float".to_string()
        }
    }
}

/// A CompositeType represents composite types.
#[derive(Debug, Clone, PartialEq)]
pub enum CompositeType {
    /// Base type `type`.
    Type,
    /// Struct type `struct MyStruct { ... }`.
    Struct,
    /// Enum type `enum MyEnum { ... }`.
    Enum,
    /// Union type `A | B | C`.
    Union,
    /// Tuple type `(T1, T2, ...)`.
    Tuple,
    /// Trait type `trait MyTrait { ... }`.
    Trait,
    /// Function type `function (T1, T2, ...) => T`.
    Function,
}

/// A RangeLiteral is range of an array or tuple node.
///
/// Examples:
/// ```
/// 1..3
/// 1..n // exclusive
/// 1..=n // inclusive
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

/// A TupleLiteral is an anonymous tuple of heterogeneous elements.
/// For named tuple "literals", see the Call node.
///
/// Examples:
/// ```
/// (1, 2, 3)
/// (1.0, 2.0, 3.0)
/// (x: int32, y: boolean)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct TupleLiteral {
    pub elements: Vec<NodeId<TupleLiteralField>>,
}

impl Node for TupleLiteral {
    const KIND: NodeType = NodeType::TupleLiteral;
}

/// A TupleLiteralField is a tuple field definition.
/// Tuple elements may be named or anonymous, but cannot have default values.
#[derive(Debug, Clone, PartialEq)]
pub enum TupleLiteralField {
    Named {
        name: StringId,
        value: NodeId<Expression>,
    },
    Positional {
        value: NodeId<Expression>,
    },
}

impl Node for TupleLiteralField {
    const KIND: NodeType = NodeType::TupleLiteralField;
}

/// An ArrayLiteral is literal array of homogeneous elements node.
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

/// A StructLiteral is literal struct of heterogeneous fields node.
/// Struct literals always have an explicit type prefix (unlike tuple literals).
///
/// Examples:
/// ```
/// Vector2 { x: 1, y: 2 }
/// some_module.MyUnion.OptionB { a: true }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct StructLiteral {
    /// The type of the struct.
    pub r#type: NodeId<Expression>,
    /// The fields of the struct.
    pub fields: Vec<NodeId<FieldLiteral>>,
}

impl Node for StructLiteral {
    const KIND: NodeType = NodeType::StructLiteral;
}

/// A FieldLiteral is a literal field value node.
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
