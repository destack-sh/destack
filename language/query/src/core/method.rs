use std::error::Error;
use std::fmt;

use serde_json::Value;

use super::protocol::{QueryExecutionMode, QueryRequest};

/// Category for query methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum QueryCategory {
    /// Read only queries.
    Query,
    /// Assistance queries (hover, completion, etc).
    Assist,
    /// Refactor queries that return edits.
    Refactor,
}

/// Unique identifiers for query methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QueryMethodId {
    /// Completion query.
    Completion,
    /// Hover query.
    Hover,
    /// Signature help query.
    SignatureHelp,
    /// Inlay hints query.
    InlayHints,
    /// Code lenses query.
    CodeLenses,
    /// Code lens resolve query.
    ResolveCodeLens,
    /// Folding ranges query.
    FoldingRanges,
    /// Full-document semantic tokens query.
    SemanticTokens,
    /// Range semantic tokens query.
    SemanticTokensRange,
    /// Document symbols query.
    DocumentSymbols,
    /// Workspace symbols query.
    WorkspaceSymbols,
    /// Document links query.
    DocumentLinks,
    /// Document link resolve query.
    ResolveDocumentLink,
    /// Document highlight query.
    DocumentHighlight,
    /// Selection ranges query.
    SelectionRanges,
    /// Goto definition query.
    GotoDefinition,
    /// Goto declaration query.
    GotoDeclaration,
    /// Goto type definition query.
    GotoTypeDefinition,
    /// Goto implementation query.
    GotoImplementation,
    /// Find references query.
    FindReferences,
    /// Call hierarchy prepare query.
    PrepareCallHierarchy,
    /// Incoming call hierarchy query.
    CallHierarchyIncoming,
    /// Outgoing call hierarchy query.
    CallHierarchyOutgoing,
    /// Type hierarchy prepare query.
    PrepareTypeHierarchy,
    /// Type hierarchy supertypes query.
    TypeHierarchySupertypes,
    /// Type hierarchy subtypes query.
    TypeHierarchySubtypes,
    /// Prepare rename query.
    PrepareRename,
    /// Rename query.
    Rename,
    /// Rename files query.
    RenameFiles,
    /// Extract function query.
    ExtractFunction,
    /// Extract variable query.
    ExtractVariable,
    /// Inline query.
    Inline,
    /// Change signature query.
    ChangeSignature,
    /// Code actions query.
    CodeActions,
}

impl QueryMethodId {
    /// Return the execution mode for this query method.
    pub fn execution_mode(self) -> QueryExecutionMode {
        match self {
            Self::Rename
            | Self::RenameFiles
            | Self::ExtractFunction
            | Self::ExtractVariable
            | Self::Inline
            | Self::ChangeSignature => QueryExecutionMode::Write,
            _ => QueryExecutionMode::Read,
        }
    }
}

/// Metadata for a query method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct QueryMethod {
    /// Unique method identifier.
    pub(crate) id: QueryMethodId,
    /// Canonical method name.
    pub(crate) name: &'static str,
    /// Alias names accepted by the CLI.
    pub(crate) aliases: &'static [&'static str],
    /// Category for the method.
    pub(crate) category: QueryCategory,
    /// Short summary of the method.
    pub(crate) summary: &'static str,
    /// Params type name for help output.
    pub(crate) params_type: &'static str,
    /// Result type name for help output.
    pub(crate) result_type: &'static str,
}

/// Query request parse error.
#[derive(Debug)]
pub enum QueryRequestParseError {
    /// Unknown method name.
    UnknownMethod {
        /// Method name from the caller.
        method: String,
    },
    /// Invalid params payload for a known method.
    InvalidParams {
        /// Canonical method name.
        method: &'static str,
        /// Expected params type name.
        params_type: &'static str,
        /// Json decode error.
        source: serde_json::Error,
    },
}

impl fmt::Display for QueryRequestParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownMethod { method } => write!(formatter, "unknown query method: {method}"),
            Self::InvalidParams {
                method,
                params_type,
                source,
            } => write!(
                formatter,
                "invalid params for query method `{method}` ({params_type}): {source}"
            ),
        }
    }
}

impl Error for QueryRequestParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::UnknownMethod { .. } => None,
            Self::InvalidParams { source, .. } => Some(source),
        }
    }
}

