use crate::{MatchQuality, MatchSortKey, SymbolEntry, SymbolKind, match_quality};

/// The lexical and structural relevance for one symbol entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolRelevance {
    /// The lexical match quality for the symbol name.
    pub lexical: MatchQuality,
    /// The coarse symbol kind rank.
    pub kind_rank: u8,
    /// The container-name lexical match quality when available.
    pub container_lexical: Option<MatchQuality>,
    /// Whether this is a nested symbol rather than one top-level declaration.
    pub has_container: bool,
}

/// Compute symbol relevance for one indexed entry.
pub fn symbol_relevance(entry: &SymbolEntry, query: &str) -> Option<SymbolRelevance> {
    let name = entry.name.text();
    let lexical = match_quality(&name, query)?;
    let kind_rank = symbol_kind_rank(entry.kind);
    let container_lexical = entry
        .container_name
        .as_ref()
        .and_then(|container_name| match_quality(container_name, query));
    let has_container = entry.container_name.is_some();

    Some(SymbolRelevance {
        lexical,
        kind_rank,
        container_lexical,
        has_container,
    })
}

/// Return one stable sort key for one workspace symbol.
#[allow(clippy::type_complexity)]
pub fn symbol_sort_key(
    relevance: &SymbolRelevance,
    entry: &SymbolEntry,
) -> (
    MatchSortKey,
    u8,
    u8,
    u8,
    Option<MatchSortKey>,
    Option<String>,
    usize,
    String,
    u128,
    u32,
    u32,
) {
    (
        relevance.lexical.sort_key(),
        relevance.kind_rank,
        u8::from(relevance.container_lexical.is_none()),
        u8::from(relevance.has_container),
        relevance
            .container_lexical
            .as_ref()
            .map(MatchQuality::sort_key),
        entry.container_name.clone(),
        entry.name.text().chars().count(),
        entry.name.text().to_lowercase(),
        entry.file_id.0,
        entry.range.start,
        entry.range.end,
    )
}

/// Return one coarse symbol kind rank for workspace-symbol ordering.
fn symbol_kind_rank(kind: SymbolKind) -> u8 {
    match kind {
        SymbolKind::Namespace => 0,
        SymbolKind::Class | SymbolKind::Struct | SymbolKind::Interface | SymbolKind::Enum => 1,
        SymbolKind::Function => 2,
        SymbolKind::Constant | SymbolKind::Variable => 3,
        SymbolKind::TypeParameter => 5,
    }
}
