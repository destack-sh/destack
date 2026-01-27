use serde_json::Value;

use super::request::QueryRequest;

/// Category for query methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QueryCategory {
    /// Read only queries.
    Query,
    /// Assistance queries (hover, completion, etc).
    Assist,
    /// Refactor queries that return edits.
    Refactor,
}

impl QueryCategory {
    /// Return the category label.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Query => "query",
            Self::Assist => "assist",
            Self::Refactor => "refactor",
        }
    }
}

/// Unique identifiers for query methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QueryMethodId {
    Completion,
    Hover,
    SignatureHelp,
    InlayHints,
    CodeLenses,
    ResolveCodeLens,
    FoldingRanges,
    SemanticTokens,
    SemanticTokensRange,
    DocumentSymbols,
    WorkspaceSymbols,
    DocumentLinks,
    ResolveDocumentLink,
    DocumentHighlight,
    SelectionRanges,
    GotoDefinition,
    GotoDeclaration,
    GotoTypeDefinition,
    GotoImplementation,
    FindReferences,
    PrepareCallHierarchy,
    CallHierarchyIncoming,
    CallHierarchyOutgoing,
    PrepareTypeHierarchy,
    TypeHierarchySupertypes,
    TypeHierarchySubtypes,
    PrepareRename,
    Rename,
    CodeActions,
}

/// Metadata for a query method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueryMethod {
    /// Unique method identifier.
    pub id: QueryMethodId,
    /// Canonical method name.
    pub name: &'static str,
    /// Alias names accepted by the CLI.
    pub aliases: &'static [&'static str],
    /// Category for the method.
    pub category: QueryCategory,
    /// Short summary of the method.
    pub summary: &'static str,
    /// Params type name for help output.
    pub params_type: &'static str,
    /// Result type name for help output.
    pub result_type: &'static str,
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
        id: QueryMethodId::CodeActions,
        name: "code_actions",
        aliases: &["codeaction", "textdocument/codeaction"],
        category: QueryCategory::Refactor,
        summary: "list code actions for a range",
        params_type: "CodeActionsRequest",
        result_type: "CodeActionsResponse",
    },
];

/// Return the registry of query methods.
pub fn query_methods() -> &'static [QueryMethod] {
    QUERY_METHODS
}

/// Resolve a query method by name or alias.
pub fn query_method(name: &str) -> Option<&'static QueryMethod> {
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
pub fn parse_query_request(method: &str, params: Value) -> Result<QueryRequest, String> {
    let Some(method) = query_method(method) else {
        return Err(format!("unknown query method: {method}"));
    };

    let request = match method.id {
        QueryMethodId::Completion => QueryRequest::Completion(parse_params(params)?),
        QueryMethodId::Hover => QueryRequest::Hover(parse_params(params)?),
        QueryMethodId::SignatureHelp => QueryRequest::SignatureHelp(parse_params(params)?),
        QueryMethodId::InlayHints => QueryRequest::InlayHints(parse_params(params)?),
        QueryMethodId::CodeLenses => QueryRequest::CodeLenses(parse_params(params)?),
        QueryMethodId::ResolveCodeLens => QueryRequest::ResolveCodeLens(parse_params(params)?),
        QueryMethodId::FoldingRanges => QueryRequest::FoldingRanges(parse_params(params)?),
        QueryMethodId::SemanticTokens => QueryRequest::SemanticTokens(parse_params(params)?),
        QueryMethodId::SemanticTokensRange => {
            QueryRequest::SemanticTokensRange(parse_params(params)?)
        }
        QueryMethodId::DocumentSymbols => QueryRequest::DocumentSymbols(parse_params(params)?),
        QueryMethodId::WorkspaceSymbols => QueryRequest::WorkspaceSymbols(parse_params(params)?),
        QueryMethodId::DocumentLinks => QueryRequest::DocumentLinks(parse_params(params)?),
        QueryMethodId::ResolveDocumentLink => {
            QueryRequest::ResolveDocumentLink(parse_params(params)?)
        }
        QueryMethodId::DocumentHighlight => QueryRequest::DocumentHighlight(parse_params(params)?),
        QueryMethodId::SelectionRanges => QueryRequest::SelectionRanges(parse_params(params)?),
        QueryMethodId::GotoDefinition => QueryRequest::GotoDefinition(parse_params(params)?),
        QueryMethodId::GotoDeclaration => QueryRequest::GotoDeclaration(parse_params(params)?),
        QueryMethodId::GotoTypeDefinition => {
            QueryRequest::GotoTypeDefinition(parse_params(params)?)
        }
        QueryMethodId::GotoImplementation => {
            QueryRequest::GotoImplementation(parse_params(params)?)
        }
        QueryMethodId::FindReferences => QueryRequest::FindReferences(parse_params(params)?),
        QueryMethodId::PrepareCallHierarchy => {
            QueryRequest::PrepareCallHierarchy(parse_params(params)?)
        }
        QueryMethodId::CallHierarchyIncoming => {
            QueryRequest::CallHierarchyIncoming(parse_params(params)?)
        }
        QueryMethodId::CallHierarchyOutgoing => {
            QueryRequest::CallHierarchyOutgoing(parse_params(params)?)
        }
        QueryMethodId::PrepareTypeHierarchy => {
            QueryRequest::PrepareTypeHierarchy(parse_params(params)?)
        }
        QueryMethodId::TypeHierarchySupertypes => {
            QueryRequest::TypeHierarchySupertypes(parse_params(params)?)
        }
        QueryMethodId::TypeHierarchySubtypes => {
            QueryRequest::TypeHierarchySubtypes(parse_params(params)?)
        }
        QueryMethodId::PrepareRename => QueryRequest::PrepareRename(parse_params(params)?),
        QueryMethodId::Rename => QueryRequest::Rename(parse_params(params)?),
        QueryMethodId::CodeActions => QueryRequest::CodeActions(parse_params(params)?),
    };

    Ok(request)
}

fn normalize_method_name(name: &str) -> String {
    name.trim().to_ascii_lowercase()
}

fn parse_params<T: serde::de::DeserializeOwned>(params: Value) -> Result<T, String> {
    serde_json::from_value(params).map_err(|error| error.to_string())
}