static QUERY_METHODS: &[QueryMethod] = &[
    QueryMethod {
        id: QueryMethodId::Completion,
        name: "completion",
        aliases: &["completions", "completioninfo", "textdocument/completion"],
        category: QueryCategory::Assist,
        summary: "list completion items at a position",
        params_type: "CompletionRequest",
        result_type: "CompletionResponse",
    },
    QueryMethod {
        id: QueryMethodId::Hover,
        name: "hover",
        aliases: &["quickinfo", "textdocument/hover"],
        category: QueryCategory::Assist,
        summary: "show hover information at a position",
        params_type: "HoverRequest",
        result_type: "HoverResponse",
    },
    QueryMethod {
        id: QueryMethodId::SignatureHelp,
        name: "signature_help",
        aliases: &["signaturehelp", "textdocument/signaturehelp"],
        category: QueryCategory::Assist,
        summary: "show signature help at a position",
        params_type: "SignatureHelpRequest",
        result_type: "SignatureHelpResponse",
    },
    QueryMethod {
        id: QueryMethodId::InlayHints,
        name: "inlay_hints",
        aliases: &["inlayhints", "textdocument/inlayhint"],
        category: QueryCategory::Assist,
        summary: "list inlay hints for a range",
        params_type: "InlayHintsRequest",
        result_type: "InlayHintsResponse",
    },
    QueryMethod {
        id: QueryMethodId::CodeLenses,
        name: "code_lens",
        aliases: &["codelens", "textdocument/codelens"],
        category: QueryCategory::Assist,
        summary: "list code lenses for a document",
        params_type: "CodeLensesRequest",
        result_type: "CodeLensesResponse",
    },
    QueryMethod {
        id: QueryMethodId::ResolveCodeLens,
        name: "resolve_code_lens",
        aliases: &["codelens/resolve", "textdocument/codelens/resolve"],
        category: QueryCategory::Assist,
        summary: "resolve a code lens",
        params_type: "ResolveCodeLensRequest",
        result_type: "ResolveCodeLensResponse",
    },
    QueryMethod {
        id: QueryMethodId::FoldingRanges,
        name: "folding_ranges",
        aliases: &["foldingrange", "textdocument/foldingrange"],
        category: QueryCategory::Assist,
        summary: "list folding ranges for a document",
        params_type: "FoldingRangesRequest",
        result_type: "FoldingRangesResponse",
    },
    QueryMethod {
        id: QueryMethodId::SemanticTokens,
        name: "semantic_tokens",
        aliases: &["semantictokens/full", "textdocument/semantictokens/full"],
        category: QueryCategory::Assist,
        summary: "list semantic tokens for a document",
        params_type: "SemanticTokensRequest",
        result_type: "SemanticTokensResponse",
    },
    QueryMethod {
        id: QueryMethodId::SemanticTokensRange,
        name: "semantic_tokens_range",
        aliases: &["semantictokens/range", "textdocument/semantictokens/range"],
        category: QueryCategory::Assist,
        summary: "list semantic tokens for a range",
        params_type: "SemanticTokensRangeRequest",
        result_type: "SemanticTokensResponse",
    },
    QueryMethod {
        id: QueryMethodId::DocumentSymbols,
        name: "document_symbols",
        aliases: &["documentsymbol", "textdocument/documentsymbol"],
        category: QueryCategory::Query,
        summary: "list document symbols for a document",
        params_type: "DocumentSymbolsRequest",
        result_type: "DocumentSymbolsResponse",
    },
    QueryMethod {
        id: QueryMethodId::WorkspaceSymbols,
        name: "workspace_symbols",
        aliases: &["workspacesymbol", "workspace/symbol"],
        category: QueryCategory::Query,
        summary: "search workspace symbols",
        params_type: "WorkspaceSymbolsRequest",
        result_type: "WorkspaceSymbolsResponse",
    },
    QueryMethod {
        id: QueryMethodId::DocumentLinks,
        name: "document_links",
        aliases: &["documentlink", "textdocument/documentlink"],
        category: QueryCategory::Query,
        summary: "list document links for a document",
        params_type: "DocumentLinksRequest",
        result_type: "DocumentLinksResponse",
    },
    QueryMethod {
        id: QueryMethodId::ResolveDocumentLink,
        name: "resolve_document_link",
        aliases: &["documentlink/resolve", "textdocument/documentlink/resolve"],
        category: QueryCategory::Query,
        summary: "resolve a document link",
        params_type: "ResolveDocumentLinkRequest",
        result_type: "ResolveDocumentLinkResponse",
    },
    QueryMethod {
        id: QueryMethodId::DocumentHighlight,
        name: "document_highlight",
        aliases: &["documenthighlight", "textdocument/documenthighlight"],
        category: QueryCategory::Query,
        summary: "list highlights at a position",
        params_type: "DocumentHighlightRequest",
        result_type: "DocumentHighlightResponse",
    },
    QueryMethod {
        id: QueryMethodId::SelectionRanges,
        name: "selection_ranges",
        aliases: &["selectionrange", "textdocument/selectionrange"],
        category: QueryCategory::Query,
        summary: "list selection ranges for positions",
        params_type: "SelectionRangesRequest",
        result_type: "SelectionRangesResponse",
    },
    QueryMethod {
        id: QueryMethodId::GotoDefinition,
        name: "definition",
        aliases: &["gotodefinition", "textdocument/definition"],
        category: QueryCategory::Query,
        summary: "find definition locations",
        params_type: "GotoDefinitionRequest",
        result_type: "GotoDefinitionResponse",
    },
    QueryMethod {
        id: QueryMethodId::GotoDeclaration,
        name: "declaration",
        aliases: &["gotodeclaration", "textdocument/declaration"],
        category: QueryCategory::Query,
        summary: "find declaration locations",
        params_type: "GotoDeclarationRequest",
        result_type: "GotoDeclarationResponse",
    },
    QueryMethod {
        id: QueryMethodId::GotoTypeDefinition,
        name: "type_definition",
        aliases: &["gototypedefinition", "textdocument/typedefinition"],
        category: QueryCategory::Query,
        summary: "find type definition locations",
        params_type: "GotoTypeDefinitionRequest",
        result_type: "GotoTypeDefinitionResponse",
    },
    QueryMethod {
        id: QueryMethodId::GotoImplementation,
        name: "implementation",
        aliases: &["gotoimplementation", "textdocument/implementation"],
        category: QueryCategory::Query,
        summary: "find implementation locations",
        params_type: "GotoImplementationRequest",
        result_type: "GotoImplementationResponse",
    },
    QueryMethod {
        id: QueryMethodId::FindReferences,
        name: "references",
        aliases: &["findreferences", "textdocument/references"],
        category: QueryCategory::Query,
        summary: "find symbol references",
        params_type: "FindReferencesRequest",
        result_type: "FindReferencesResponse",
    },
    QueryMethod {
        id: QueryMethodId::PrepareCallHierarchy,
        name: "prepare_call_hierarchy",
        aliases: &["preparecallhierarchy", "textdocument/preparecallhierarchy"],
        category: QueryCategory::Query,
        summary: "prepare a call hierarchy item",
        params_type: "PrepareCallHierarchyRequest",
        result_type: "PrepareCallHierarchyResponse",
    },
    QueryMethod {
        id: QueryMethodId::CallHierarchyIncoming,
        name: "call_hierarchy_incoming",
        aliases: &["callhierarchy/incomingcalls", "callhierarchy/incoming"],
        category: QueryCategory::Query,
        summary: "list incoming call hierarchy edges",
        params_type: "CallHierarchyIncomingRequest",
        result_type: "CallHierarchyIncomingResponse",
    },
    QueryMethod {
        id: QueryMethodId::CallHierarchyOutgoing,
        name: "call_hierarchy_outgoing",
        aliases: &["callhierarchy/outgoingcalls", "callhierarchy/outgoing"],
        category: QueryCategory::Query,
        summary: "list outgoing call hierarchy edges",
        params_type: "CallHierarchyOutgoingRequest",
        result_type: "CallHierarchyOutgoingResponse",
    },
    QueryMethod {
        id: QueryMethodId::PrepareTypeHierarchy,
        name: "prepare_type_hierarchy",
        aliases: &["preparetypehierarchy", "textdocument/preparetypehierarchy"],
        category: QueryCategory::Query,
        summary: "prepare a type hierarchy item",
        params_type: "PrepareTypeHierarchyRequest",
        result_type: "PrepareTypeHierarchyResponse",
    },
    QueryMethod {
        id: QueryMethodId::TypeHierarchySupertypes,
        name: "type_hierarchy_supertypes",
        aliases: &["typehierarchy/supertypes"],
        category: QueryCategory::Query,
        summary: "list type hierarchy supertypes",
        params_type: "TypeHierarchySupertypesRequest",
        result_type: "TypeHierarchySupertypesResponse",
    },
    QueryMethod {
        id: QueryMethodId::TypeHierarchySubtypes,
        name: "type_hierarchy_subtypes",
        aliases: &["typehierarchy/subtypes"],
        category: QueryCategory::Query,
        summary: "list type hierarchy subtypes",
        params_type: "TypeHierarchySubtypesRequest",
        result_type: "TypeHierarchySubtypesResponse",
    },
    QueryMethod {
        id: QueryMethodId::PrepareRename,
        name: "prepare_rename",
        aliases: &["preparerename", "textdocument/preparerename"],
        category: QueryCategory::Refactor,
        summary: "prepare rename at a position",
        params_type: "PrepareRenameRequest",
        result_type: "PrepareRenameResponse",
    },
    QueryMethod {
        id: QueryMethodId::Rename,
        name: "rename",
        aliases: &["textdocument/rename"],
        category: QueryCategory::Refactor,
        summary: "rename a symbol",
        params_type: "RenameRequest",
        result_type: "RenameResponse",
    },
    QueryMethod {
        id: QueryMethodId::RenameFiles,
        name: "rename_files",
        aliases: &["renamefiles", "workspace/willrenamefiles"],
        category: QueryCategory::Refactor,
        summary: "rename file imports across the workspace",
        params_type: "RenameFilesRequest",
        result_type: "RenameFilesResponse",
    },
    QueryMethod {
        id: QueryMethodId::ExtractFunction,
        name: "extract_function",
        aliases: &["extractfunction", "refactor/extract_function"],
        category: QueryCategory::Refactor,
        summary: "extract a selection into a new function",
        params_type: "ExtractFunctionRequest",
        result_type: "ExtractFunctionResponse",
    },
    QueryMethod {
        id: QueryMethodId::ExtractVariable,
        name: "extract_variable",
        aliases: &["extractvariable", "refactor/extract_variable"],
        category: QueryCategory::Refactor,
        summary: "extract a selection into a const binding",
        params_type: "ExtractVariableRequest",
        result_type: "ExtractVariableResponse",
    },
    QueryMethod {
        id: QueryMethodId::Inline,
        name: "inline",
        aliases: &["inlinevalue", "refactor/inline"],
        category: QueryCategory::Refactor,
        summary: "inline a symbol at a position",
        params_type: "InlineRequest",
        result_type: "InlineResponse",
    },
    QueryMethod {
        id: QueryMethodId::ChangeSignature,
        name: "change_signature",
        aliases: &[
            "changesignature",
            "refactor/change_signature",
            "change_signature_preview",
            "refactor/change_signature_preview",
        ],
        category: QueryCategory::Refactor,
        summary: "change a function signature and update call sites",
        params_type: "ChangeSignatureRequest",
        result_type: "ChangeSignatureResponse",
    },
    QueryMethod {
        id: QueryMethodId::CodeActions,
        name: "code_actions",
        aliases: &["codeaction", "textdocument/codeaction"],
        category: QueryCategory::Refactor,
        summary: "list code actions for a range",
        params_type: "CodeActionsRequest",
        result_type: "CodeActionsResponse",
    },
];

