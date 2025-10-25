use dyst_source::StringId;

use crate::{Node, NodeType, Path};

/// How an Export should be treated for processing by the system.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExportType {
    // Export as regular item (export foo)
    Item,
    // Export as default item (export default foo)
    Default,
}

/// A DependencyTarget is the target to import from.
#[derive(Debug, Clone, PartialEq)]
pub enum DependencyTarget {
    // Regular Path target as an identifier/path (like `foo` or `foo.bar`)
    Path(Path),
    // Virtual string target as a literal string (like `"foo"` or `"foo/bar"`)
    Virtual(StringId),
}

/// The type of a dependency item.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DependencyType {
    /// Type dependency (`import type foo` or `export type foo`).
    Type,
    /// Value dependency (`import foo` or `export foo`).
    Value,
}

/// A DependencyItem is an item to use in a import clause.
#[derive(Debug, Clone, PartialEq)]
pub enum DependencyItem {
    /// Import all items from a target (`import * from foo` or `export * from foo`).
    Glob {
        ty: DependencyType,
        target: DependencyTarget,
        alias: Option<StringId>,
    },
    /// Import a single item from a target (or current scope when target is None) (`import foo` or `export foo`).
    Scalar {
        ty: DependencyType,
        target: Option<DependencyTarget>,
        name: StringId,
        alias: Option<StringId>,
    },
}

impl Node for DependencyItem {
    const KIND: NodeType = NodeType::DependencyItem;
}
