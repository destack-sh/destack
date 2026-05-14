use destack_core::StringId;
use serde::{Deserialize, Serialize};

use crate::{
    DependencyTarget, Expression, LocalNodeId, Name, Node, NodeType, StaticKey, SymbolForm,
    SymbolSpace,
};

/// How one dependency item binds into the local module.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DependencyBinding {
    /// Regular item (`import { foo } from "foo"` or `export { foo } from "foo"`).
    Item,
    /// Default item (`export default foo`).
    Default,
    /// Namespace (`export * from "foo"`).
    Namespace,
}

/// The export kind of a declaration or binding.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ExportKind {
    /// Named export (`export const foo = 1`).
    Named,
    /// Default export (`export default foo`).
    Default,
}

/// The symbol space one dependency item imports or exports.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DependencySpace {
    /// Type dependency (`import type { Foo }` or `export type { Foo }`).
    Type,
    /// Value dependency (`import foo` or `export foo`).
    Value,
}

impl DependencySpace {
    /// Return the symbol space introduced by this dependency space.
    pub fn symbol_space(self) -> SymbolSpace {
        match self {
            Self::Type => SymbolSpace::Type,
            Self::Value => SymbolSpace::Value,
        }
    }

    /// Return the symbol form introduced by this dependency space.
    pub fn symbol_form(self) -> SymbolForm {
        match self {
            Self::Type => SymbolForm::TypeAlias,
            Self::Value => SymbolForm::Variable,
        }
    }
}

/// A dependency item imports or exports one binding from a target.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DependencyItem {
    /// One valid dependency item.
    ///
    /// Examples:
    /// ```
    /// baz
    /// qux as quux
    /// default
    /// default as bar
    /// ```
    Item {
        /// How the item binds into the local module.
        binding: DependencyBinding,
        /// The symbol space of the item, when specified.
        space: Option<DependencySpace>,
        /// The name of the item (like `foo` in `foo as bar`, None if default).
        name: Option<Name>,
        /// The alias to use for the item (like `bar` in `foo as bar`).
        alias: Option<StringId>,
        /// The value of the item (for namespace exports).
        value: Option<LocalNodeId<Expression>>,
    },
    /// One malformed dependency item slot.
    Error,
}

impl Node for DependencyItem {
    const TYPE: NodeType = NodeType::DependencyItem;
}

impl DependencyItem {
    /// Return the symbol key introduced by this dependency item.
    pub fn symbol_key(&self) -> Option<StaticKey> {
        let Self::Item { name, alias, .. } = self else {
            return None;
        };

        if let Some(alias) = alias {
            return Some(StaticKey::Name(*alias));
        }

        match name {
            None => None,
            Some(Name::Identifier(name) | Name::String(name)) => Some(StaticKey::Name(*name)),
            Some(Name::Number(name)) => Some(StaticKey::Number(*name)),
        }
    }

    /// Return the symbol space introduced by this dependency item.
    pub fn symbol_space(&self, default_space: DependencySpace) -> Option<SymbolSpace> {
        let Self::Item { space, .. } = self else {
            return None;
        };

        Some(space.unwrap_or(default_space).symbol_space())
    }

    /// Return the symbol form introduced by this dependency item.
    pub fn symbol_form(&self, default_space: DependencySpace) -> Option<SymbolForm> {
        let Self::Item { space, .. } = self else {
            return None;
        };

        Some(space.unwrap_or(default_space).symbol_form())
    }
}

/// A namespace export edge from `export * from` declarations.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NamespaceExport {
    /// The target module.
    pub target: DependencyTarget,
    /// The dependency space for the export.
    pub space: DependencySpace,
    /// The dependency item node that declared the export.
    pub item: LocalNodeId<DependencyItem>,
}
