use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

use crate::{Argument, FloatType, IntegerType, LocalNodeId, PrimitiveType, StringId};

/// A ScalarLiteral is literal scalar value.
///
/// Examples:
/// ```
/// undefined
/// null
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScalarLiteral {
    /// Null value.
    Null,
    /// Undefined value.
    Undefined,
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

impl PartialEq for ScalarLiteral {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Null, Self::Null) => true,
            (Self::Undefined, Self::Undefined) => true,
            (Self::Boolean(left), Self::Boolean(right)) => left == right,
            (Self::Integer(left), Self::Integer(right)) => left == right,
            (Self::Bigint(left), Self::Bigint(right)) => left == right,
            (Self::Float(left), Self::Float(right)) => {
                float_literal_key(*left) == float_literal_key(*right)
            }
            (Self::Character(left), Self::Character(right)) => left == right,
            (Self::String(left), Self::String(right)) => left == right,
            (
                Self::RegexString {
                    content: left_content,
                    flags: left_flags,
                },
                Self::RegexString {
                    content: right_content,
                    flags: right_flags,
                },
            ) => left_content == right_content && left_flags == right_flags,
            _ => false,
        }
    }
}

impl Eq for ScalarLiteral {}

impl Hash for ScalarLiteral {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Self::Null => 0_u8.hash(state),
            Self::Undefined => 1_u8.hash(state),
            Self::Boolean(value) => {
                2_u8.hash(state);
                value.hash(state);
            }
            Self::Integer(value) => {
                3_u8.hash(state);
                value.hash(state);
            }
            Self::Bigint(value) => {
                4_u8.hash(state);
                value.hash(state);
            }
            Self::Float(value) => {
                5_u8.hash(state);
                float_literal_key(*value).hash(state);
            }
            Self::Character(value) => {
                6_u8.hash(state);
                value.hash(state);
            }
            Self::String(value) => {
                7_u8.hash(state);
                value.hash(state);
            }
            Self::RegexString { content, flags } => {
                8_u8.hash(state);
                content.hash(state);
                flags.hash(state);
            }
        }
    }
}

/// Return a stable equality and hash key for one float literal.
fn float_literal_key(value: f64) -> u64 {
    if value == 0.0 {
        0.0_f64.to_bits()
    } else if value.is_nan() {
        f64::NAN.to_bits()
    } else {
        value.to_bits()
    }
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
    /// Floating-point type.
    Float(FloatType),
    /// Symbol type.
    Symbol,
    /// Unique symbol type.
    UniqueSymbol,
}

impl From<PrimitiveType> for TypeLiteral {
    /// Convert a semantic primitive type into a source type literal.
    fn from(primitive: PrimitiveType) -> Self {
        match primitive {
            PrimitiveType::Boolean => Self::Boolean,
            PrimitiveType::Character => Self::Character,
            PrimitiveType::String => Self::String,
            PrimitiveType::Bigint => Self::Bigint,
            PrimitiveType::Integer(integer) => Self::Integer(integer),
            PrimitiveType::Float(float) => Self::Float(float),
            PrimitiveType::Symbol => Self::Symbol,
            PrimitiveType::UniqueSymbol => Self::UniqueSymbol,
        }
    }
}
