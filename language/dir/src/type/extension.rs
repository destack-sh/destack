use std::fmt::Display;

use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{GlobalSymbolId, LocalTypeId};

/// Unique identifier for extensions.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LocalExtensionId(pub u32);

impl LocalExtensionId {
    /// Wrap an id as a local extension id.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Turn into a global extension id.
    pub fn into_global(self, module_id: ModuleId) -> GlobalExtensionId {
        GlobalExtensionId {
            module_id,
            local_id: self,
        }
    }
}

/// Global extension id across modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct GlobalExtensionId {
    /// The module id of the global extension.
    pub module_id: ModuleId,
    /// The local id of the global extension.
    pub local_id: LocalExtensionId,
}

impl GlobalExtensionId {
    /// Create a new global extension id.
    pub fn new(module_id: ModuleId, local_id: LocalExtensionId) -> Self {
        Self {
            module_id,
            local_id,
        }
    }

    /// Turn into a local extension id.
    pub fn into_local(self) -> LocalExtensionId {
        self.local_id
    }
}

impl From<GlobalExtensionId> for LocalExtensionId {
    fn from(id: GlobalExtensionId) -> Self {
        id.local_id
    }
}

impl Display for LocalExtensionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "~{}", self.0)
    }
}

/// How an extension declaration relates to its target type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExtensionForm {
    /// Inherent extension defined in same module as target type.
    /// Automatically visible wherever the type is used.
    Inherent,
    /// Local extension on a foreign type.
    /// Only visible in the defining module.
    Local,
    /// Named extension on a foreign type.
    /// Must be explicitly imported to use (outside of the defining module).
    Named,
}

/// A resolved extension declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Extension {
    /// The extension declaration's symbol.
    pub symbol: GlobalSymbolId,
    /// The extension declaration form.
    pub form: ExtensionForm,
    /// The nominal type symbol used to index extension lookup.
    pub target_symbol: GlobalSymbolId,
    /// The solved target type being extended.
    pub target_type: LocalTypeId,
}

impl Extension {
    /// Create a new extension.
    pub fn new(
        symbol: GlobalSymbolId,
        form: ExtensionForm,
        target_symbol: GlobalSymbolId,
        target_type: LocalTypeId,
    ) -> Self {
        Self {
            symbol,
            form,
            target_symbol,
            target_type,
        }
    }

    /// Check if this extension is inherent.
    pub fn is_inherent(&self) -> bool {
        matches!(self.form, ExtensionForm::Inherent)
    }

    /// Check if this extension is named (can be exported/imported).
    pub fn is_named(&self) -> bool {
        matches!(self.form, ExtensionForm::Named)
    }

    /// Check if this extension is local.
    pub fn is_local(&self) -> bool {
        matches!(self.form, ExtensionForm::Local)
    }
}
