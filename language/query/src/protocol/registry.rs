use std::error::Error;
use std::fmt;

use serde::de::DeserializeOwned;
use serde_json::Value;

use super::QueryRequest;

/// Category for query methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QueryCategory {
    /// Navigation queries.
    Navigation,
    /// Symbol search and outline queries.
    Symbol,
    /// Hierarchy queries.
    Hierarchy,
    /// Editor-facing queries.
    Editor,
    /// Edit-producing queries.
    Edit,
}

impl QueryCategory {
    /// Return the category label.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Navigation => "navigation",
            Self::Symbol => "symbol",
            Self::Hierarchy => "hierarchy",
            Self::Editor => "editor",
            Self::Edit => "edit",
        }
    }
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
    /// Outline query.
    Outline,
    /// Symbol search query.
    SymbolSearch,
    /// Links query.
    Links,
    /// Highlight query.
    Highlight,
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
    /// Call item query.
    CallItem,
    /// Incoming calls query.
    IncomingCalls,
    /// Outgoing calls query.
    OutgoingCalls,
    /// Type hierarchy item query.
    TypeItem,
    /// Type hierarchy supertypes query.
    Supertypes,
    /// Type hierarchy subtypes query.
    Subtypes,
    /// Decorator query.
    Decorators,
    /// Rename target query.
    RenameTarget,
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

/// Metadata for a query method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueryMethod {
    /// Unique method identifier.
    pub id: QueryMethodId,
    /// Canonical method name.
    pub name: &'static str,
    /// Category for the method.
    pub category: QueryCategory,
    /// Short summary of the method.
    pub summary: &'static str,
    /// Params type name for help output.
    pub params_type: &'static str,
    /// Result type name for help output.
    pub result_type: &'static str,
}

impl QueryMethod {
    /// Return whether this method accepts a method name.
    pub fn matches_name(&self, name: &str) -> bool {
        // normalize caller text before matching the canonical name
        let name = name.trim();

        self.name.eq_ignore_ascii_case(name)
    }

