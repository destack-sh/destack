use destack_serde::Reflect;
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

use destack_core::StringPool;

use crate::{
    Argument, Expression, FloatType, IntegerType, LanguageItem, LocalNodeId, Name, Node, NodeType,
    PrimitiveType, RangeType, ScalarAlias, ScalarDomain, StringId, Type,
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
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Reflect)]
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
    /// Return this literal's boolean value.
    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            Self::Boolean(value) => Some(*value),
            _ => None,
        }
    }

    /// Return this literal's integral value.
    pub fn as_integral(&self) -> Option<i64> {
        match self {
            Self::Integer(value) | Self::Bigint(value) => Some(*value),
            _ => None,
        }
    }

    /// Return whether this literal is negative floating-point zero.
    pub fn is_negative_zero(&self) -> bool {
        matches!(self, Self::Float(value) if value.to_bits() == (-0.0_f64).to_bits())
    }

    /// Return whether this literal is a floating-point NaN.
    pub fn is_nan(&self) -> bool {
        matches!(self, Self::Float(value) if value.is_nan())
    }

    /// Return this literal's variant name.
    pub fn variant_name(&self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Undefined => "undefined",
            Self::Boolean(_) => "boolean",
            Self::Integer(_) => "integer",
            Self::Bigint(_) => "bigint",
            Self::Float(_) => "float",
            Self::Character(_) => "character",
            Self::String(_) => "string",
            Self::RegexString { .. } => "regex",
        }
    }

    /// Return this literal's scalar domain.
    pub fn scalar_domain(&self) -> Option<ScalarDomain> {
        let domain = match self {
            Self::Integer(_) => ScalarDomain::Integer,
            Self::Float(_) => ScalarDomain::Float,
            Self::Bigint(_) => ScalarDomain::Bigint,
            Self::Character(_) => ScalarDomain::Character,
            Self::String(_) => ScalarDomain::String,
            Self::Boolean(_) => ScalarDomain::Boolean,
            Self::Null => ScalarDomain::Null,
            Self::Undefined => ScalarDomain::Undefined,
            Self::RegexString { .. } => return None,
        };

        Some(domain)
    }

    /// Return this literal's scalar domain when it can bound an interval.
    pub fn interval_domain(&self) -> Option<ScalarDomain> {
        let domain = match self {
            Self::Integer(_) => ScalarDomain::Integer,
            Self::Bigint(_) => ScalarDomain::Bigint,
            Self::Character(_) => ScalarDomain::Character,
            _ => return None,
        };

        Some(domain)
    }

    /// Return the next literal in this literal's discrete interval domain.
    pub fn successor(&self) -> Option<Self> {
        let literal = match self {
            Self::Integer(value) => Self::Integer(value.checked_add(1)?),
            Self::Bigint(value) => Self::Bigint(value.checked_add(1)?),
            Self::Character(value) => {
                let mut scalar = (*value as u32).checked_add(1)?;

                // skip invalid Unicode scalar values
                while scalar <= char::MAX as u32 {
                    if let Some(value) = char::from_u32(scalar) {
                        return Some(Self::Character(value));
                    }

                    scalar = scalar.checked_add(1)?;
                }

                return None;
            }
            _ => return None,
        };

        Some(literal)
    }

    /// Widen one scalar literal to its base type.
    pub fn widen(&self) -> Type {
        match self {
            // numeric literals without a numeric context widen to plain number
            Self::Integer(_) => Type::Primitive(PrimitiveType::Float(FloatType::Float64)),
            Self::Float(_) => Type::Primitive(PrimitiveType::Float(FloatType::Float64)),
            Self::Bigint(_) => Type::Primitive(PrimitiveType::Bigint),
            Self::String(_) => Type::Primitive(PrimitiveType::String),
            Self::Boolean(_) => Type::Primitive(PrimitiveType::Boolean),
            Self::Character(_) => Type::Primitive(PrimitiveType::Character),
            Self::Null => Type::Null,
            Self::Undefined => Type::Undefined,
            Self::RegexString { .. } => Type::Error,
        }
    }

    /// Return whether this literal can widen to one target type.
    pub fn widens_to(&self, target: &Type) -> bool {
        match target {
            Type::Literal(target) => self == target,
            Type::Primitive(primitive) => self.widens_to_primitive(*primitive),
            Type::Range(range) => self.widens_to_range(range),
            _ => false,
        }
    }

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

    /// Return whether this literal's family stores every member identically.
    ///
    /// Numeric families span carriers of different widths, so a numeric
    /// literal names no single storage representation; string-like families
    /// have exactly one carrier and widen as stored.
    pub fn has_uniform_carrier(&self) -> bool {
        match self {
            Self::String(_)
            | Self::Character(_)
            | Self::Boolean(_)
            | Self::Bigint(_)
            | Self::RegexString { .. }
            | Self::Null
            | Self::Undefined => true,
            Self::Integer(_) | Self::Float(_) => false,
        }
    }

    /// Return whether this literal can widen to one primitive type.
    pub fn widens_to_primitive(&self, primitive: PrimitiveType) -> bool {
        match (self, primitive) {
            (Self::String(_), PrimitiveType::String) => true,
            (Self::Character(_), PrimitiveType::Character) => true,
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

    /// Return whether this literal can widen to one interval type.
    pub fn widens_to_range(&self, range: &RangeType) -> bool {
        range.contains_literal(*self)
    }

    /// Return whether this literal inhabits every member of one scalar domain.
    pub fn widens_to_domain(&self, domain: ScalarDomain) -> bool {
        const FLOATS: [FloatType; 4] = [
            FloatType::Float16,
            FloatType::Bfloat16,
            FloatType::Float32,
            FloatType::Float64,
        ];

        match (self, domain) {
            (Self::Integer(value), ScalarDomain::Integer) => {
                let signed = IntegerType::Fixed {
                    width: 8,
                    is_signed: true,
                };
                let unsigned = IntegerType::Fixed {
                    width: 8,
                    is_signed: false,
                };

                signed.fits_literal(*value) && unsigned.fits_literal(*value)
            }
            (Self::Integer(value), ScalarDomain::Float) => FLOATS
                .iter()
                .all(|float| float.fits_integer_literal(*value)),
            (Self::Float(value), ScalarDomain::Float) => {
                FLOATS.iter().all(|float| float.fits_literal(*value))
            }
            (Self::String(_), ScalarDomain::String) => true,
            (Self::Character(_), ScalarDomain::Character) => true,
            (Self::Boolean(_), ScalarDomain::Boolean) => true,
            (Self::Bigint(_), ScalarDomain::Bigint) => true,
            _ => false,
        }
    }

    /// Return the language item carrying this literal's runtime representation.
    pub fn representation_item(&self) -> Option<LanguageItem> {
        self.scalar_domain()?.representation_item()
    }
}

/// A tree tag attribute.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum TreeAttribute {
    /// Named attribute with an optional value.
    Named {
        name: Name,
        value: Option<TreeAttributeValue>,
    },
    /// Spread attribute.
    Spread { value: LocalNodeId<Expression> },
    /// Malformed attribute slot.
    Error,
}

