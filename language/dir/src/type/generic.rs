use std::fmt::Display;

use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{
    GlobalSymbolId, LocalStaticId, LocalTypeId, StaticArgument, StringId, VarianceModifier,
};

/// Unique identifier for generic templates.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LocalGenericTemplateId(pub u32);

impl LocalGenericTemplateId {
    /// Wrap an id as a local generic template id.
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

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

/// Unique identifier for generic applications.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LocalGenericApplicationId(pub u32);

impl LocalGenericApplicationId {
    /// Wrap an id as a local generic application id.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Turn into a global generic application id.
    pub fn into_global(self, module_id: ModuleId) -> GlobalGenericApplicationId {
        GlobalGenericApplicationId {
            module_id,
            local_id: self,
        }
    }
}

/// Global generic application id across modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct GlobalGenericApplicationId {
    /// The module id of the global generic application.
    pub module_id: ModuleId,
    /// The local generic application id.
    pub local_id: LocalGenericApplicationId,
}

impl GlobalGenericApplicationId {
    /// Create a new global generic application id.
    pub fn new(module_id: ModuleId, local_id: LocalGenericApplicationId) -> Self {
        Self {
            module_id,
            local_id,
        }
    }

    /// Turn into a local generic application id.
    pub fn into_local(self) -> LocalGenericApplicationId {
        self.local_id
    }
}

impl From<GlobalGenericApplicationId> for LocalGenericApplicationId {
    fn from(id: GlobalGenericApplicationId) -> Self {
        id.local_id
    }
}

impl Display for LocalGenericApplicationId {
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

/// One owner-level declaration of generic slots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenericTemplate {
    /// The symbol that owns this generic template.
    pub owner: GlobalSymbolId,
    /// The generic slots in declaration order.
    pub slots: Vec<LocalGenericSlotId>,
}

impl GenericTemplate {
    /// Create an empty generic template for one owner.
    pub fn new(owner: GlobalSymbolId) -> Self {
        Self {
            owner,
            slots: Vec::new(),
        }
    }
}

/// One declaration-side generic slot.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum GenericSlot {
    /// Type generic slot.
    Type {
        /// The generic template that owns this slot.
        template: LocalGenericTemplateId,
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
        /// The generic template that owns this slot.
        template: LocalGenericTemplateId,
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
        /// The generic template that owns this slot.
        template: LocalGenericTemplateId,
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
        /// The generic template that owns this slot.
        template: LocalGenericTemplateId,
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
    /// Return the owner generic template.
    pub fn template(&self) -> LocalGenericTemplateId {
        match self {
            Self::Type { template, .. }
            | Self::VariadicType { template, .. }
            | Self::Static { template, .. }
            | Self::VariadicStatic { template, .. } => *template,
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

/// One concrete application of static arguments to a generic template.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenericApplication {
    /// The generic template being applied.
    pub template: LocalGenericTemplateId,
    /// The static arguments in declaration order.
    pub arguments: Vec<StaticArgument>,
}

impl GenericApplication {
    /// Create a generic application.
    pub fn new(template: LocalGenericTemplateId, arguments: Vec<StaticArgument>) -> Self {
        Self {
            template,
            arguments,
        }
    }
}
