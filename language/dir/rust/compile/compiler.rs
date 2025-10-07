use dyst_session::Session;

use crate::{NodeTree, Workspace};

/// The options for compiling a Workspace.
#[derive(Debug, Clone, Default)]
pub struct CompilerOptions {}

/// A compiler for a Workspace of Dyst sources on Dyst DIR.
#[derive(Debug, Clone)]
pub struct Compiler<'s> {
    /// The node tree of the compiled DIR.
    pub tree: NodeTree,
    /// The options for compiling the Workspace.
    pub options: CompilerOptions,

    /// The workspace to compile.
    pub workspace: &'s Workspace,
	/// The session to compile with.
	pub session: &'s Session,
}

impl<'s> Compiler<'s> {
    /// Create a new compiler.
    pub fn new(workspace: &'s Workspace, options: CompilerOptions) -> Self {
        Self {
			tree: NodeTree::new(),
            options,
            workspace,
			session: &workspace.session,
        }
    }
}
