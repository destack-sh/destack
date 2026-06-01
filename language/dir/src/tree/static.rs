use serde::{Deserialize, Serialize};

use destack_source::ModuleId;

use crate::{
    Declaration, FunctionSignature, GenericParameterRef, GlobalSymbolId, GlobalTypeId, LocalNodeId,
    ScalarLiteral, StaticKey, StringId, TypeLiteral,
};

/// Static value produced by checked static evaluation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StaticTerm {
    /// Generic parameter reference.
    Parameter(GenericParameterRef),
    /// Static symbol reference.
    Symbol { symbol: GlobalSymbolId },
    /// Normalized access value.
    Access { access: Access },
    /// Normalized storage space value.
    Space { space: Space },
    /// Normalized place value.
    Place { place: Place },
    /// Normalized lifetime value.
    Lifetime { lifetime: Lifetime },
    /// Scalar literal.
    ScalarLiteral { value: ScalarLiteral },
    /// Type literal.
    TypeLiteral { value: TypeLiteral },
    /// Declaration reference with optional static arguments.
    Declaration {
        /// The declaration node.
        declaration: LocalNodeId<Declaration>,
        /// Static generic arguments.
        generic_arguments: Option<Vec<StaticArgument>>,
    },
    /// Type value.
    Type { ty: GlobalTypeId },
    /// Array value.
    Array { elements: Vec<StaticTerm> },
    /// Fixed array value.
    FixedArray {
        /// The repeated value.
        value: Box<StaticTerm>,
        /// The fixed array length.
        length: Box<StaticTerm>,
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
    /// Union of static values.
    Union { elements: Vec<GlobalStaticId> },
}

/// Normalized memory access value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Access {
    /// Shared readonly access.
    Readonly,
    /// Mutable access.
    Mutable,
    /// Exclusive access.
    Exclusive,
}

/// Normalized storage space value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Space {
    /// Local storage.
    Local,
    /// Shared storage.
    Shared,
    /// Static storage.
    Static,
    /// Frame storage.
    Frame,
}

/// Normalized place value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Place {
    /// Ambient placement.
    Ambient,
    /// Concrete storage space.
    Space(Space),
}

/// Normalized lifetime value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Lifetime {
    /// Static lifetime.
    Static,
    /// Symbolic lifetime parameter or associated constant.
    Symbol(GlobalSymbolId),
}

/// Static argument in a checked static context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StaticArgument {
    /// The optional argument name.
    pub name: Option<StringId>,
    /// The static value id.
    pub value: GlobalStaticId,
}

impl StaticArgument {
    /// Build a positional static argument.
    pub fn value(value: GlobalStaticId) -> Self {
        Self { name: None, value }
    }
}

/// Static object property in a checked static context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

/// Unique identifier for a local static value.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
