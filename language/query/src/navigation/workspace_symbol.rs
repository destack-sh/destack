use destack_source::ProfileId;
use serde::{Deserialize, Serialize};

use crate::core::{
    MemberEntry, QueryModule, QueryTarget, SymbolEntry, WorkspaceQueryContext, match_quality,
    symbol_relevance, symbol_sort_key,
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
        name: entry.name.text(),
        kind: entry.kind.into(),
        target,
        container: entry.container_name,
    }
}

/// Convert one cached member entry to a workspace symbol.
fn workspace_symbol_from_member_entry(
    profile_id: ProfileId,
    entry: MemberEntry,
) -> WorkspaceSymbol {
    let module = QueryModule {
        module_id: entry.module_id,
        profile_id,
    };
    let target = QueryTarget::span(module, entry.range).with_symbol(entry.member_symbol);

    WorkspaceSymbol {
        name: entry.name.text(),
        kind: entry.kind.into(),
        target,
        container: entry.container_name,
    }
}

impl WorkspaceQueryContext<'_> {
    /// Search for symbols across the workspace.
    ///
    /// Returns symbols whose names match the lexical query.
    pub fn workspace_symbols(&self, query: &str, max_results: usize) -> Vec<WorkspaceSymbol> {
        let ctx = self;
        // prepare the scored symbol buffer
        let mut scored_symbols = Vec::new();

        // normalize query input
        let query = query.trim();

        // collect matching entries from the workspace symbol index
        for (profile_id, entry) in ctx.search_symbol_candidates(query) {
            let Some(relevance) = symbol_relevance(&entry, query) else {
                continue;
            };

            let sort_key = symbol_sort_key(&relevance, &entry);
            scored_symbols.push((
                sort_key,
                workspace_symbol_from_index_entry(profile_id, entry),
            ));
        }

        // collect matching entries from the workspace member index
        for (profile_id, entry) in ctx.search_member_candidates(query) {
            let name = entry.name.text();
            let Some(relevance) = match_quality(&name, query) else {
                continue;
            };

            let sort_key = (
                relevance.sort_key(),
                4,
                0,
                1,
                None,
                entry.container_name.clone(),
                name.chars().count(),
                name.to_lowercase(),
                entry.file_id.0,
                entry.range.start,
                entry.range.end,
            );
            scored_symbols.push((
                sort_key,
                workspace_symbol_from_member_entry(profile_id, entry),
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
}
