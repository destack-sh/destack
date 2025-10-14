use dyst_source::StringId;

use crate::{Node, NodeId, NodeType, Path};

/// How an Export should be treated for processing by the system.
#[derive(Debug, Clone, PartialEq)]
pub enum ExportMode {
    // Export as regular item (export foo)
    Item,
    // Export as default item (export default foo)
    Default,
}

/// A ImportClause is a single clause in a import dependency declaration.
///
/// Examples:
/// ```
/// foo
/// foo as bar
/// foo.bar as baz
/// foo.{baz, qux}
/// { baz, qux } from foo // equivalent
/// * from foo // equivalent
/// * as foo from foo // equivalent
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ImportClause {
    /// The target to import from (like `foo.bar` in `import foo.bar.{baz, qux}`)
    pub target: Path,
    /// The alias to use for the definition (like `bar` in `import foo as bar`)
    pub alias: Option<StringId>,
    /// The items to import from the target (like `{baz, qux}` in `import foo.bar.{baz, qux}`)
    pub items: Option<Vec<NodeId<ImportItem>>>,
}

impl Node for ImportClause {
    const KIND: NodeType = NodeType::ImportClause;
}

/// A ImportItem is an item to import from a target in a import clause.
///
/// Examples:
/// ```
/// baz
/// qux as quux
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ImportItem {
    /// The source of the item (like `foo` in `foo as bar`)
    pub name: StringId,
    /// The alias to use for the item (like `bar` in `foo as bar`)
    pub alias: Option<StringId>,
}

impl Node for ImportItem {
    const KIND: NodeType = NodeType::ImportItem;
}
