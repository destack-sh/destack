use dyst_source::StringId;

use crate::{Node, NodeType};

/// How an Export should be treated for processing by the system.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExportType {
    /// Export as regular item (export foo)
    Item,
    /// Export as default item (export default foo)
    Default,
    /// Export as entire module (export * from foo)
    Module,
}

/// The type of a dependency item.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DependencyKind {
    /// Type dependency (`import type foo` or `export type foo`).
    Type,
    /// Value dependency (`import foo` or `export foo`).
    Value,
}

/// The source of the import.
#[derive(Debug, Clone, PartialEq)]
pub enum DependencySource {
    /// Plain import statement (like `import "foo"`).
    Import,
    /// Import call (like `await import("foo")`).
    ImportCall,
    /// Require call (like `require("foo")`).
    RequireCall,
}

impl DependencySource {
    /// Whether the source is dynamic (like `await import("foo")` or `require("foo")`).
    pub fn is_dynamic(&self) -> bool {
        matches!(
            self,
            DependencySource::ImportCall | DependencySource::RequireCall
        )
    }
}

/// A DependencyItem is an item to use in a import clause.
#[derive(Debug, Clone, PartialEq)]
pub enum DependencyItem {
    /// Import a target as a side effect without alias (like `import "foo"`)
    SideEffect {
        kind: DependencyKind,
        target: StringId,
    },
    /// Import or export all items from a target (`import * from "foo"` or `export * from "foo"`).
    Namespace {
        kind: DependencyKind,
        target: StringId,
        alias: StringId,
    },
    /// Import or export a single item from a target (`import "foo"` or `export "foo"`).
    Named {
        kind: DependencyKind,
        target: Option<StringId>,
        name: StringId,
        alias: Option<StringId>,
    },
}

impl Node for DependencyItem {
    const TYPE: NodeType = NodeType::DependencyItem;
}
