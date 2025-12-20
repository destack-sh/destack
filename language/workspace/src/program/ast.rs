use destack_ast::{self as ast};
use destack_base::StringPool;
use destack_source::{ModuleId, ModuleVersion};

/// AST-level module data.
/// NOTE #Performance: revisit actually required ModuleAst state (tokens add ~10-20% memory overhead)
#[derive(Debug)]
pub struct ModuleAst {
    /// The id of the Module.
    pub id: ModuleId,
    /// The version of the Module.
    pub version: ModuleVersion,

    /// The AST of the Module (may be empty).
    pub tree: ast::NodeTree,
    /// The AST parent index.
    pub parents: ast::NodeParentIndex,
    /// The top-level AST expressions of the Module.
    pub roots: Vec<ast::LocalNodeId<ast::Expression>>,
    /// The string pool of the Module.
    pub strings: StringPool,
    /// The tokens of the Module.
    pub tokens: Vec<ast::TokenSpan>,
    /// The side tokens (comments, whitespace) of the Module.
    pub side_tokens: Vec<ast::TokenSpan>,
}

impl ModuleAst {
    /// Create a new empty ModuleAst.
    pub fn new(id: ModuleId, version: ModuleVersion) -> Self {
        Self {
            id,
            version,
            tree: ast::NodeTree::new(),
            parents: ast::NodeParentIndex::new(),
            roots: Vec::new(),
            strings: StringPool::new(),
            tokens: Vec::new(),
            side_tokens: Vec::new(),
        }
    }

    /// Create a ModuleAst from a tree.
    pub fn from_tree(
        id: ModuleId,
        version: ModuleVersion,
        tree: ast::NodeTree,
        roots: Vec<ast::LocalNodeId<ast::Expression>>,
        strings: StringPool,
        tokens: Vec<ast::TokenSpan>,
        side_tokens: Vec<ast::TokenSpan>,
    ) -> Self {
        let parents = ast::NodeParentIndex::from_tree(&tree);
        Self {
            id,
            version,
            tree,
            parents,
            roots,
            strings,
            tokens,
            side_tokens,
        }
    }
}
