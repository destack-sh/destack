use destack_core::StringId;
use serde::{Deserialize, Serialize};

use crate::{Expression, LocalNodeId, Name, Node, NodeType, StaticKey, SymbolForm};

/// How one dependency item binds into the local module.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DependencyBinding {
    /// Named binding (`import { foo } from "foo"` or `export { foo } from "foo"`).
    Named,
    /// Default binding (`export default foo`).
    Default,
    /// Namespace binding (`export * from "foo"`).
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

/// The source form of one dependency declaration.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DependencyForm {
    /// Plain dependency (`import foo` or `export foo`).
    Plain,
    /// Type-marked dependency (`import type { Foo }` or `export type { Foo }`).
    Type,
}

/// A dependency item imports or exports one binding from a target.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DependencyItem {
    /// One valid dependency binding.
    ///
    /// Examples:
    /// ```
    /// baz
    /// qux as quux
    /// default
    /// default as bar
    /// ```
    Binding {
        /// How the item binds into the local module.
        binding: DependencyBinding,
        /// The source form of the item, when specified.
        form: Option<DependencyForm>,
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
        let Self::Binding { name, alias, .. } = self else {
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

    /// Return the symbol form introduced by this dependency item.
    pub fn symbol_form(&self) -> Option<SymbolForm> {
        let Self::Binding { .. } = self else {
            return None;
        };

        Some(SymbolForm::Import)
    }
}
