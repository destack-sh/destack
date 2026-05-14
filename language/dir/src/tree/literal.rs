use serde::{Deserialize, Serialize};

use crate::{Argument, FloatType, IntegerType, IntrinsicType, LocalNodeId, StringId};

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
    /// Floating-point type.
    Float(FloatType),
    /// Symbol type.
    Symbol,
    /// Unique symbol type.
    UniqueSymbol,
    /// Intrinsic type.
    Intrinsic(IntrinsicType),
}