    /// Parse one request payload for this method.
    pub fn parse_request(&self, params: Value) -> Result<QueryRequest, QueryRequestParseError> {
        // deserialize params into the request payload for this method
        let request = match self.id {
            QueryMethodId::Completion => QueryRequest::Completion(self.parse_params(params)?),
            QueryMethodId::Hover => QueryRequest::Hover(self.parse_params(params)?),
            QueryMethodId::SignatureHelp => QueryRequest::SignatureHelp(self.parse_params(params)?),
            QueryMethodId::InlayHints => QueryRequest::InlayHints(self.parse_params(params)?),
            QueryMethodId::CodeLenses => QueryRequest::CodeLenses(self.parse_params(params)?),
            QueryMethodId::ResolveCodeLens => {
                QueryRequest::ResolveCodeLens(self.parse_params(params)?)
            }
            QueryMethodId::FoldingRanges => QueryRequest::FoldingRanges(self.parse_params(params)?),
            QueryMethodId::SemanticTokens => {
                QueryRequest::SemanticTokens(self.parse_params(params)?)
            }
            QueryMethodId::SemanticTokensRange => {
                QueryRequest::SemanticTokensRange(self.parse_params(params)?)
            }
            QueryMethodId::Outline => QueryRequest::Outline(self.parse_params(params)?),
            QueryMethodId::SymbolSearch => QueryRequest::SymbolSearch(self.parse_params(params)?),
            QueryMethodId::Links => QueryRequest::Links(self.parse_params(params)?),
            QueryMethodId::Highlight => QueryRequest::Highlight(self.parse_params(params)?),
            QueryMethodId::SelectionRanges => {
                QueryRequest::SelectionRanges(self.parse_params(params)?)
            }
            QueryMethodId::GotoDefinition => {
                QueryRequest::GotoDefinition(self.parse_params(params)?)
            }
            QueryMethodId::GotoDeclaration => {
                QueryRequest::GotoDeclaration(self.parse_params(params)?)
            }
            QueryMethodId::GotoTypeDefinition => {
                QueryRequest::GotoTypeDefinition(self.parse_params(params)?)
            }
            QueryMethodId::GotoImplementation => {
                QueryRequest::GotoImplementation(self.parse_params(params)?)
            }
            QueryMethodId::FindReferences => {
                QueryRequest::FindReferences(self.parse_params(params)?)
            }
            QueryMethodId::CallItem => QueryRequest::CallItem(self.parse_params(params)?),
            QueryMethodId::IncomingCalls => QueryRequest::IncomingCalls(self.parse_params(params)?),
            QueryMethodId::OutgoingCalls => QueryRequest::OutgoingCalls(self.parse_params(params)?),
            QueryMethodId::TypeItem => QueryRequest::TypeItem(self.parse_params(params)?),
            QueryMethodId::Supertypes => QueryRequest::Supertypes(self.parse_params(params)?),
            QueryMethodId::Subtypes => QueryRequest::Subtypes(self.parse_params(params)?),
            QueryMethodId::Decorators => QueryRequest::Decorators(self.parse_params(params)?),
            QueryMethodId::RenameTarget => QueryRequest::RenameTarget(self.parse_params(params)?),
            QueryMethodId::Rename => QueryRequest::Rename(self.parse_params(params)?),
            QueryMethodId::RenameFiles => QueryRequest::RenameFiles(self.parse_params(params)?),
            QueryMethodId::ExtractFunction => {
                QueryRequest::ExtractFunction(self.parse_params(params)?)
            }
            QueryMethodId::ExtractVariable => {
                QueryRequest::ExtractVariable(self.parse_params(params)?)
            }
            QueryMethodId::Inline => QueryRequest::Inline(self.parse_params(params)?),
            QueryMethodId::ChangeSignature => {
                QueryRequest::ChangeSignature(self.parse_params(params)?)
            }
            QueryMethodId::CodeActions => QueryRequest::CodeActions(self.parse_params(params)?),
        };

        // keep method id mappings synchronized across query surfaces
        debug_assert_eq!(request.method_id(), self.id);

        Ok(request)
    }

