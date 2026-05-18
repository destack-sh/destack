use destack_source::ProfileId;
use serde::{Deserialize, Serialize};

use crate::core::{
    QueryModule, QueryTarget, SymbolEntry, SymbolEntryKind, WorkspaceQueryContext,
    search_workspace_symbol_candidates, symbol_relevance, symbol_sort_key,
};
use crate::dir::SymbolKind;

/// A symbol in the workspace (flat list for workspace symbol search).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceSymbol {
    /// The symbol's name.
    pub name: String,
    /// The kind of symbol.
    pub kind: SymbolKind,
    /// The symbol source target.
    pub target: QueryTarget,
    /// Container name (e.g., class name for methods).
    pub container: Option<String>,
}

/// Request workspace symbols for a query string.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceSymbolsRequest {
    /// The profiles to search.
    pub profile_ids: Vec<ProfileId>,
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
/// Returns symbols whose names match the lexical query.
pub fn workspace_symbols(
    ctx: &WorkspaceQueryContext<'_>,
    query: &str,
    max_results: usize,
) -> Vec<WorkspaceSymbol> {
    // prepare the scored symbol buffer
    let mut scored_symbols = Vec::new();

    // normalize query input
    let query = query.trim();

    // collect matching entries from the workspace symbol index
    for (profile_id, entry) in search_workspace_symbol_candidates(ctx, query) {
        let Some(relevance) = symbol_relevance(&entry, query) else {
            continue;
        };

        let sort_key = symbol_sort_key(&relevance, &entry);
        scored_symbols.push((
            sort_key,
            workspace_symbol_from_index_entry(profile_id, entry),
        ));
    }

    // sort by lexical relevance, then kind, then location for deterministic results
    scored_symbols.sort_by(|left, right| left.0.cmp(&right.0));

    let mut symbols: Vec<_> = scored_symbols
        .into_iter()
        .map(|(_, symbol)| symbol)
        .collect();

    // enforce the maximum result limit
    if symbols.len() > max_results {
        symbols.truncate(max_results);
    }

    // return the final symbol list
    symbols
}

/// Convert one cached symbol entry to a workspace symbol.
fn workspace_symbol_from_index_entry(profile_id: ProfileId, entry: SymbolEntry) -> WorkspaceSymbol {
    let module = QueryModule {
        module_id: entry.module_id,
        profile_id,
    };
    let target = QueryTarget::span(module, entry.range);
    let target = if let Some(symbol_id) = entry.symbol_id {
        target.with_symbol(symbol_id)
    } else {
        target
    };

    WorkspaceSymbol {
        name: entry.name,
        kind: symbol_kind_from_index_kind(entry.kind),
        target,
        container: entry.container_name,
    }
}

/// Map one symbol index kind to the public workspace symbol kind.
fn symbol_kind_from_index_kind(kind: SymbolEntryKind) -> SymbolKind {
    match kind {
        SymbolEntryKind::Namespace => SymbolKind::Namespace,
        SymbolEntryKind::Class => SymbolKind::Class,
        SymbolEntryKind::Method => SymbolKind::Method,
        SymbolEntryKind::Field => SymbolKind::Field,
        SymbolEntryKind::Enum => SymbolKind::Enum,
        SymbolEntryKind::Interface => SymbolKind::Interface,
        SymbolEntryKind::Function => SymbolKind::Function,
        SymbolEntryKind::Variable => SymbolKind::Variable,
        SymbolEntryKind::Constant => SymbolKind::Constant,
        SymbolEntryKind::EnumMember => SymbolKind::EnumMember,
        SymbolEntryKind::Struct => SymbolKind::Struct,
        SymbolEntryKind::TypeParameter => SymbolKind::TypeParameter,
    }
}
