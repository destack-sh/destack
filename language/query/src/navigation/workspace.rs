use destack_dir as dir;
use destack_serde::Reflect;
use destack_source::ProfileId;
use serde::{Deserialize, Serialize};

use crate::{MatchOrder, MatchQuality, Module, ProgramQueryContext, Target, match_quality};

/// One symbol search result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SymbolMatch {
    /// The symbol's name.
    pub name: String,
    /// The kind of symbol.
    pub kind: dir::SymbolKind,
    /// The symbol source target.
    pub target: Target,
    /// Container name (e.g., class name for methods).
    pub container: Option<String>,
}

/// Request symbol search for a query string.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SymbolSearchRequest {
    /// The profiles to search.
    pub profile_ids: Vec<ProfileId>,
    /// The search query string.
    pub query: String,
    /// The maximum number of results.
    pub max_results: u32,
}

/// Response payload for symbol search queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SymbolSearchResponse {
    /// The matching symbols.
    pub symbols: Vec<SymbolMatch>,
}

impl SymbolMatch {
    /// Convert one cached symbol entry to a symbol match.
    fn symbol_entry(profile_id: ProfileId, entry: dir::SymbolEntry) -> Self {
        let module = Module {
            module_id: entry.source.module_id,
            profile_id,
        };
        let target = Target::new(module, entry.span);
        let target = target.with_symbol_id(entry.symbol);

        Self {
            name: entry.name,
            kind: entry.kind,
            target,
            container: entry.container,
        }
    }

    /// Convert one cached member entry to a symbol match.
    fn member_entry(profile_id: ProfileId, entry: dir::MemberEntry) -> Self {
        let module = Module {
            module_id: entry.source.module_id,
            profile_id,
        };
        let target = Target::new(module, entry.span);
        let target = if let Some(symbol) = entry.symbol {
            target.with_symbol_id(symbol)
        } else {
            target
        };

        Self {
            name: entry.name,
            kind: Self::member_kind(entry.kind),
            target,
            container: entry.container,
        }
    }

    /// Return the DIR symbol kind closest to one indexed member kind.
    fn member_kind(kind: dir::MemberKind) -> dir::SymbolKind {
        match kind {
            dir::MemberKind::AssociatedConst => dir::SymbolKind::AssociatedConst,
            dir::MemberKind::AssociatedType => dir::SymbolKind::AssociatedType,
            dir::MemberKind::CallSignature => dir::SymbolKind::Function,
            dir::MemberKind::Constructor => dir::SymbolKind::Function,
            dir::MemberKind::ConstructSignature => dir::SymbolKind::Function,
            dir::MemberKind::Field => dir::SymbolKind::Variable,
            dir::MemberKind::IndexSignature => dir::SymbolKind::Function,
            dir::MemberKind::Method => dir::SymbolKind::Function,
            dir::MemberKind::Variant => dir::SymbolKind::Variant,
        }
    }
}

/// One matched symbol with its stable search order.
#[derive(Debug, Clone, PartialEq)]
struct SymbolCandidate {
    /// The stable search order.
    order: SymbolOrder,
    /// The returned symbol search.
    symbol: SymbolMatch,
}

impl SymbolCandidate {
    /// Build one candidate from an indexed symbol entry.
    fn symbol_entry(profile_id: ProfileId, entry: dir::SymbolEntry, query: &str) -> Option<Self> {
        let relevance = SymbolRelevance::symbol_entry(&entry, query)?;
        let order = SymbolOrder::symbol_entry(&relevance, &entry);
        let symbol = SymbolMatch::symbol_entry(profile_id, entry);

        Some(Self { order, symbol })
    }

    /// Build one candidate from an indexed member entry.
    fn member_entry(profile_id: ProfileId, entry: dir::MemberEntry, query: &str) -> Option<Self> {
        let relevance = match_quality(&entry.name, query)?;
        let order = SymbolOrder::member_entry(relevance, &entry);
        let symbol = SymbolMatch::member_entry(profile_id, entry);

        Some(Self { order, symbol })
    }
}

/// The lexical and structural relevance for one symbol entry.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SymbolRelevance {
    /// The lexical match quality for the symbol name.
    lexical: MatchQuality,
    /// The coarse symbol kind priority.
    kind_priority: u8,
    /// The container-name lexical match quality when available.
    container_lexical: Option<MatchQuality>,
    /// Whether this is a nested symbol rather than one top-level declaration.
    has_container: bool,
}

