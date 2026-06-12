use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

use destack_core::StringPool;

use crate::{
    Argument, FloatType, IntegerType, LanguageItem, Layout, LocalNodeId, PrimitiveType, RangeType,
    StringId,
};

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
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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

impl ScalarLiteral {
    /// Return the printed text of this literal inside a template.
    pub fn template_text(&self, strings: &StringPool) -> Option<String> {
        match self {
            Self::String(value) => Some(strings.get(*value).to_string()),
            Self::Character(value) => Some(value.to_string()),
            Self::Boolean(value) => Some(value.to_string()),
            Self::Integer(value) => Some(value.to_string()),
            Self::Bigint(value) => Some(value.to_string()),
            // floats print like javascript numbers
            Self::Float(value) => {
                if value.fract() == 0.0 && value.is_finite() {
                    Some(format!("{}", *value as i64))
                } else {
                    Some(value.to_string())
                }
            }
            Self::Null => Some("null".to_string()),
            Self::Undefined => Some("undefined".to_string()),
            Self::RegexString { .. } => None,
        }
    }

    /// Return whether this literal value inhabits one primitive type.
    pub fn fits_primitive(&self, primitive: PrimitiveType) -> bool {
        match (self, primitive) {
            (Self::String(_), PrimitiveType::String) => true,
            (Self::Character(_), PrimitiveType::Character) => true,
            (Self::Character(_), PrimitiveType::String) => true,
            (Self::Boolean(_), PrimitiveType::Boolean) => true,
            (Self::Bigint(_), PrimitiveType::Bigint) => true,
            (Self::Integer(value), PrimitiveType::Integer(integer)) => integer.fits_literal(*value),
            (Self::Integer(value), PrimitiveType::Float(float)) => {
                float.fits_integer_literal(*value)
            }
            (Self::Float(value), PrimitiveType::Float(float)) => float.fits_literal(*value),
            _ => false,
        }
    }

    /// Return whether this literal value inhabits one interval type.
    pub fn fits_range(&self, range: &RangeType) -> bool {
        let below_start = match (&range.start, self) {
            (Some(Self::Integer(start)), Self::Integer(value)) => value < start,
            (Some(Self::Character(start)), Self::Character(value)) => value < start,
            (Some(_), _) => return false,
            (None, _) => false,
        };
        if below_start {
            return false;
        }

        match (&range.end, self) {
            (Some(Self::Integer(end)), Self::Integer(value)) => {
                if range.is_inclusive {
                    value <= end
                } else {
                    value < end
                }
            }
            (Some(Self::Character(end)), Self::Character(value)) => {
                if range.is_inclusive {
                    value <= end
                } else {
                    value < end
                }
            }
            (Some(_), _) => false,
            (None, _) => true,
        }
    }

    /// Return this literal singleton's layout.
    /// Singleton values are statically known and occupy no storage.
    pub fn layout(&self) -> Option<Layout> {
        match self {
            Self::Null
            | Self::Undefined
            | Self::Boolean(_)
            | Self::Integer(_)
            | Self::Float(_)
            | Self::Character(_)
            | Self::String(_)
            | Self::Bigint(_) => Some(Layout::unit()),
            // regex literals are managed runtime objects
            Self::RegexString { .. } => None,
        }
    }

    /// Return the language item owning this literal's members.
    pub fn owner_item(&self) -> Option<LanguageItem> {
        match self {
            Self::String(_) => Some(LanguageItem::String),
            Self::Integer(_) | Self::Float(_) => Some(LanguageItem::Number),
            _ => None,
        }
    }
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
