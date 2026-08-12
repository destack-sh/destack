use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use destack_source::ModuleId;

use crate::{FunctionSignature, GlobalTypeId, ScalarLiteral, StaticKey, StringId, TypeFold};

/// A concrete static value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, TypeFold)]
pub enum StaticTerm {
    /// Scalar literal.
    ScalarLiteral { value: ScalarLiteral },
    /// Type value.
    Type { ty: GlobalTypeId },
    /// Array value.
    Array { elements: Vec<StaticTerm> },
    /// Fixed array value.
    FixedArray {
        /// The repeated value.
        value: Box<StaticTerm>,
        /// The fixed array length.
        length: u64,
    },
    /// Tuple value.
    Tuple { elements: Vec<StaticTerm> },
    /// Nominal newtype value.
    Newtype {
        /// The instantiated newtype.
        ty: GlobalTypeId,
        /// The wrapped value.
        value: Box<StaticTerm>,
    },
    /// Structural object value.
    Object {
        /// The object properties.
        properties: Vec<StaticProperty>,
    },
    /// Nominal struct value.
    Struct {
        /// The struct type selected for this value.
        ty: GlobalTypeId,
        /// The struct properties.
        properties: Vec<StaticProperty>,
    },
}

impl From<ScalarLiteral> for StaticTerm {
    fn from(value: ScalarLiteral) -> Self {
        Self::ScalarLiteral { value }
    }
}

impl StaticTerm {
    /// Return this value's scalar literal.
    pub fn as_scalar(&self) -> Option<ScalarLiteral> {
        match self {
            Self::ScalarLiteral { value } => Some(*value),
            _ => None,
        }
    }

    /// Return this value's string literal.
    pub fn as_string(&self) -> Option<StringId> {
        match self.as_scalar()? {
            ScalarLiteral::String(value) => Some(value),
            _ => None,
        }
    }

    /// Return this value's boolean literal.
    pub fn as_boolean(&self) -> Option<bool> {
        self.as_scalar()?.as_boolean()
    }

    /// Return this value's integer literal.
    pub fn as_integer(&self) -> Option<i64> {
        match self.as_scalar()? {
            ScalarLiteral::Integer(value) => Some(value),
            _ => None,
        }
    }

    /// Return this nominal newtype's wrapped value.
    pub fn as_newtype(&self) -> Option<(GlobalTypeId, &StaticTerm)> {
        match self {
            Self::Newtype { ty, value } => Some((*ty, value)),
            _ => None,
        }
    }

    /// Return this value's tuple elements.
    pub fn as_tuple(&self) -> Option<&[StaticTerm]> {
        match self {
            Self::Tuple { elements } => Some(elements),
            _ => None,
        }
    }

    /// Return this value's object properties.
    pub fn as_object(&self) -> Option<&[StaticProperty]> {
        match self {
            Self::Object { properties } => Some(properties),
            _ => None,
        }
    }

    /// Return this value's array elements.
    pub fn as_array(&self) -> Option<&[StaticTerm]> {
        match self {
            Self::Array { elements } => Some(elements),
            _ => None,
        }
    }
}

/// Static object property in a checked static context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, TypeFold)]
pub enum StaticProperty {
    /// Static field.
    Field {
        /// The property key.
        key: StaticKey,
        /// The property value.
        value: StaticTerm,
    },
    /// Static member function.
    Method {
        /// The optional method key.
        key: Option<StaticKey>,
        /// The method signature.
        signature: FunctionSignature,
        /// The method body.
        body: StaticTerm,
    },
    /// Static spread.
    Spread {
        /// The spread value.
        value: StaticTerm,
    },
}

impl StaticProperty {
    /// Return this property's field key and value.
    pub fn as_field(&self) -> Option<(StaticKey, &StaticTerm)> {
        match self {
            Self::Field { key, value } => Some((*key, value)),
            Self::Method { .. } | Self::Spread { .. } => None,
        }
    }
}

/// Unique identifier for a local static value.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub struct LocalStaticId(pub u32);

impl LocalStaticId {
    /// Wrap a raw id as a LocalStaticId.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Convert this id into a global static id.
    pub fn into_global(self, module_id: ModuleId) -> GlobalStaticId {
        GlobalStaticId {
            module_id,
            local_id: self,
        }
    }
}

/// Global static id across modules.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub struct GlobalStaticId {
    /// The module id of the global static value.
    pub module_id: ModuleId,
    /// The local id of the global static value.
    pub local_id: LocalStaticId,
}

impl GlobalStaticId {
    /// Create a new global static id.
    pub fn new(module_id: ModuleId, local_id: LocalStaticId) -> Self {
        Self {
            module_id,
            local_id,
        }
    }

    /// Convert this id into a local static id.
    pub fn into_local(self) -> LocalStaticId {
        self.local_id
    }
}

impl From<GlobalStaticId> for LocalStaticId {
    fn from(id: GlobalStaticId) -> Self {
        id.local_id
    }
}
