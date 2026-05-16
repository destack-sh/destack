use destack_core::{StringId, StringPool};
use serde::{Deserialize, Serialize};

use crate::{
    ExportKey, ExportSelector, Expression, LocalNodeId, Name, Node, NodeType, StaticKey, SymbolForm,
};

/// How one dependency item binds into the local module.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DependencyBinding {
    /// Named binding (`import { foo } from "foo"` or `export { foo } from "foo"`).
    Named,
    /// Default binding (`import foo from "foo"` or `export default foo`).
    Default,
    /// Namespace binding (`import * as foo from "foo"` or `export * from "foo"`).
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
    /// Plain dependency (`import { foo } from "foo"` or `export { foo }`).
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

    /// Return the local binding key selected by this export item.
    pub fn export_source_key(&self) -> Option<StaticKey> {
        let Self::Binding { name, .. } = self else {
            return None;
        };

        name.map(|name| name.static_key())
    }

    /// Return the target selector introduced by this export item.
    pub fn export_selector(&self) -> Option<ExportSelector> {
        let Self::Binding { binding, name, .. } = self else {
            return None;
        };

        let selector = match binding {
            DependencyBinding::Default => ExportSelector::Default,
            DependencyBinding::Namespace => ExportSelector::Namespace,
            DependencyBinding::Named => ExportSelector::Named((*name)?.static_key()),
        };

        Some(selector)
    }

    /// Return the export key introduced by this dependency item.
    pub fn export_key(&self, strings: &StringPool) -> Option<ExportKey> {
        let Self::Binding {
            binding,
            name,
            alias,
            ..
        } = self
        else {
            return None;
        };

        if let Some(alias) = alias {
            return Some(ExportKey::from_string(*alias, strings));
        }

        match binding {
            DependencyBinding::Default => Some(ExportKey::Default),
            DependencyBinding::Named | DependencyBinding::Namespace => {
                name.map(|name| ExportKey::from_name(name, strings))
            }
        }
    }

    /// Check whether this item declares a star export.
    pub fn is_star_export(&self) -> bool {
        matches!(
            self,
            Self::Binding {
                binding: DependencyBinding::Namespace,
                alias: None,
                ..
            }
        )
    }

    /// Check whether this item declares a default export expression.
    pub fn is_default_value_export(&self) -> bool {
        matches!(
            self,
            Self::Binding {
                binding: DependencyBinding::Default,
                value: Some(_),
                ..
            }
        )
    }
}
