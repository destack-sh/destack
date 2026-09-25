use serde::{Deserialize, Serialize};
use tspp_core::StringId;
use tspp_mir as mir;
use tspp_serde::Reflect;

/// One object-local global declaration or definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct Global {
    /// The module-local MIR global identity.
    pub id: mir::GlobalId,
    /// The source-facing global name.
    pub name: StringId,
    /// The persistent linkable global identity.
    pub symbol: mir::Symbol,
    /// The stored type.
    pub ty: mir::TypeId,
    /// Whether instructions may mutate this global.
    pub mutability: mir::Mutability,
    /// The space containing this global.
    pub space: mir::Space,
    /// The global linkage.
    pub linkage: mir::Linkage,
    /// The initializer when this object defines the global.
    pub initializer: Option<mir::GlobalInitializer>,
}

impl Global {
    /// Return whether this global permits stores.
    pub fn is_mutable(&self) -> bool {
        self.mutability == mir::Mutability::Mutable
    }
}
