use std::sync::Arc;

use destack_dir::LocalNodeIdAny;
use destack_source::{FileId, Span};
use parking_lot::RwLock;

use crate::program::{ModuleAst, ModuleDir};
use crate::{Module, Session};

/// Get a module by FileId.
pub fn get_module_by_file_id(session: &Session, file_id: FileId) -> Option<Arc<RwLock<Module>>> {
    session.modules.get_by_file_id(file_id)
}

/// Get the span of a DIR node by mapping through AST source map.
pub fn get_dir_node_span(
    ast: &ModuleAst,
    dir: &ModuleDir,
    dir_node_id: LocalNodeIdAny,
) -> Option<Span> {
    let dir_tree = dir.tree.read();

    // get the AST node id from the DIR node
    let ast_node_id = dir_tree.get_source(dir_node_id.id);
    drop(dir_tree);

    // get the span from AST source map
    Some(ast.tree.source_map.get(ast_node_id))
}

/// Get the main span of a DIR node (e.g., identifier for declarations).
/// Falls back to full span if no main span is set.
pub fn get_dir_node_main_span(
    ast: &ModuleAst,
    dir: &ModuleDir,
    dir_node_id: LocalNodeIdAny,
) -> Option<Span> {
    let dir_tree = dir.tree.read();

    // get the AST node id from the DIR node
    let ast_node_id = dir_tree.get_source(dir_node_id.id);
    drop(dir_tree);

    // try to get the main span first (e.g., identifier span for declarations)
    Some(ast.tree.source_map.get_main_or_enclosing(ast_node_id))
}
