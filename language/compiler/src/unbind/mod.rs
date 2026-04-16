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
pub(crate) mod module;
mod operator;
mod path;
mod pattern;
mod property;
mod r#type;
mod r#where;

pub use module::*;

use std::collections::HashMap;
use {destack_ast as ast, destack_dir as dir};

/// Shared state for unbinding a module.
pub(super) struct UnbindContext {
    /// Map DIR node ids to their corresponding AST node ids.
    pub node_map: HashMap<dir::LocalNodeIdAny, ast::LocalNodeIdAny>,
}

impl UnbindContext {
    /// Create a new unbind context.
    pub(super) fn new() -> Self {
        // build the context defaults
        Self {
            node_map: HashMap::new(),
        }
    }

    /// Record a DIR to AST node mapping.
    pub(super) fn map(&mut self, dir_id: dir::LocalNodeIdAny, ast_id: ast::LocalNodeIdAny) {
        // record the mapping
        self.node_map.insert(dir_id, ast_id);
    }
}