/// Resolve a query method by name or alias.
pub(crate) fn query_method(name: &str) -> Option<&'static QueryMethod> {
    let normalized = normalize_method_name(name);
    QUERY_METHODS.iter().find(|method| {
        normalize_method_name(method.name) == normalized
            || method
                .aliases
                .iter()
                .any(|alias| normalize_method_name(alias) == normalized)
    })
}

/// Parse a query request for a method name and params.
pub fn parse_query_request(
    method: &str,
    params: Value,
) -> Result<QueryRequest, QueryRequestParseError> {
    let Some(method) = query_method(method) else {
        return Err(QueryRequestParseError::UnknownMethod {
            method: method.to_string(),
        });
    };

    let request = match method.id {
        QueryMethodId::Completion => QueryRequest::Completion(parse_params(method, params)?),
        QueryMethodId::Hover => QueryRequest::Hover(parse_params(method, params)?),
        QueryMethodId::SignatureHelp => QueryRequest::SignatureHelp(parse_params(method, params)?),
        QueryMethodId::InlayHints => QueryRequest::InlayHints(parse_params(method, params)?),
        QueryMethodId::CodeLenses => QueryRequest::CodeLenses(parse_params(method, params)?),
        QueryMethodId::ResolveCodeLens => {
            QueryRequest::ResolveCodeLens(parse_params(method, params)?)
        }
        QueryMethodId::FoldingRanges => QueryRequest::FoldingRanges(parse_params(method, params)?),
        QueryMethodId::SemanticTokens => {
            QueryRequest::SemanticTokens(parse_params(method, params)?)
        }
        QueryMethodId::SemanticTokensRange => {
            QueryRequest::SemanticTokensRange(parse_params(method, params)?)
        }
        QueryMethodId::DocumentSymbols => {
            QueryRequest::DocumentSymbols(parse_params(method, params)?)
        }
        QueryMethodId::WorkspaceSymbols => {
            QueryRequest::WorkspaceSymbols(parse_params(method, params)?)
        }
        QueryMethodId::DocumentLinks => QueryRequest::DocumentLinks(parse_params(method, params)?),
        QueryMethodId::ResolveDocumentLink => {
            QueryRequest::ResolveDocumentLink(parse_params(method, params)?)
        }
        QueryMethodId::DocumentHighlight => {
            QueryRequest::DocumentHighlight(parse_params(method, params)?)
        }
        QueryMethodId::SelectionRanges => {
            QueryRequest::SelectionRanges(parse_params(method, params)?)
        }
        QueryMethodId::GotoDefinition => {
            QueryRequest::GotoDefinition(parse_params(method, params)?)
        }
        QueryMethodId::GotoDeclaration => {
            QueryRequest::GotoDeclaration(parse_params(method, params)?)
        }
        QueryMethodId::GotoTypeDefinition => {
            QueryRequest::GotoTypeDefinition(parse_params(method, params)?)
        }
        QueryMethodId::GotoImplementation => {
            QueryRequest::GotoImplementation(parse_params(method, params)?)
        }
        QueryMethodId::FindReferences => {
            QueryRequest::FindReferences(parse_params(method, params)?)
        }
        QueryMethodId::PrepareCallHierarchy => {
            QueryRequest::PrepareCallHierarchy(parse_params(method, params)?)
        }
        QueryMethodId::CallHierarchyIncoming => {
            QueryRequest::CallHierarchyIncoming(parse_params(method, params)?)
        }
        QueryMethodId::CallHierarchyOutgoing => {
            QueryRequest::CallHierarchyOutgoing(parse_params(method, params)?)
        }
        QueryMethodId::PrepareTypeHierarchy => {
            QueryRequest::PrepareTypeHierarchy(parse_params(method, params)?)
        }
        QueryMethodId::TypeHierarchySupertypes => {
            QueryRequest::TypeHierarchySupertypes(parse_params(method, params)?)
        }
        QueryMethodId::TypeHierarchySubtypes => {
            QueryRequest::TypeHierarchySubtypes(parse_params(method, params)?)
        }
        QueryMethodId::PrepareRename => QueryRequest::PrepareRename(parse_params(method, params)?),
        QueryMethodId::Rename => QueryRequest::Rename(parse_params(method, params)?),
        QueryMethodId::RenameFiles => QueryRequest::RenameFiles(parse_params(method, params)?),
        QueryMethodId::ExtractFunction => {
            QueryRequest::ExtractFunction(parse_params(method, params)?)
        }
        QueryMethodId::ExtractVariable => {
            QueryRequest::ExtractVariable(parse_params(method, params)?)
        }
        QueryMethodId::Inline => QueryRequest::Inline(parse_params(method, params)?),
        QueryMethodId::ChangeSignature => {
            QueryRequest::ChangeSignature(parse_params(method, params)?)
        }
        QueryMethodId::CodeActions => QueryRequest::CodeActions(parse_params(method, params)?),
    };

    // keep method id mappings synchronized across query surfaces
    debug_assert_eq!(request.method_id(), method.id);

    Ok(request)
}

/// Normalize query method names for lookup.
fn normalize_method_name(name: &str) -> String {
    name.trim().to_ascii_lowercase()
}

/// Parse typed params for a query method.
fn parse_params<T: serde::de::DeserializeOwned>(
    method: &QueryMethod,
    params: Value,
) -> Result<T, QueryRequestParseError> {
    serde_json::from_value(params).map_err(|source| QueryRequestParseError::InvalidParams {
        method: method.name,
        params_type: method.params_type,
        source,
    })
}
