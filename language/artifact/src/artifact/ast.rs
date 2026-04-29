use destack_ast::{self as ast};
use destack_core::StringPool;
use serde::{Deserialize, Serialize};

/// AST payload for one parsed module.
/// TODO #Performance: revisit actually required AST state (tokens add ~10-20% memory overhead)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ast {
    /// The AST tree.
    pub tree: ast::Tree,
    /// The AST parent index.
    pub parents: ast::NodeParentIndex,
    /// The top-level AST expressions.
    pub roots: Vec<ast::LocalNodeId<ast::Expression>>,
    /// The module string pool.
    pub strings: StringPool,
    /// The module tokens.
    pub tokens: Vec<ast::TokenSpan>,
    /// The module side tokens.
    pub side_tokens: Vec<ast::TokenSpan>,
    /// Stable anchor expression for diagnostics.
    pub anchor_expression: ast::LocalNodeId<ast::Expression>,
}

impl Ast {
    /// Create an AST payload from a tree.
    pub fn from_tree(
        tree: ast::Tree,
        roots: Vec<ast::LocalNodeId<ast::Expression>>,
        strings: StringPool,
        tokens: Vec<ast::TokenSpan>,
        side_tokens: Vec<ast::TokenSpan>,
        anchor_expression: ast::LocalNodeId<ast::Expression>,
    ) -> Self {
        // derive parent index
        let parents = ast::NodeParentIndex::from_tree(&tree);

        // build module ast from parts
        Self {
            tree,
            parents,
            roots,
            strings,
            tokens,
            side_tokens,
            anchor_expression,
        }
    }
}
