use destack_core::StringId;
use serde::{Deserialize, Serialize};

use crate::{IntegerReference, TypeReference};

/// One identifier inside attribute syntax.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttributeIdentifier {
    /// One concrete identifier.
    Identifier(StringId),
    /// One required identifier that was omitted.
    Missing,
    /// One malformed identifier fragment.
    Error,
}

impl AttributeIdentifier {
    /// Create one concrete identifier.
    pub fn identifier(identifier: StringId) -> Self {
        Self::Identifier(identifier)
    }
}

/// A metadata attribute attached to a MIR node.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Attribute {
    /// The attribute name.
    pub name: AttributeIdentifier,
    /// The attribute arguments.
    pub args: AttributeArgs,
}

/// Arguments for a MIR attribute.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttributeArgs {
    /// No arguments were provided.
    None,
    /// A single unnamed value was provided.
    Value(AttributeValue),
    /// Multiple unnamed values were provided.
    Values(Vec<AttributeValue>),
    /// Named arguments were provided as key-value pairs.
    KeyValues(Vec<AttributeKeyValue>),
}

/// A key-value pair within an attribute argument list.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AttributeKeyValue {
    /// The argument name.
    pub key: AttributeIdentifier,
    /// The argument value.
    pub value: AttributeValue,
}

/// A floating point literal stored by bit pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FloatValue {
    /// The IEEE-754 bits for the value.
    pub bits: u64,
}

impl FloatValue {
    /// Create a floating point literal from an `f64`.
    pub fn from_f64(value: f64) -> Self {
        Self {
            bits: value.to_bits(),
        }
    }

    /// Return the literal as an `f64`.
    pub fn to_f64(self) -> f64 {
        f64::from_bits(self.bits)
    }
}

/// A value inside an attribute argument list.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttributeValue {
    /// An identifier value.
    Identifier(AttributeIdentifier),
    /// A type value.
    Type(TypeReference),
    /// An integer literal.
    Integer(IntegerReference),
    /// A floating point literal.
    Float(FloatValue),
    /// A boolean literal.
    Boolean(bool),
    /// A string literal.
    String(StringId),
    /// A list of values.
    List(Vec<AttributeValue>),
    /// One required value that was omitted.
    Missing,
    /// One malformed value fragment.
    Error,
}
