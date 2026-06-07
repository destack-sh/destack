use std::fmt::Display;

use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{GlobalNodeIdAny, GlobalSymbolId, GlobalTypeId};

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
    /// The checked receiver target.
    pub target: ExtensionTarget,
    /// The checked where clauses that gate this extension.
    pub where_clauses: Vec<ExtensionWhereClause>,
}

impl Extension {
    /// Create a new extension.
    pub fn new(
        symbol: GlobalSymbolId,
        form: ExtensionForm,
        target: ExtensionTarget,
        where_clauses: Vec<ExtensionWhereClause>,
    ) -> Self {
        Self {
            symbol,
            form,
            target,
            where_clauses,
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

/// Extension lookup target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExtensionTarget {
    /// Extension whose receiver type has a nominal root.
    ///
    /// Example:
    /// ```ds
    /// extension<T> of ^Array<T> {}
    /// ```
    Nominal {
        /// The nominal root used for member lookup.
        root: GlobalSymbolId,
        /// The checked receiver type.
        ty: GlobalTypeId,
    },
    /// Extension over an open receiver type.
    ///
    /// Example:
    /// ```ds
    /// extension<T> of T where T: Copy {}
    /// ```
    Blanket {
        /// The checked receiver type.
        ty: GlobalTypeId,
    },
}

impl ExtensionTarget {
    /// Return the checked receiver type.
    pub fn r#type(&self) -> GlobalTypeId {
        match self {
            Self::Nominal { ty, .. } | Self::Blanket { ty } => *ty,
        }
    }

    /// Return the nominal lookup root when this target has one.
    pub fn nominal_root(&self) -> Option<GlobalSymbolId> {
        match self {
            Self::Nominal { root, .. } => Some(*root),
            Self::Blanket { .. } => None,
        }
    }

    /// Return whether this is an open blanket target.
    pub fn is_blanket(&self) -> bool {
        matches!(self, Self::Blanket { .. })
    }
}

/// A checked where clause attached to one extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtensionWhereClause {
    /// The source where clause node.
    pub source: GlobalNodeIdAny,
    /// The constrained type.
    pub left: GlobalTypeId,
    /// The required constraint type.
    pub right: GlobalTypeId,
}
