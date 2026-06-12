use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{
    DefinitionMember, GlobalNodeIdAny, GlobalSymbolId, GlobalTypeId, LocalGenericTemplateId,
    NominalHeritage,
};

/// How an extension declaration relates to its target type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExtensionForm {
    /// Inherent extension defined in same module as target type.
    /// Automatically visible wherever the type is used.
    ///
    /// Examples:
    /// ```ds
    /// struct Vector { x: float64; y: float64 }
    /// extension of Vector { length(): float64 { ... } }
    /// ```
    Inherent,
    /// Local extension on a foreign type.
    /// Only visible in the defining module.
    ///
    /// Examples:
    /// ```ds
    /// extension of string { shout(): string { ... } }
    /// ```
    Local,
    /// Named extension on a foreign type.
    /// Must be explicitly imported to use (outside of the defining module).
    ///
    /// Examples:
    /// ```ds
    /// export extension Slugify of string { slug(): string { ... } }
    /// ```
    Named,
}

/// A resolved extension declaration.
///
/// Examples:
/// ```ds
/// extension<T> of Array<T> implements Iterable<T> { ... }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Extension {
    /// The extension declaration's symbol.
    pub symbol: GlobalSymbolId,
    /// The extension declaration form.
    pub form: ExtensionForm,
    /// The extension's generic template.
    pub template: Option<LocalGenericTemplateId>,
    /// The checked receiver target.
    pub target: ExtensionTarget,
    /// The implemented interfaces.
    pub implements: Vec<NominalHeritage>,
    /// The checked where clauses that gate this extension.
    pub where_clauses: Vec<ExtensionWhereClause>,
    /// The members in declaration order.
    pub members: Vec<DefinitionMember>,
}

impl Extension {
    /// Create a new extension.
    pub fn new(
        symbol: GlobalSymbolId,
        form: ExtensionForm,
        template: Option<LocalGenericTemplateId>,
        target: ExtensionTarget,
        implements: Vec<NominalHeritage>,
        where_clauses: Vec<ExtensionWhereClause>,
        members: Vec<DefinitionMember>,
    ) -> Self {
        Self {
            symbol,
            form,
            template,
            target,
            implements,
            where_clauses,
            members,
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

    /// Return whether this extension is visible from one module.
    pub fn is_visible_from(&self, module_id: ModuleId) -> bool {
        match self.form {
            ExtensionForm::Inherent | ExtensionForm::Named => true,
            ExtensionForm::Local => self.symbol.module_id == module_id,
        }
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
///
/// Examples:
/// ```ds
/// extension<T> of Array<T> where T: Comparable { sort(): void { ... } }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtensionWhereClause {
    /// The source where clause node.
    pub source: GlobalNodeIdAny,
    /// The constrained type.
    pub left: GlobalTypeId,
    /// The required constraint type.
    pub right: GlobalTypeId,
}
