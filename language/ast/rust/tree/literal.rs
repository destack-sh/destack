use crate::{Argument, NodeId, Path, StringId};

/// A ScalarLiteral is literal scalar value node.
/// NOTE: for #Leniency we parse multi-character `'aa'` as regular string literals.
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
/// /abc/
/// /abc/g
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
    /// Regex string value.
    RegexString {
        content: StringId,
        flags: Option<StringId>,
    },
    /// Byte string value.
    ByteString(Vec<u8>),
}

/// A TemplateLiteral is literal template value node.
/// For interpolated templates, the start and end string may be empty.
///  (If there is an immediate argument after the first ` or before the last `, respectively).
///
/// Examples:
/// ```
/// `hello`
/// `hello ${name}`
/// sql`SELECT * FROM users`
/// sql`${stmt}`
/// sql.expr`SELECT * FROM users WHERE name = ${name}` AND age > ${group.age()} LIMIT 10`
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum TemplateLiteral {
    /// Template string value.
    String { string: StringId },
    /// Tagged template literal value.
    TaggedString { tag: Path, string: StringId },
    /// Interpolated template literal value.
    InterpolatedString {
        strings: Vec<StringId>,
        arguments: Vec<NodeId<Argument>>,
    },
    /// Tagged interpolated template literal value.
    TaggedInterpolatedString {
        tag: Path,
        strings: Vec<StringId>,
        arguments: Vec<NodeId<Argument>>,
    },
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
    /// Any type `$` or `any`.
    Any,
    /// Infer type `_`.
    Infer,
    /// Uninitialized type and value.
    Undefined,
    /// Unknown type.
    Unknown,
    /// Void type.
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
    /// Interface type `interface MyInterface { ... }`.
    Interface,
    /// Function type `function (T1, T2, ...) => T`.
    Function,
}
