use destack_core::StringId;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{Space, Tree, TypeId};

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

impl Tree {
    /// Intern one compile-time value.
    pub fn intern_static(&mut self, value: Static) -> StaticId {
        let value = match value {
            Static::Float(bits) if f64::from_bits(bits).is_nan() => {
                Static::Float(f64::NAN.to_bits())
            }
            value => value,
        };
        let hash = Self::intern_hash(&value);
        if let Some(ids) = self.static_index.get(&hash) {
            for id in ids {
                if self.static_value(*id) == &value {
                    return *id;
                }
            }
        }

        // retain one canonical value for subsequent identities
        let id = StaticId(self.statics.allocate(value));
        self.static_index.entry(hash).or_default().push(id);

        id
    }

    /// Return one interned compile-time value.
    pub fn static_value(&self, id: StaticId) -> &Static {
        self.statics.get(id.0)
    }
}

impl Static {
    /// Return the closed length this static names, absent for a value parameter.
    pub fn length(&self) -> Option<u64> {
        match self {
            Static::Integer(length) => u64::try_from(*length).ok(),
            _ => None,
        }
    }
}