impl SymbolRelevance {
    /// Compute symbol relevance for one indexed entry.
    fn symbol_entry(entry: &dir::SymbolEntry, query: &str) -> Option<Self> {
        let lexical = match_quality(&entry.name, query)?;
        let kind_priority = Self::kind_priority(entry.kind);
        let container_lexical = entry
            .container
            .as_ref()
            .and_then(|container_name| match_quality(container_name, query));
        let has_container = entry.container.is_some();

        Some(Self {
            lexical,
            kind_priority,
            container_lexical,
            has_container,
        })
    }

    /// Return one coarse symbol kind priority for workspace-symbol ordering.
    fn kind_priority(kind: dir::SymbolKind) -> u8 {
        match kind {
            dir::SymbolKind::Class
            | dir::SymbolKind::Struct
            | dir::SymbolKind::Interface
            | dir::SymbolKind::NewtypeInterface
            | dir::SymbolKind::Enum
            | dir::SymbolKind::Newtype => 1,
            dir::SymbolKind::Function => 2,
            dir::SymbolKind::AssociatedConst
            | dir::SymbolKind::Variant
            | dir::SymbolKind::GenericValueParameter
            | dir::SymbolKind::Variable => 3,
            dir::SymbolKind::AssociatedType
            | dir::SymbolKind::GenericTypeParameter
            | dir::SymbolKind::TypeAlias => 5,
            dir::SymbolKind::Extension => 6,
            dir::SymbolKind::Import => 7,
            dir::SymbolKind::Label => 8,
        }
    }
}

/// The stable order for one symbol search.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SymbolOrder {
    /// The symbol-name lexical order.
    lexical: MatchOrder,
    /// The symbol-kind priority.
    kind_priority: u8,
    /// Whether the container name matched.
    container_miss: u8,
    /// Whether this symbol has a container.
    has_container: u8,
    /// The container-name lexical order when available.
    container_lexical: Option<MatchOrder>,
    /// The container display name.
    container_name: Option<String>,
    /// The display-name length.
    name_length: usize,
    /// The case-folded display name.
    name: String,
    /// The file id.
    file_id: u64,
    /// The source start offset.
    start: u32,
    /// The source end offset.
    end: u32,
}

impl SymbolOrder {
    /// Build the stable order for one indexed symbol.
    fn symbol_entry(relevance: &SymbolRelevance, entry: &dir::SymbolEntry) -> Self {
        Self {
            lexical: relevance.lexical.order(),
            kind_priority: relevance.kind_priority,
            container_miss: u8::from(relevance.container_lexical.is_none()),
            has_container: u8::from(relevance.has_container),
            container_lexical: relevance
                .container_lexical
                .as_ref()
                .map(MatchQuality::order),
            container_name: entry.container.clone(),
            name_length: entry.name.chars().count(),
            name: entry.name.to_lowercase(),
            file_id: entry.file.0,
            start: entry.span.start,
            end: entry.span.end,
        }
    }

    /// Build the stable order for one indexed member.
    fn member_entry(relevance: MatchQuality, entry: &dir::MemberEntry) -> Self {
        let name = entry.name.clone();

        Self {
            lexical: relevance.order(),
            kind_priority: 4,
            container_miss: 0,
            has_container: 1,
            container_lexical: None,
            container_name: entry.container.clone(),
            name_length: name.chars().count(),
            name: name.to_lowercase(),
            file_id: entry.file.0,
            start: entry.span.start,
            end: entry.span.end,
        }
    }
}

impl ProgramQueryContext<'_> {
    /// Search for symbols across indexed program profiles.
    ///
    /// Returns symbols whose names match the lexical query.
    pub fn search_symbols(&self, query: &str, max_results: usize) -> Vec<SymbolMatch> {
        let mut candidates = Vec::new();

        // normalize query input
        let query = query.trim();

        // collect matching entries from the program symbol index
        for (profile_id, entry) in self.search_symbol_candidates(query) {
            if let Some(candidate) = SymbolCandidate::symbol_entry(profile_id, entry, query) {
                candidates.push(candidate);
            }
        }

        // collect matching entries from the program member index
        for (profile_id, entry) in self.search_member_candidates(query) {
            if let Some(candidate) = SymbolCandidate::member_entry(profile_id, entry, query) {
                candidates.push(candidate);
            }
        }

        // sort by lexical relevance, then kind, then location for deterministic results
        candidates.sort_by(|left, right| left.order.cmp(&right.order));

        // collect the returned symbols
        let mut symbols: Vec<_> = candidates
            .into_iter()
            .map(|candidate| candidate.symbol)
            .collect();

        // enforce the maximum result limit
        if symbols.len() > max_results {
            symbols.truncate(max_results);
        }

        // return the final symbol list
        symbols
    }
}
