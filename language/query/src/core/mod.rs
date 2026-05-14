pub(crate) mod context;
mod import;
mod method;
pub(crate) mod path;
mod protocol;
mod provide;
mod query;
mod scope;
mod workspace;

pub(crate) use context::{
    DirQueryContext, QueryContext, SourceQueryContext, query_context, query_context_for_profile,
    with_query_context_for_file, with_query_context_for_module, with_source_query_for_file,
    with_source_query_for_module,
};
pub(crate) use destack_qir::{
    CallEntry, CallIndex, ExtensionEntry, ExtensionIndex, ImportEntry, ImportIndex, ImportSortKey,
    MatchKind, MatchQuality, NominalEntry, NominalIndex, NominalRelation, ReferenceEntry,
    ReferenceIndex, SpecifierEntry, SpecifierIndex, SymbolEntry, SymbolIndex,
    SymbolKind as SymbolEntryKind, import_sort_key, import_sort_text, match_quality,
    symbol_relevance, symbol_sort_key,
};
pub(crate) use import::repository_import_relevance;
pub use method::{
    QueryCategory, QueryMethod, QueryMethodId, QueryRequestParseError, parse_query_request,
    query_method, query_methods,
};
pub use protocol::{QueryExecutionMode, QueryRequest, QueryResponse};
pub use query::Query;
pub use scope::QueryScope;
pub(crate) use workspace::{
    call_candidates_for_callee, call_candidates_for_caller, extension_candidates_for_target,
    modules_referencing_symbol, nominal_relations_for_target, search_import_candidates,
    search_workspace_symbol_candidates, specifier_candidates_for_rename_paths,
};
