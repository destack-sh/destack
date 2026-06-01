pub(crate) mod context;
mod import;
mod method;
pub(crate) mod path;
mod protocol;
mod provide;
mod query;
mod target;
mod workspace;

pub(crate) use context::{DirQueryContext, require_module_query_context};
pub use context::{
    ModuleQueryContext, WorkspaceQueryContext, module_query_context,
    module_query_context_from_checked, workspace_query_context,
};
pub(crate) use destack_qir::{
    AnnotationIndex, CallEntry, CallIndex, ExtensionEntry, ExtensionIndex, ImportEntry,
    ImportIndex, ImportSortKey, MatchKind, MatchQuality, MemberEntry, MemberIndex,
    MemberKind as MemberEntryKind, MemberSource, Name, NominalEntry, NominalIndex, NominalRelation,
    ReferenceEntry, ReferenceIndex, SpecifierEntry, SpecifierIndex, SymbolEntry, SymbolIndex,
    SymbolKind as SymbolEntryKind, import_sort_key, import_sort_text, match_quality,
    symbol_relevance, symbol_sort_key,
};
pub(crate) use import::repository_import_relevance;
pub use method::{
    QueryCategory, QueryMethod, QueryMethodId, QueryRequestParseError, parse_query_request,
    query_method, query_methods,
};
pub use protocol::{QueryRequest, QueryResponse};
pub use query::Query;
pub use target::{QueryModule, QueryPosition, QueryRange, QueryTarget, QueryText};
