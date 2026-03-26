use destack_source::{FileId, Span};
use serde::{Deserialize, Serialize};

use crate::core::SessionQueryIndexExt;
use crate::core::fuzzy::score_completion;
use crate::dir::SymbolKind;
use destack_workspace::{Session, SymbolIndexEntry, SymbolIndexKind};

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
/// Returns symbols whose names contain the query string (case insensitive).
pub fn workspace_symbols(
    session: &Session,
    query: &str,
    max_results: usize,
) -> Vec<WorkspaceSymbol> {
    // prepare the scored symbol buffer
    let mut scored_symbols: Vec<(u32, WorkspaceSymbol)> = Vec::new();

    // normalize query input
    let query = query.trim();

    // collect candidate entries from the workspace symbol index
    let entries = session.search_workspace_symbol_entries(query);

    // score the cached entries in memory
    for entry in entries {
        let Some(score) =
            score_workspace_symbol(&entry.name, entry.container_name.as_deref(), query)
        else {
            continue;
        };

        scored_symbols.push((score, workspace_symbol_from_index_entry(entry)));
    }

    // sort by score descending, then name and location for deterministic results
    scored_symbols.sort_by(|left, right| {
        let left_key = (
            std::cmp::Reverse(left.0),
            left.1.name.len(),
            left.1.name.to_lowercase(),
            left.1.file.0,
            left.1.range.start,
            left.1.range.end,
        );
        let right_key = (
            std::cmp::Reverse(right.0),
            right.1.name.len(),
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

    // enforce the maximum result limit
    if symbols.len() > max_results {
        symbols.truncate(max_results);
    }

    // return the final symbol list
    symbols
}

/// Score a workspace symbol against the query.
fn score_workspace_symbol(name: &str, _container: Option<&str>, query: &str) -> Option<u32> {
    // match the symbol name against the query
    score_completion(name, query).map(|matched| matched.score)
}

/// Convert one cached symbol entry to a workspace symbol.
fn workspace_symbol_from_index_entry(entry: SymbolIndexEntry) -> WorkspaceSymbol {
    WorkspaceSymbol {
        name: entry.name,
        kind: symbol_kind_from_index_kind(entry.kind),
        file: entry.file_id,
        range: entry.range,
        container: entry.container_name,
    }
}

/// Map one symbol index kind to the public workspace symbol kind.
fn symbol_kind_from_index_kind(kind: SymbolIndexKind) -> SymbolKind {
    match kind {
        SymbolIndexKind::Namespace => SymbolKind::Namespace,
        SymbolIndexKind::Class => SymbolKind::Class,
        SymbolIndexKind::Method => SymbolKind::Method,
        SymbolIndexKind::Field => SymbolKind::Field,
        SymbolIndexKind::Enum => SymbolKind::Enum,
        SymbolIndexKind::Interface => SymbolKind::Interface,
        SymbolIndexKind::Function => SymbolKind::Function,
        SymbolIndexKind::Variable => SymbolKind::Variable,
        SymbolIndexKind::Constant => SymbolKind::Constant,
        SymbolIndexKind::EnumMember => SymbolKind::EnumMember,
        SymbolIndexKind::Struct => SymbolKind::Struct,
        SymbolIndexKind::TypeParameter => SymbolKind::TypeParameter,
    }
}
