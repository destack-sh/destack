use crate::{Expression, Node, NodeId, NodeType, StringId, Visibility};

/// The style of a module.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ModuleFormat {
    /// Implicit module in a source (e.g., whole file).
    Implicit,
    /// Forward declaration for a module (e.g., `module x;`)
    Forward,
    /// Inline module with explicit braces (e.g., `module x { ... }`).
    Inline,
}

/// A Module is a module declaration.
/// Modules may be whole directories, single files, or nested within a file.
/// NOTE #Incomplete: Module-level static parameterisation? (just use `let` somehow?)
///
/// Examples:
/// ```
/// module foo {
///     ...
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    /// The module format.
    pub format: ModuleFormat,
    /// The name of the module.
    pub name: Option<StringId>,
    /// The visibility of the module.
    pub visibility: Option<Visibility>,
    /// The body of the module.
    pub expressions: Vec<NodeId<Expression>>,
}

impl Node for Module {
    const KIND: NodeType = NodeType::Module;
}
