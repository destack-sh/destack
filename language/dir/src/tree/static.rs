use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use destack_source::ModuleId;

use crate::{FunctionSignature, GlobalTypeId, ScalarLiteral, StaticKey};

/// Concrete static value produced by checked static evaluation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
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
    /// Apply one mapping to every type id stored in this static value.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        match self {
            Self::ScalarLiteral { .. } => {}
            Self::Type { ty } => *ty = map(*ty),
            Self::Array { elements } | Self::Tuple { elements } => {
                for element in elements {
                    element.map_type_ids(map);
                }
            }
            Self::FixedArray { value, .. } => value.map_type_ids(map),
            Self::Object { properties } => {
                for property in properties {
                    property.map_type_ids(map);
                }
            }
            Self::Struct { ty, properties } => {
                *ty = map(*ty);
                for property in properties {
                    property.map_type_ids(map);
                }
            }
        }
    }
}

/// Static object property in a checked static context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
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
    /// Apply one mapping to every type id stored in this property.
    pub fn map_type_ids(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        match self {
            Self::Field { value, .. } | Self::Spread { value } => value.map_type_ids(map),
            // method signatures reference source nodes, not checked type ids
            Self::Method { body, .. } => body.map_type_ids(map),
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
