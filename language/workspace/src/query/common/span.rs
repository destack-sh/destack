use std::sync::Arc;

use destack_dir::LocalNodeIdAny;
use destack_source::{FileId, Span};
use parking_lot::RwLock;

use crate::{Module, Session};

/// Get a module by FileId (searches through modules).
pub fn get_module_by_file_id(session: &Session, file_id: FileId) -> Option<Arc<RwLock<Module>>> {
    // iterate through modules to find one with matching file_id
    for module in session.modules.iter() {
        let module_guard = module.read();
        if module_guard.file_id == file_id {
            drop(module_guard);
            return Some(module);
        }
    }
    None
}

/// Get the span of a DIR node by mapping through AST source map.
pub fn get_dir_node_span(module: &Module, dir_node_id: LocalNodeIdAny) -> Option<Span> {
    let dir_tree = module.dir.tree.read();

    // get the AST node id from the DIR node
    let ast_node_id = dir_tree.get_source(dir_node_id.id);

    drop(dir_tree);

    // get the span from AST source map
    let span = module.ast.tree.source_map.get(ast_node_id);

    // return span with correct file_id
    Some(Span::new(module.file_id, span.start, span.end))
}

/// Get the main span of a DIR node (e.g., identifier for declarations).
/// Falls back to full span if no main span is set.
pub fn get_dir_node_main_span(module: &Module, dir_node_id: LocalNodeIdAny) -> Option<Span> {
    let dir_tree = module.dir.tree.read();

    // get the AST node id from the DIR node
    let ast_node_id = dir_tree.get_source(dir_node_id.id);

    drop(dir_tree);

    // try to get the main span first (e.g., identifier span for declarations)
    if let Some(main_span) = module.ast.tree.source_map.get_main(ast_node_id) {
        return Some(Span::new(module.file_id, main_span.start, main_span.end));
    }

    // fall back to full span
    let span = module.ast.tree.source_map.get(ast_node_id);
    Some(Span::new(module.file_id, span.start, span.end))
}