    /// Parse typed params for this method.
    fn parse_params<T: DeserializeOwned>(
        &self,
        params: Value,
    ) -> Result<T, QueryRequestParseError> {
        // preserve method metadata on decode errors
        serde_json::from_value(params).map_err(|source| QueryRequestParseError::InvalidParams {
            method: self.name,
            params_type: self.params_type,
            source,
        })
    }
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
        category: QueryCategory::Editor,
        summary: "list completion items at a position",
        params_type: "CompletionRequest",
        result_type: "CompletionResponse",
    },
    QueryMethod {
        id: QueryMethodId::Hover,
        name: "hover",
        category: QueryCategory::Editor,
        summary: "show hover information at a position",
        params_type: "HoverRequest",
        result_type: "HoverResponse",
    },
    QueryMethod {
        id: QueryMethodId::SignatureHelp,
        name: "signature_help",
        category: QueryCategory::Editor,
        summary: "show signature help at a position",
        params_type: "SignatureHelpRequest",
        result_type: "SignatureHelpResponse",
    },
    QueryMethod {
        id: QueryMethodId::InlayHints,
        name: "inlay_hints",
        category: QueryCategory::Editor,
        summary: "list inlay hints for a range",
        params_type: "InlayHintsRequest",
        result_type: "InlayHintsResponse",
    },
    QueryMethod {
        id: QueryMethodId::CodeLenses,
        name: "code_lenses",
        category: QueryCategory::Editor,
        summary: "list code lenses for a module",
        params_type: "CodeLensesRequest",
        result_type: "CodeLensesResponse",
    },
    QueryMethod {
        id: QueryMethodId::ResolveCodeLens,
        name: "resolve_code_lens",
        category: QueryCategory::Editor,
        summary: "resolve a code lens",
        params_type: "ResolveCodeLensRequest",
        result_type: "ResolveCodeLensResponse",
    },
    QueryMethod {
        id: QueryMethodId::FoldingRanges,
        name: "folding_ranges",
        category: QueryCategory::Editor,
        summary: "list folding ranges for a module",
        params_type: "FoldingRangesRequest",
        result_type: "FoldingRangesResponse",
    },
    QueryMethod {
        id: QueryMethodId::SemanticTokens,
        name: "semantic_tokens",
        category: QueryCategory::Editor,
        summary: "list semantic tokens for a module",
        params_type: "SemanticTokensRequest",
        result_type: "SemanticTokensResponse",
    },
    QueryMethod {
        id: QueryMethodId::SemanticTokensRange,
        name: "semantic_tokens_range",
        category: QueryCategory::Editor,
        summary: "list semantic tokens for a range",
        params_type: "SemanticTokensRangeRequest",
        result_type: "SemanticTokensResponse",
    },
    QueryMethod {
        id: QueryMethodId::Outline,
        name: "outline",
        category: QueryCategory::Symbol,
        summary: "list module outline symbols",
        params_type: "OutlineRequest",
        result_type: "OutlineResponse",
    },
    QueryMethod {
        id: QueryMethodId::SymbolSearch,
        name: "search_symbols",
        category: QueryCategory::Symbol,
        summary: "search symbols",
        params_type: "SymbolSearchRequest",
        result_type: "SymbolSearchResponse",
    },
    QueryMethod {
        id: QueryMethodId::Links,
        name: "links",
        category: QueryCategory::Navigation,
        summary: "list links for a module",
        params_type: "LinksRequest",
        result_type: "LinksResponse",
    },
    QueryMethod {
        id: QueryMethodId::Highlight,
        name: "highlight",
        category: QueryCategory::Navigation,
        summary: "list highlights at a position",
        params_type: "HighlightRequest",
        result_type: "HighlightResponse",
    },
    QueryMethod {
        id: QueryMethodId::SelectionRanges,
        name: "selection_ranges",
        category: QueryCategory::Navigation,
        summary: "list selection ranges for positions",
        params_type: "SelectionRangesRequest",
        result_type: "SelectionRangesResponse",
    },
    QueryMethod {
        id: QueryMethodId::GotoDefinition,
        name: "definition",
        category: QueryCategory::Navigation,
        summary: "find definition locations",
        params_type: "GotoDefinitionRequest",
        result_type: "GotoDefinitionResponse",
    },
    QueryMethod {
        id: QueryMethodId::GotoDeclaration,
        name: "declaration",
        category: QueryCategory::Navigation,
        summary: "find declaration locations",
        params_type: "GotoDeclarationRequest",
        result_type: "GotoDeclarationResponse",
    },
    QueryMethod {
        id: QueryMethodId::GotoTypeDefinition,
        name: "type_definition",
        category: QueryCategory::Navigation,
        summary: "find type definition locations",
        params_type: "GotoTypeDefinitionRequest",
        result_type: "GotoTypeDefinitionResponse",
    },
    QueryMethod {
        id: QueryMethodId::GotoImplementation,
        name: "implementation",
        category: QueryCategory::Navigation,
        summary: "find implementation locations",
        params_type: "GotoImplementationRequest",
        result_type: "GotoImplementationResponse",
    },
    QueryMethod {
        id: QueryMethodId::FindReferences,
        name: "references",
        category: QueryCategory::Navigation,
        summary: "find symbol references",
        params_type: "FindReferencesRequest",
        result_type: "FindReferencesResponse",
    },
    QueryMethod {
        id: QueryMethodId::CallItem,
        name: "call_item",
        category: QueryCategory::Hierarchy,
        summary: "return the call item at a position",
        params_type: "CallItemRequest",
        result_type: "CallItemResponse",
    },
    QueryMethod {
        id: QueryMethodId::IncomingCalls,
        name: "incoming_calls",
        category: QueryCategory::Hierarchy,
        summary: "list incoming calls",
        params_type: "IncomingCallsRequest",
        result_type: "IncomingCallsResponse",
    },
    QueryMethod {
        id: QueryMethodId::OutgoingCalls,
        name: "outgoing_calls",
        category: QueryCategory::Hierarchy,
        summary: "list outgoing calls",
        params_type: "OutgoingCallsRequest",
        result_type: "OutgoingCallsResponse",
    },
    QueryMethod {
        id: QueryMethodId::TypeItem,
        name: "type_item",
        category: QueryCategory::Hierarchy,
        summary: "return the type item at a position",
        params_type: "TypeItemRequest",
        result_type: "TypeItemResponse",
    },
    QueryMethod {
        id: QueryMethodId::Supertypes,
        name: "supertypes",
        category: QueryCategory::Hierarchy,
        summary: "list supertypes",
        params_type: "SupertypesRequest",
        result_type: "SupertypesResponse",
    },
    QueryMethod {
        id: QueryMethodId::Subtypes,
        name: "subtypes",
        category: QueryCategory::Hierarchy,
        summary: "list subtypes",
        params_type: "SubtypesRequest",
        result_type: "SubtypesResponse",
    },
    QueryMethod {
        id: QueryMethodId::Decorators,
        name: "decorators",
        category: QueryCategory::Navigation,
        summary: "list decorators",
        params_type: "DecoratorsRequest",
        result_type: "DecoratorsResponse",
    },
    QueryMethod {
        id: QueryMethodId::RenameTarget,
        name: "rename_target",
        category: QueryCategory::Edit,
        summary: "return the rename target at a position",
        params_type: "RenameTargetRequest",
        result_type: "RenameTargetResponse",
    },
    QueryMethod {
        id: QueryMethodId::Rename,
        name: "rename",
        category: QueryCategory::Edit,
        summary: "rename a symbol",
        params_type: "RenameRequest",
        result_type: "RenameResponse",
    },
    QueryMethod {
        id: QueryMethodId::RenameFiles,
        name: "rename_files",
        category: QueryCategory::Edit,
        summary: "rename file imports across indexed modules",
        params_type: "RenameFilesRequest",
        result_type: "RenameFilesResponse",
    },
    QueryMethod {
        id: QueryMethodId::ExtractFunction,
        name: "extract_function",
        category: QueryCategory::Edit,
        summary: "extract a selection into a new function",
        params_type: "ExtractFunctionRequest",
        result_type: "ExtractFunctionResponse",
    },
    QueryMethod {
        id: QueryMethodId::ExtractVariable,
        name: "extract_variable",
        category: QueryCategory::Edit,
        summary: "extract a selection into a const binding",
        params_type: "ExtractVariableRequest",
        result_type: "ExtractVariableResponse",
    },
    QueryMethod {
        id: QueryMethodId::Inline,
        name: "inline",
        category: QueryCategory::Edit,
        summary: "inline a symbol at a position",
        params_type: "InlineRequest",
        result_type: "InlineResponse",
    },
    QueryMethod {
        id: QueryMethodId::ChangeSignature,
        name: "change_signature",
        category: QueryCategory::Edit,
        summary: "change a function signature and update call sites",
        params_type: "ChangeSignatureRequest",
        result_type: "ChangeSignatureResponse",
    },
    QueryMethod {
        id: QueryMethodId::CodeActions,
        name: "code_actions",
        category: QueryCategory::Edit,
        summary: "list code actions for a range",
        params_type: "CodeActionsRequest",
        result_type: "CodeActionsResponse",
    },
];

/// Resolve a query method by canonical name.
pub fn query_method(name: &str) -> Option<&'static QueryMethod> {
    QUERY_METHODS
        .iter()
        .find(|method| method.matches_name(name))
}

/// Return the supported query methods.
pub fn query_methods() -> &'static [QueryMethod] {
    QUERY_METHODS
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

    method.parse_request(params)
}
