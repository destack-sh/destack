use destack_core::StringId;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{Space, TypeId};

/// Compact identity of one interned compile-time value.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub struct StaticId(pub u32);

/// One compile-time value retained in MIR.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum Static {
    /// The value one template parameter names.
    Parameter(u32),
    /// The null value.
    Null,
    /// The undefined value.
    Undefined,
    /// A boolean value.
    Boolean(bool),
    /// A signed integer value.
    Integer(i64),
    /// A bigint value.
    Bigint(i64),
    /// A 64-bit floating-point value stored as exact bits.
    Float(u64),
    /// A Unicode scalar value.
    Character(char),
    /// An interned string value.
    String(StringId),
    /// An interned regular expression value.
    Regex {
        /// The regular expression source.
        content: StringId,
        /// The regular expression flags when present.
        flags: Option<StringId>,
    },
    /// A type reflected as a compile-time value.
    Type(TypeId),
    /// A storage space bound as a compile-time value.
    Space(Space),
    /// A homogeneous array value.
    Array(Vec<StaticId>),
    /// A compact repeated fixed-array value.
    FixedArray {
        /// The repeated element value.
        value: StaticId,
        /// The array length.
        length: u64,
    },
    /// A heterogeneous tuple value.
    Tuple(Vec<StaticId>),
    /// A nominal newtype value.
    Newtype {
        /// The concrete newtype.
        ty: TypeId,
        /// The wrapped value.
        value: StaticId,
    },
    /// A structural object value.
    Object(Vec<StaticField>),
    /// A nominal struct value.
    Struct {
        /// The concrete struct type.
        ty: TypeId,
        /// The struct fields.
        fields: Vec<StaticField>,
    },
}

/// One field in a compile-time object or struct value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct StaticField {
    /// The field key.
    pub key: StaticKey,
    /// The field value.
    pub value: StaticId,
}

/// One compile-time property key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum StaticKey {
    /// An interned name key.
    Name(StringId),
    /// A positional index key.
    Index(u64),
}

impl Static {
    /// Map nested compile-time values.
    pub fn map_values(&mut self, map: &mut impl FnMut(StaticId) -> StaticId) {
        match self {
            Self::Array(values) | Self::Tuple(values) => {
                for value in values {
                    *value = map(*value);
                }
            }
            Self::FixedArray { value, .. } | Self::Newtype { value, .. } => *value = map(*value),
            Self::Object(fields) | Self::Struct { fields, .. } => {
                for field in fields {
                    field.value = map(field.value);
                }
            }
            _ => {}
        }
    }

    /// Map the types named by this value.
    pub fn map_types(&mut self, map: &mut impl FnMut(TypeId) -> TypeId) {
        match self {
            Self::Type(ty) | Self::Newtype { ty, .. } | Self::Struct { ty, .. } => *ty = map(*ty),
            _ => {}
        }
    }

    /// Return the closed length this static names, absent for a value parameter.
    pub fn length(&self) -> Option<u64> {
        match self {
            Static::Integer(length) => u64::try_from(*length).ok(),
            _ => None,
        }
    }
}
