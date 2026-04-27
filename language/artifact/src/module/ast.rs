use destack_ast::{self as ast};
use destack_core::StringPool;
use destack_source::{FileId, ModuleId, Span};
use serde::{Deserialize, Serialize};

/// AST payload for one parsed module.
/// NOTE #Performance: revisit actually required AST state (tokens add ~10-20% memory overhead)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ast {
    /// The id of the Module.
    pub id: ModuleId,

    /// The AST of the Module (may be empty).
    pub tree: ast::Tree,
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
    /// Stable anchor expression for diagnostics.
    pub anchor_expression: Option<ast::LocalNodeId<ast::Expression>>,
}

impl Ast {
    /// Create a new empty AST payload.
    pub fn new(id: ModuleId) -> Self {
        // build empty module ast
        Self {
            id,
            tree: ast::Tree::new(),
            parents: ast::NodeParentIndex::new(),
            roots: Vec::new(),
            strings: StringPool::new(),
            tokens: Vec::new(),
            side_tokens: Vec::new(),
            anchor_expression: None,
        }
    }

    /// Create an AST payload from a tree.
    pub fn from_tree(
        id: ModuleId,
        tree: ast::Tree,
        roots: Vec<ast::LocalNodeId<ast::Expression>>,
        strings: StringPool,
        tokens: Vec<ast::TokenSpan>,
        side_tokens: Vec<ast::TokenSpan>,
    ) -> Self {
        // derive parent index
        let parents = ast::NodeParentIndex::from_tree(&tree);

        // build module ast from parts
        Self {
            id,
            tree,
            parents,
            roots,
            strings,
            tokens,
            side_tokens,
            anchor_expression: None,
        }
    }

    /// Ensure a stable anchor expression exists for diagnostics.
    pub fn ensure_anchor_expression(
        &mut self,
        file_id: FileId,
    ) -> ast::LocalNodeId<ast::Expression> {
        // reuse existing anchor when already present
        if let Some(anchor_id) = self.anchor_expression {
            return anchor_id;
        }

        // insert a synthetic literal anchored at the file start
        let span = Span::empty(file_id);
        let anchor_id = self.tree.insert(
            ast::Expression::ScalarLiteral(ast::ScalarLiteral::Boolean(false)),
            span,
        );
        self.parents.append_root();
        self.anchor_expression = Some(anchor_id);

        anchor_id
    }
}
