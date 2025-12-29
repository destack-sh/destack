use std::panic;

use destack_source::{FileId, Span};
use {destack_ast as ast, destack_dir as dir};

use super::document_symbol::SymbolKind;
use crate::Session;

/// A symbol in the workspace (flat list for workspace symbol search).
#[derive(Debug, Clone)]
pub struct WorkspaceSymbol {
    /// The symbol's name.
    pub name: String,
    /// The kind of symbol.
    pub kind: SymbolKind,
    /// The file containing the symbol.
    pub file: FileId,
    /// The location of the symbol.
    pub range: Span,
    /// Container name (e.g., class name for methods).
    pub container: Option<String>,
}

/// Search for symbols across the workspace.
///
/// Returns symbols whose names contain the query string (case-insensitive).
pub fn workspace_symbols(
    session: &Session,
    query: &str,
    max_results: usize,
) -> Vec<WorkspaceSymbol> {
    let mut symbols = Vec::new();
    let query_lower = query.to_lowercase();

    // search all modules
    'outer: for module in session.modules.iter() {
        let module = module.read();
        let Some(ctx) = session.query_context(&module) else {
            continue;
        };

        let dir_tree = ctx.tree();

        // iterate through all declarations
        for (declaration_id, declaration) in dir_tree.iter_nodes_of_type::<dir::Declaration>() {
            // get the declaration name
            let descriptor = declaration.descriptor();
            let Some(string_id) = descriptor.name else {
                continue;
            };

            let name = ctx.ast.strings.get(string_id).to_string();

            // check if name matches query (case-insensitive substring match)
            if !query.is_empty() && !name.to_lowercase().contains(&query_lower) {
                continue;
            }

            // get the kind based on declaration type
            let kind = match declaration {
                dir::Declaration::Global { .. } => SymbolKind::Namespace,
                dir::Declaration::Function { .. } => SymbolKind::Function,
                dir::Declaration::Struct { .. } => SymbolKind::Struct,
                dir::Declaration::Class { .. } => SymbolKind::Class,
                dir::Declaration::Interface { .. } => SymbolKind::Interface,
                dir::Declaration::Enum { .. } => SymbolKind::Enum,
                dir::Declaration::Namespace { .. } => SymbolKind::Namespace,
                dir::Declaration::Type { .. } => SymbolKind::TypeParameter,
                dir::Declaration::Extension { .. } => SymbolKind::Class,
            };

            // find container by walking up parent tree
            let container = find_container_name(&dir_tree, &ctx.ast.strings, declaration_id.id);

            // get span (safely, skipping if out of bounds)
            let ast_node_id = dir_tree.get_source(declaration_id.id);
            let full_span = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                ctx.ast.tree.source_map.get(ast_node_id)
            }));

            let Ok(full_span) = full_span else {
                continue;
            };

            symbols.push(WorkspaceSymbol {
                name,
                kind,
                file: ctx.file_id,
                range: full_span,
                container,
            });

            if symbols.len() >= max_results {
                // NOTE #Performance: revisit workspace_symbols limits
                break 'outer;
            }
        }
    }

    symbols
}

/// Find the container name for a node by walking up the parent tree.
/// NOTE #Performance: find workspace symbol containers more efficiently?
fn find_container_name(
    dir_tree: &dir::NodeTree,
    strings: &ast::StringPool,
    node_id: u32,
) -> Option<String> {
    let mut current_id = node_id;

    // walk up parent chain
    while let Some(parent) = dir_tree.get_parent(current_id) {
        if parent.ty == dir::NodeType::Declaration {
            // found a parent declaration, get its name
            let Ok(decl_id) = dir::LocalNodeId::<dir::Declaration>::try_from(parent) else {
                current_id = parent.id;
                continue;
            };
            let parent_decl = dir_tree.get(decl_id);
            let descriptor = parent_decl.descriptor();
            if let Some(name_id) = descriptor.name {
                return Some(strings.get(name_id).to_string());
            }
        }
        current_id = parent.id;
    }

    None
}
