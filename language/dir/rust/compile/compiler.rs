use dyst_session::Session;

use crate::{NodeTree, Workspace};

/// A compiler for a Workspace of Dyst sources on Dyst DIR.
///
/// Basically:
/// ```ignore
/// fn compile(trees: ast::NodeTree[]) -> (tree: dir::NodeTree)
/// ```
#[derive(Debug, Clone)]
pub struct Compiler<'s> {
    /// The workspace to compile.
    pub workspace: &'s Workspace,
	/// The node tree of the compiled DIR.
	pub tree: NodeTree,
}

impl<'s> Compiler<'s> {
    /// Create a new compiler.
    pub fn new(workspace: &'s Workspace) -> Self {
        Self {
            workspace,
            tree: NodeTree::new(),
        }
    }
}
