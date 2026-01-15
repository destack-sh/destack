use destack_ast::{self as ast};
use destack_base::StringPool;
use destack_source::{ModuleId, ModuleVersion};
use serde::{Deserialize, Serialize};

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

/// Serializable snapshot of ModuleAst data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleAstData {
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
        // build empty module ast
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
        // derive parent index
        let parents = ast::NodeParentIndex::from_tree(&tree);

        // build module ast from parts
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

    /// Create a serializable snapshot of this module ast.
    pub fn to_data(&self) -> ModuleAstData {
        // snapshot module ast state
        ModuleAstData {
            id: self.id,
            version: self.version,
            tree: self.tree.clone(),
            parents: self.parents.clone(),
            roots: self.roots.clone(),
            strings: self.strings.clone(),
            tokens: self.tokens.clone(),
            side_tokens: self.side_tokens.clone(),
        }
    }

    /// Rebuild a ModuleAst from serialized data.
    pub fn from_data(data: ModuleAstData) -> Self {
        // rebuild module ast from snapshot
        Self {
            id: data.id,
            version: data.version,
            tree: data.tree,
            parents: data.parents,
            roots: data.roots,
            strings: data.strings,
            tokens: data.tokens,
            side_tokens: data.side_tokens,
        }
    }
}

impl Serialize for ModuleAst {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_data().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ModuleAst {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let data = ModuleAstData::deserialize(deserializer)?;
        Ok(ModuleAst::from_data(data))
    }
}