impl Node for TreeAttribute {
    const TYPE: NodeType = NodeType::TreeAttribute;
}

/// The value form of a tree tag attribute.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum TreeAttributeValue {
    /// Quoted string attribute value.
    String(StringId),
    /// Expression container attribute value.
    Expression(LocalNodeId<Expression>),
}

impl TreeAttribute {
    /// Return the value expression carried by this attribute when present.
    pub fn value(&self) -> Option<LocalNodeId<Expression>> {
        match self {
            Self::Named { value, .. } => value.as_ref().and_then(TreeAttributeValue::expression),
            Self::Spread { value } => Some(*value),
            Self::Error => None,
        }
    }
}

impl TreeAttributeValue {
    /// Return the expression carried by this value when present.
    pub const fn expression(&self) -> Option<LocalNodeId<Expression>> {
        match self {
            Self::String(_) => None,
            Self::Expression(value) => Some(*value),
        }
    }
}

/// A tree child.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum TreeChild {
    /// Raw tree text.
    Text { value: StringId },
    /// Expression container child.
    Expression { value: LocalNodeId<Expression> },
    /// Spread expression container child.
    Spread { value: LocalNodeId<Expression> },
    /// Nested tree expression child.
    Tree { value: LocalNodeId<Expression> },
    /// Empty expression container child.
    Empty,
    /// Malformed child slot.
    Error,
}

impl Node for TreeChild {
    const TYPE: NodeType = NodeType::TreeChild;
}

impl TreeChild {
    /// Return the value expression carried by this child when present.
    pub const fn value(&self) -> Option<LocalNodeId<Expression>> {
        match self {
            Self::Text { .. } | Self::Empty | Self::Error => None,
            Self::Expression { value } | Self::Spread { value } | Self::Tree { value } => {
                Some(*value)
            }
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
            (Self::Float(left), Self::Float(right)) => left.to_bits() == right.to_bits(),
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
                value.to_bits().hash(state);
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum TypeLiteral {
    /// Never type `never`.
    Never,
    /// Any type `any`.
    Any,
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
    /// Bigint type (unsized).
    Bigint,
    /// `number`, the `float64` source alias.
    Number,
    /// A widthless source alias for a sized scalar, like `int` for `int64`.
    Alias(ScalarAlias),
    /// Width-spelled integer type, like `int32` or `usize`.
    Integer(IntegerType),
    /// Width-spelled floating-point type, like `float32`.
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

impl TypeLiteral {
    /// Return this literal type's scalar domain.
    pub fn scalar_domain(&self) -> Option<ScalarDomain> {
        let domain = match self {
            Self::Undefined => ScalarDomain::Undefined,
            Self::Null => ScalarDomain::Null,
            Self::Boolean => ScalarDomain::Boolean,
            Self::Character => ScalarDomain::Character,
            Self::String => ScalarDomain::String,
            Self::Bigint => ScalarDomain::Bigint,
            Self::Number | Self::Float(_) => ScalarDomain::Float,
            Self::Alias(alias) => alias.primitive().scalar_domain(),
            Self::Integer(_) => ScalarDomain::Integer,
            Self::Symbol | Self::UniqueSymbol => ScalarDomain::Symbol,
            Self::Never | Self::Any | Self::Unknown | Self::Void => return None,
        };

        Some(domain)
    }

    /// Return the language item owning this type's runtime representation.
    pub fn representation_item(&self) -> Option<LanguageItem> {
        self.scalar_domain()?.representation_item()
    }
}
