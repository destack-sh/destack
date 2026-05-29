use std::fmt::Display;

use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{
    GlobalSymbolId, LocalStaticId, LocalTypeId, StaticArgument, StringId, VarianceModifier,
};

/// Unique identifier for generic slots.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LocalGenericSlotId(pub u32);

impl LocalGenericSlotId {
    /// Wrap an id as a local generic slot id.
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

/// Unique identifier for generic instances.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LocalInstanceId(pub u32);

impl LocalInstanceId {
    /// Wrap an id as a local instance id.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Turn into a global instance id.
    pub fn into_global(self, module_id: ModuleId) -> GlobalInstanceId {
        GlobalInstanceId {
            module_id,
            local_id: self,
        }
    }
}

/// Global instance id across modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct GlobalInstanceId {
    /// The module id of the global instance.
    pub module_id: ModuleId,
    /// The local id of the global instance.
    pub local_id: LocalInstanceId,
}

impl GlobalInstanceId {
    /// Create a new global instance id.
    pub fn new(module_id: ModuleId, local_id: LocalInstanceId) -> Self {
        Self {
            module_id,
            local_id,
        }
    }

    /// Turn into a local instance id.
    pub fn into_local(self) -> LocalInstanceId {
        self.local_id
    }
}

impl From<GlobalInstanceId> for LocalInstanceId {
    fn from(id: GlobalInstanceId) -> Self {
        id.local_id
    }
}

impl Display for LocalInstanceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

/// Source that introduced one generic slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GenericSlotOrigin {
    /// The slot was written in source.
    Explicit,
    /// The slot was induced by check.
    Induced,
}

/// User-visible key of one generic slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GenericSlotKey {
    /// Explicit source symbol.
    Symbol(GlobalSymbolId),
    /// Generated checked slot key.
    Generated(StringId),
}

/// Declaration order index for one generic slot.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GenericSlotIndex(pub u32);

impl GenericSlotIndex {
    /// Wrap an index as a generic slot index.
    pub fn new(index: u32) -> Self {
        Self(index)
    }

    /// Return the following generic slot index.
    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }

    /// Return the raw index.
    pub fn get(self) -> u32 {
        self.0
    }
}

/// One declaration-side generic slot.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum GenericSlot {
    /// Type generic slot.
    Type {
        /// The generic owner symbol.
        owner: GlobalSymbolId,
        /// The slot key.
        key: GenericSlotKey,
        /// The declaration order index.
        index: GenericSlotIndex,
        /// The slot variance.
        variance: Option<VarianceModifier>,
        /// The optional type constraint.
        constraint: Option<LocalTypeId>,
        /// The optional type default.
        default: Option<LocalTypeId>,
        /// The slot origin.
        origin: GenericSlotOrigin,
    },
    /// Variadic type generic slot.
    VariadicType {
        /// The generic owner symbol.
        owner: GlobalSymbolId,
        /// The slot key.
        key: GenericSlotKey,
        /// The declaration order index.
        index: GenericSlotIndex,
        /// The slot variance.
        variance: Option<VarianceModifier>,
        /// The optional type constraint.
        constraint: Option<LocalTypeId>,
        /// The optional type default.
        default: Option<LocalTypeId>,
        /// The slot origin.
        origin: GenericSlotOrigin,
    },
    /// Static generic slot.
    Static {
        /// The generic owner symbol.
        owner: GlobalSymbolId,
        /// The slot key.
        key: GenericSlotKey,
        /// The declaration order index.
        index: GenericSlotIndex,
        /// The optional static value type constraint.
        constraint: Option<LocalTypeId>,
        /// The optional static default.
        default: Option<LocalStaticId>,
        /// The slot origin.
        origin: GenericSlotOrigin,
    },
    /// Variadic static generic slot.
    VariadicStatic {
        /// The generic owner symbol.
        owner: GlobalSymbolId,
        /// The slot key.
        key: GenericSlotKey,
        /// The declaration order index.
        index: GenericSlotIndex,
        /// The optional static value type constraint.
        constraint: Option<LocalTypeId>,
        /// The optional static default.
        default: Option<LocalStaticId>,
        /// The slot origin.
        origin: GenericSlotOrigin,
    },
}

impl GenericSlot {
    /// Return the owner symbol.
    pub fn owner(&self) -> GlobalSymbolId {
        match self {
            Self::Type { owner, .. }
            | Self::VariadicType { owner, .. }
            | Self::Static { owner, .. }
            | Self::VariadicStatic { owner, .. } => *owner,
        }
    }

    /// Return the slot key.
    pub fn key(&self) -> GenericSlotKey {
        match self {
            Self::Type { key, .. }
            | Self::VariadicType { key, .. }
            | Self::Static { key, .. }
            | Self::VariadicStatic { key, .. } => *key,
        }
    }

    /// Return the declaration order index.
    pub fn index(&self) -> GenericSlotIndex {
        match self {
            Self::Type { index, .. }
            | Self::VariadicType { index, .. }
            | Self::Static { index, .. }
            | Self::VariadicStatic { index, .. } => *index,
        }
    }

    /// Return the slot origin.
    pub fn origin(&self) -> GenericSlotOrigin {
        match self {
            Self::Type { origin, .. }
            | Self::VariadicType { origin, .. }
            | Self::Static { origin, .. }
            | Self::VariadicStatic { origin, .. } => *origin,
        }
    }
}

/// A concrete application of static arguments to one generic symbol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenericInstance {
    /// The symbol being instantiated.
    pub symbol: GlobalSymbolId,
    /// The static arguments in declaration order.
    pub arguments: Vec<StaticArgument>,
}

impl GenericInstance {
    /// Create a generic instance.
    pub fn new(symbol: GlobalSymbolId, arguments: Vec<StaticArgument>) -> Self {
        Self { symbol, arguments }
    }
}
