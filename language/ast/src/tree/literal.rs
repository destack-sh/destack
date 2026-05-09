use serde::{Deserialize, Serialize};

use crate::{Argument, LocalNodeId, StringId};

/// A ScalarLiteral is literal scalar value.
///
/// Examples:
/// ```
/// true
/// false
/// 1
/// 1n
/// 0x21
/// 1.0
/// "Hello, world!"
/// 'a'
/// /abc/
/// /abc/g
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ScalarLiteral {
    /// Null value.
    Null,
    /// Boolean value.
    Boolean(bool),
    /// Integer value.
    Integer(i64),
    /// Bigint value.
    Bigint(i64),
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
}

/// A TemplateLiteral is literal template value.
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TemplateLiteral {
    /// Template string value.
    String { string: StringId },
    /// Interpolated template literal value.
    InterpolatedString {
        strings: Vec<StringId>,
        arguments: Vec<LocalNodeId<Argument>>,
    },
}

// NOTE #Architecture: maybe TypeLiterals shouldn't even exist at AST level?

/// A TypeLiteral is literal type.
/// Some types are also their literal scalar values (like `null`).
///
/// Examples:
/// ```
/// never
/// any
/// undefined
/// void
/// null
/// int2
/// float64
/// boolean
/// symbol
/// unique symbol
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TypeLiteral {
    /// Never type `never`.
    Never,
    /// Any type `any`.
    Any,
    /// Infer type `_`.
    Infer,
    /// Uninitialized type and value.
    Undefined,
    /// Unknown type.
    Unknown,
    /// Object type (any non-primitive).
    Object,
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
    /// Bigint type (unsized).
    Bigint,
    /// `number`, the `float64` source alias.
    Number,
    /// Integer type.
    Integer(IntegerType),
    /// Float type.
    Float(FloatType),
    /// Symbol type.
    Symbol,
    /// Unique symbol type.
    UniqueSymbol,
    /// Intrinsic type (TypeScript compiler-provided).
    Intrinsic(IntrinsicType),
}

/// A TypeIntrinsic is a compiler-provided intrinsic type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntrinsicType {
    /// Uppercase string intrinsic.
    Uppercase,
    /// Lowercase string intrinsic.
    Lowercase,
    /// Capitalize string intrinsic.
    Capitalize,
    /// Uncapitalize string intrinsic.
    Uncapitalize,
    /// NoInfer intrinsic.
    NoInfer,
    /// Builtin iterator return intrinsic.
    BuiltinIteratorReturn,
}

impl TryFrom<&str> for IntrinsicType {
    /// The error type for intrinsic parsing.
    type Error = ();

    /// Parse a TypeIntrinsic from a standard intrinsic name.
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "Uppercase" => Ok(IntrinsicType::Uppercase),
            "Lowercase" => Ok(IntrinsicType::Lowercase),
            "Capitalize" => Ok(IntrinsicType::Capitalize),
            "Uncapitalize" => Ok(IntrinsicType::Uncapitalize),
            "NoInfer" => Ok(IntrinsicType::NoInfer),
            "BuiltinIteratorReturn" => Ok(IntrinsicType::BuiltinIteratorReturn),
            _ => Err(()),
        }
    }
}

/// An integer type name.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum IntegerType {
    /// The signed or unsigned integer family.
    Integer { is_signed: bool },
    /// A fixed-width signed or unsigned integer.
    Fixed { width: u16, is_signed: bool },
    /// A pointer-sized signed or unsigned integer.
    Pointer { is_signed: bool },
}

impl IntegerType {
    /// Whether the integer is signed.
    pub fn is_signed(&self) -> bool {
        match self {
            IntegerType::Integer { is_signed }
            | IntegerType::Fixed {
                width: _,
                is_signed,
            }
            | IntegerType::Pointer { is_signed } => *is_signed,
        }
    }
}

/// A floating-point type name.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FloatType {
    /// The floating-point family.
    Float,
    /// A 32-bit IEEE-754 float.
    Float32,
    /// A 64-bit IEEE-754 float.
    Float64,
}

impl IntegerType {
    #[inline]
    pub fn as_str(self) -> String {
        match self {
            IntegerType::Pointer { is_signed } => {
                if is_signed {
                    "isize".to_string()
                } else {
                    "usize".to_string()
                }
            }
            IntegerType::Integer { is_signed } => {
                if is_signed {
                    "int".to_string()
                } else {
                    "uint".to_string()
                }
            }
            IntegerType::Fixed { width, is_signed } => {
                if is_signed {
                    format!("int{width}")
                } else {
                    format!("uint{width}")
                }
            }
        }
    }
}

impl FloatType {
    #[inline]
    pub fn as_str(self) -> &'static str {
        match self {
            FloatType::Float => "float",
            FloatType::Float32 => "float32",
            FloatType::Float64 => "float64",
        }
    }
}
