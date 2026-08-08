use destack_dir as dir;
use destack_serde::Reflect;
use destack_source::{FileId, ProfileId, Span};
use serde::{Deserialize, Serialize};

use crate::{
    MatchOrder, MatchQuality, Module, ProgramQueryContext, QueryError, QueryResult, SymbolKind,
    Target, match_quality,
};

/// One symbol search result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SearchSymbol {
    /// The symbol's name.
    pub name: String,
    /// The kind of symbol.
    pub kind: SymbolKind,
    /// The exact symbol identity.
    pub symbol_id: dir::GlobalSymbolId,
    /// The symbol source target.
    pub target: Target,
    /// The containing declaration name.
    pub container: Option<String>,
}

/// The stable order for one symbol search.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SearchSymbolOrder {
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

/// A symbol search request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SearchSymbolsRequest {
    /// The search query string.
    pub query: String,
    /// The maximum number of results.
    pub max_results: u32,
}

/// A symbol search response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SearchSymbolsResponse {
    /// The matching symbols.
    pub symbols: Vec<SearchSymbol>,
}

/// Search for symbols across indexed programs.
pub fn search_symbols(
    request: SearchSymbolsRequest,
    programs: &[ProgramQueryContext<'_>],
) -> QueryResult<SearchSymbolsResponse> {
    let max_results = request.max_results as usize;
    if max_results == 0 {
        return Ok(SearchSymbolsResponse {
            symbols: Vec::new(),
        });
    }

    let mut candidates = Vec::new();

    // trim surrounding query whitespace
    let query = request.query.trim();

    // collect matching declarations
    for program in programs {
        for (profile_id, entry) in program.search_symbol_candidates(query)? {
            if let Some(candidate) = SymbolCandidate::symbol_entry(profile_id, entry, query)? {
                candidates.push(candidate);
            }
        }
    }

    // sort by lexical relevance, kind, container, and source location
    candidates.sort_by(|left, right| left.order.cmp(&right.order));
    let mut symbols = candidates
        .into_iter()
        .map(|candidate| candidate.symbol)
        .collect::<Vec<_>>();

    // collapse authored declarations repeated across query profiles
    symbols.dedup_by(|left, right| {
        left.name == right.name
            && left.kind == right.kind
            && left.container == right.container
            && left.target.span == right.target.span
            && left.target.selection_span == right.target.selection_span
    });
    symbols.truncate(max_results);

    Ok(SearchSymbolsResponse { symbols })
}

impl SearchSymbol {
    /// Return this symbol's stable order for one search query.
    pub fn order(&self, query: &str) -> Option<SearchSymbolOrder> {
        let query = query.trim();
        let relevance =
            SymbolRelevance::new(&self.name, self.container.as_deref(), self.kind, query)?;

        Some(SearchSymbolOrder::new(
            &relevance,
            &self.name,
            self.container.as_deref(),
            self.target.span.file,
            self.target.span,
        ))
    }

    /// Convert one cached symbol entry to a symbol match.
    fn symbol_entry(
        profile_id: ProfileId,
        entry: dir::SymbolEntry,
        kind: SymbolKind,
    ) -> QueryResult<Self> {
        let module = Module {
            module_id: entry.symbol.module_id,
            profile_id,
        };
        let target = Target::new(module, entry.span).with_selection_span(entry.selection)?;

        Ok(Self {
            name: entry.name,
            kind,
            symbol_id: entry.symbol,
            target,
            container: entry.container,
        })
    }
}

/// One matched symbol with its stable search order.
#[derive(Debug, Clone, PartialEq)]
struct SymbolCandidate {
    /// The stable search order.
    order: SearchSymbolOrder,
    /// The returned symbol search.
    symbol: SearchSymbol,
}

impl SymbolCandidate {
    /// Build one candidate from an indexed symbol entry.
    fn symbol_entry(
        profile_id: ProfileId,
        entry: dir::SymbolEntry,
        query: &str,
    ) -> QueryResult<Option<Self>> {
        // classify supported indexed declarations
        let mut kind = match entry.member_kind {
            Some(kind) => SymbolKind::from(kind),
            None => SymbolKind::try_from(entry.kind).map_err(|kind| {
                QueryError::invalid(format!(
                    "search symbol kind: {:?}, {:?}",
                    entry.symbol, kind
                ))
            })?,
        };
        if kind == SymbolKind::Variable && entry.mutability == Some(dir::Mutability::Immutable) {
            kind = SymbolKind::Constant;
        }

        // rank the exact declaration candidate
        let symbol = SearchSymbol::symbol_entry(profile_id, entry, kind)?;
        let Some(order) = symbol.order(query) else {
            return Ok(None);
        };

        Ok(Some(Self { order, symbol }))
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
    fn new(name: &str, container: Option<&str>, kind: SymbolKind, query: &str) -> Option<Self> {
        let lexical = match_quality(name, query)?;
        let kind_priority = kind.priority();
        let container_lexical = container.and_then(|name| match_quality(name, query));
        let has_container = container.is_some();

        Some(Self {
            lexical,
            kind_priority,
            container_lexical,
            has_container,
        })
    }
}

impl SearchSymbolOrder {
    /// Build the stable order for one indexed declaration.
    fn new(
        relevance: &SymbolRelevance,
        name: &str,
        container: Option<&str>,
        file: FileId,
        span: Span,
    ) -> Self {
        Self {
            lexical: relevance.lexical.order(),
            kind_priority: relevance.kind_priority,
            container_miss: u8::from(relevance.container_lexical.is_none()),
            has_container: u8::from(relevance.has_container),
            container_lexical: relevance
                .container_lexical
                .as_ref()
                .map(MatchQuality::order),
            container_name: container.map(str::to_string),
            name_length: name.chars().count(),
            name: name.to_lowercase(),
            file_id: file.0,
            start: span.start,
            end: span.end,
        }
    }
}

impl SymbolKind {
    /// Return the coarse workspace-search priority.
    fn priority(self) -> u8 {
        match self {
            Self::Class
            | Self::Enum
            | Self::Interface
            | Self::Module
            | Self::Namespace
            | Self::Newtype
            | Self::NewtypeInterface
            | Self::Struct => 1,
            Self::Constructor | Self::Function | Self::Method => 2,
            Self::AssociatedConst
            | Self::Constant
            | Self::EnumMember
            | Self::Field
            | Self::Property
            | Self::Variable => 3,
            Self::AssociatedType | Self::TypeAlias => 5,
            Self::Extension => 6,
        }
    }
}

impl From<dir::MemberKind> for SymbolKind {
    /// Convert one indexed member kind into its editor-facing kind.
    fn from(kind: dir::MemberKind) -> Self {
        match kind {
            dir::MemberKind::AssociatedConst => Self::AssociatedConst,
            dir::MemberKind::AssociatedType => Self::AssociatedType,
            dir::MemberKind::CallSignature => Self::Method,
            dir::MemberKind::Constructor | dir::MemberKind::ConstructSignature => Self::Constructor,
            dir::MemberKind::Field | dir::MemberKind::IndexSignature => Self::Field,
            dir::MemberKind::Method => Self::Method,
            dir::MemberKind::Property => Self::Property,
            dir::MemberKind::Variant => Self::EnumMember,
        }
    }
}
