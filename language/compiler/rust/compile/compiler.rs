use dyst_ast::{self as ast, StringId, StringPool};
use dyst_package::{DocumentBody, Workspace};
use dyst_session::Session;

use dyst_dir::{Dumper, DumperOptions, NodeTree};
use dyst_source::SourceId;

use crate::AstNodeId;

/// The options for compiling a Workspace.
#[derive(Debug, Clone, Default)]
pub struct CompilerOptions {}

/// A compiler for a related set of Dyst sources on Dyst DIR.
#[derive(Debug, Clone)]
pub struct Compiler<'s> {
    // nocheckin TODO: restrict Compiler to just one package (and track deps)?
    /// The node tree of the compiled DIR.
    pub tree: NodeTree,
    /// The string pool.
    pub strings: StringPool,
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
            strings: StringPool::new(),
            options,
            workspace,
            session: &workspace.session,
        }
    }

    /// Create a new Dumper.
    pub fn dumper(&self, options: DumperOptions) -> Dumper<'_> {
        Dumper::new(&self.strings, &self.tree, options)
    }

    /// Get the document body by its source ID.
    pub fn get_document_body(&self, source_id: SourceId) -> &DocumentBody {
        let document = self
            .workspace
            .get_document_by_id(source_id)
            .unwrap_or_else(|| panic!("document not found: {source_id:?}"));
        &document.body
    }

    /// Resolve an AST node.
    pub fn get_ast_node<T>(&self, node: AstNodeId<T>) -> (&ast::NodeTree, &T)
    where
        T: ast::Node,
        ast::NodeTree: ast::NodeTreeStore<T>,
    {
        let body = self.get_document_body(node.source_id);
        if let DocumentBody::Text { ast, .. } = body {
            let node = ast.get(node.id);
            (ast, node)
        } else {
            panic!("node is not in a text document: {node:?}");
        }
    }

    // Intern an AST string for a certain source.
    pub fn intern_string(&mut self, source_id: SourceId, string_id: StringId) -> StringId {
        let body = self.get_document_body(source_id);
        let DocumentBody::Text { strings, .. } = body else {
            panic!("source is not a text document: {source_id:?}");
        };
        // nocheckin TODO #Performance: Compiler.intern_string clone?
        let string = strings.get(string_id).to_string();
        self.strings.intern(string)
    }
}
