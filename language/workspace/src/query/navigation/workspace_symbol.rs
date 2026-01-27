use std::panic;

use destack_base::StringPool;
use destack_dir as dir;
use destack_source::{FileId, Span};
use serde::{Deserialize, Serialize};

use super::document_symbol::SymbolKind;
use crate::Session;

/// A symbol in the workspace (flat list for workspace symbol search).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

/// Request workspace symbols for a query string.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceSymbolsRequest {
    /// The search query string.
    pub query: String,
    /// The maximum number of results.
    pub max_results: u32,
}

/// Response payload for workspace symbols queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceSymbolsResponse {
    /// Workspace symbols.
    pub symbols: Vec<WorkspaceSymbol>,
}

/// Search for symbols across the workspace.
///
/// Returns symbols whose names contain the query string (case-insensitive).
pub fn workspace_symbols(
    session: &Session,
    query: &str,
    max_results: usize,
) -> Vec<WorkspaceSymbol> {
    let mut scored_symbols: Vec<(u8, WorkspaceSymbol)> = Vec::new();
    let query_lower = query.to_lowercase();

    // search all modules
    for module in session.modules.iter() {
        let module = module.read();
        let Some(ctx) = session.query_context(&module) else {
            continue;
        };

        let dir_tree = ctx.tree();

        // iterate through all declarations
        for (declaration_id, declaration) in dir_tree.iter_nodes_of_type::<dir::Declaration>() {
            // get the declaration name
            let descriptor = declaration.descriptor();
            let Some(name) = descriptor.name else {
                continue;
            };

            let name = session.strings.get(name.string()).to_string();
            let name_lower = name.to_lowercase();

            // check whether the symbol matches the query and compute a score
            let Some(score) = match_score(&name_lower, &query_lower) else {
                continue;
            };

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
            let container = find_container_name(&dir_tree, &session.strings, declaration_id.id);

            // get span (safely, skipping if out of bounds)
            let ast_node_id = dir_tree.get_source(declaration_id.id);
            let full_span = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                ctx.ast.tree.source_map.get(ast_node_id)
            }));

            let Ok(full_span) = full_span else {
                continue;
            };

            scored_symbols.push((
                score,
                WorkspaceSymbol {
                    name,
                    kind,
                    file: ctx.file_id,
                    range: full_span,
                    container,
                },
            ));
        }
    }

    // sort by score, then name, then location for deterministic results
    scored_symbols.sort_by(|left, right| {
        let left_key = (
            left.0,
            left.1.name.to_lowercase(),
            left.1.file.0,
            left.1.range.start,
            left.1.range.end,
        );
        let right_key = (
            right.0,
            right.1.name.to_lowercase(),
            right.1.file.0,
            right.1.range.start,
            right.1.range.end,
        );
        left_key.cmp(&right_key)
    });

    // drop duplicate symbol locations
    let mut symbols = Vec::new();
    for (_, symbol) in scored_symbols {
        let is_duplicate = symbols.iter().any(|existing: &WorkspaceSymbol| {
            existing.name == symbol.name
                && existing.kind == symbol.kind
                && existing.file == symbol.file
                && existing.range.start == symbol.range.start
                && existing.range.end == symbol.range.end
        });
        if !is_duplicate {
            symbols.push(symbol);
        }
    }

    if symbols.len() > max_results {
        symbols.truncate(max_results);
    }

    symbols
}

/// Score how well a symbol name matches the query.
fn match_score(name_lower: &str, query_lower: &str) -> Option<u8> {
    if query_lower.is_empty() {
        return Some(2);
    }

    if name_lower == query_lower {
        return Some(0);
    }

    if name_lower.starts_with(query_lower) {
        return Some(1);
    }

    if name_lower.contains(query_lower) {
        return Some(2);
    }

    None
}

/// Find the container name for a node by walking up the parent tree.
/// NOTE #Performance: find workspace symbol containers more efficiently?
fn find_container_name(
    dir_tree: &dir::NodeTree,
    strings: &StringPool,
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
            if let Some(name) = descriptor.name {
                return Some(strings.get(name.string()).to_string());
            }
        }
        current_id = parent.id;
    }

    None
}
