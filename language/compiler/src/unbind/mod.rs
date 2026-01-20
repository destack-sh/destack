mod annotation;
mod argument;
mod block;
mod declaration;
mod dependency;
mod expression;
mod function;
mod key;
mod literal;
mod r#match;
pub mod module;
mod operator;
mod path;
mod pattern;
mod property;
mod r#type;
mod r#where;

pub use module::*;

use destack_workspace::ProfileId;
use std::collections::HashMap;
use {destack_ast as ast, destack_dir as dir};

/// Shared state for unbinding a module.
pub(super) struct UnbindContext {
    /// Map DIR node ids to their corresponding AST node ids.
    pub node_map: HashMap<dir::LocalNodeIdAny, ast::LocalNodeIdAny>,
    /// The profile id for this unbind run.
    pub profile: ProfileId,
    /// Fallback node for diagnostics when no source node exists.
    pub fallback_node: dir::LocalNodeIdAny,
}

impl UnbindContext {
    /// Create a new unbind context.
    pub(super) fn new(profile: ProfileId, fallback_node: dir::LocalNodeIdAny) -> Self {
        // build the context defaults
        Self {
            node_map: HashMap::new(),
            profile,
            fallback_node,
        }
    }

    /// Record a DIR to AST node mapping.
    pub(super) fn map(&mut self, dir_id: dir::LocalNodeIdAny, ast_id: ast::LocalNodeIdAny) {
        // record the mapping
        self.node_map.insert(dir_id, ast_id);
    }
}
