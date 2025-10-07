use dyst_session::Session;

/// A compiler for a workspace of Dyst sources into Dyst DIR.
///
/// Basically:
/// ```ignore
/// fn compile(trees: ast::NodeTree[]) -> (tree: dir::NodeTree)
/// ```

#[derive(Debug, Clone)]
pub struct Compiler<'s> {
	/// The shared session for the workspace.
	pub session: &'s Session,

    // nocheckin
}

impl<'s> Compiler<'s> {
	/// Create a new compiler.
	pub fn new(session: &'s Session) -> Self {
		Self { session }
	}
}