use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::query::{assist, navigation, refactor};

#[derive(Debug, Clone, PartialEq)]
pub struct QueryRequestEnvelope {
    pub snapshot_id: Option<String>,
    pub request: QueryRequest,
}

#[derive(Debug, Clone, PartialEq)]
pub struct QueryResponseEnvelope {
    pub snapshot_id: String,
    pub response: QueryResponse,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct QueryRequestEnvelopeHuman {
    #[serde(skip_serializing_if = "Option::is_none")]
    snapshot_id: Option<String>,
    #[serde(flatten)]
    request: QueryRequest,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct QueryRequestEnvelopeBinary {
    snapshot_id: Option<String>,
    request: QueryRequest,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct QueryResponseEnvelopeHuman {
    snapshot_id: String,
    #[serde(flatten)]
    response: QueryResponse,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct QueryResponseEnvelopeBinary {
    snapshot_id: String,
    response: QueryResponse,
}

impl Serialize for QueryRequestEnvelope {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // serialize as tagged payloads for human readable formats
        if serializer.is_human_readable() {
            let human = QueryRequestEnvelopeHuman {
                snapshot_id: self.snapshot_id.clone(),
                request: self.request.clone(),
            };

            return human.serialize(serializer);
        }

        // serialize as compact payloads for binary formats
        let binary = QueryRequestEnvelopeBinary {
            snapshot_id: self.snapshot_id.clone(),
            request: self.request.clone(),
        };

        binary.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for QueryRequestEnvelope {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // deserialize tagged payloads for human readable formats
        if deserializer.is_human_readable() {
            let human = QueryRequestEnvelopeHuman::deserialize(deserializer)?;

            return Ok(Self {
                snapshot_id: human.snapshot_id,
                request: human.request,
            });
        }

        // deserialize compact payloads for binary formats
        let binary = QueryRequestEnvelopeBinary::deserialize(deserializer)?;

        Ok(Self {
            snapshot_id: binary.snapshot_id,
            request: binary.request,
        })
    }
}

impl Serialize for QueryResponseEnvelope {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // serialize as tagged payloads for human readable formats
        if serializer.is_human_readable() {
            let human = QueryResponseEnvelopeHuman {
                snapshot_id: self.snapshot_id.clone(),
                response: self.response.clone(),
            };

            return human.serialize(serializer);
        }

        // serialize as compact payloads for binary formats
        let binary = QueryResponseEnvelopeBinary {
            snapshot_id: self.snapshot_id.clone(),
            response: self.response.clone(),
        };

        binary.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for QueryResponseEnvelope {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // deserialize tagged payloads for human readable formats
        if deserializer.is_human_readable() {
            let human = QueryResponseEnvelopeHuman::deserialize(deserializer)?;

            return Ok(Self {
                snapshot_id: human.snapshot_id,
                response: human.response,
            });
        }

        // deserialize compact payloads for binary formats
        let binary = QueryResponseEnvelopeBinary::deserialize(deserializer)?;

        Ok(Self {
            snapshot_id: binary.snapshot_id,
            response: binary.response,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum QueryRequest {
    Completion(assist::CompletionRequest),
    Hover(assist::HoverRequest),
    SignatureHelp(assist::SignatureHelpRequest),
    InlayHints(assist::InlayHintsRequest),
    CodeLenses(assist::CodeLensesRequest),
    ResolveCodeLens(assist::ResolveCodeLensRequest),
    FoldingRanges(assist::FoldingRangesRequest),
    SemanticTokens(assist::SemanticTokensRequest),
    SemanticTokensRange(assist::SemanticTokensRangeRequest),
    DocumentSymbols(navigation::DocumentSymbolsRequest),
    WorkspaceSymbols(navigation::WorkspaceSymbolsRequest),
    DocumentLinks(navigation::DocumentLinksRequest),
    ResolveDocumentLink(navigation::ResolveDocumentLinkRequest),
    DocumentHighlight(navigation::DocumentHighlightRequest),
    SelectionRanges(navigation::SelectionRangesRequest),
    GotoDefinition(navigation::GotoDefinitionRequest),
    GotoDeclaration(navigation::GotoDeclarationRequest),
    GotoTypeDefinition(navigation::GotoTypeDefinitionRequest),
    GotoImplementation(navigation::GotoImplementationRequest),
    FindReferences(navigation::FindReferencesRequest),
    PrepareCallHierarchy(navigation::PrepareCallHierarchyRequest),
    CallHierarchyIncoming(navigation::CallHierarchyIncomingRequest),
    CallHierarchyOutgoing(navigation::CallHierarchyOutgoingRequest),
    PrepareTypeHierarchy(navigation::PrepareTypeHierarchyRequest),
    TypeHierarchySupertypes(navigation::TypeHierarchySupertypesRequest),
    TypeHierarchySubtypes(navigation::TypeHierarchySubtypesRequest),
    PrepareRename(refactor::PrepareRenameRequest),
    Rename(refactor::RenameRequest),
    RenameFiles(refactor::RenameFilesRequest),
    ExtractFunction(refactor::ExtractFunctionRequest),
    ExtractVariable(refactor::ExtractVariableRequest),
    Inline(refactor::InlineRequest),
    ChangeSignature(refactor::ChangeSignatureRequest),
    CodeActions(refactor::CodeActionsRequest),
}

#[derive(Debug, Clone, PartialEq)]
pub enum QueryResponse {
    Completion(assist::CompletionResponse),
    Hover(assist::HoverResponse),
    SignatureHelp(assist::SignatureHelpResponse),
    InlayHints(assist::InlayHintsResponse),
    CodeLenses(assist::CodeLensesResponse),
    ResolveCodeLens(assist::ResolveCodeLensResponse),
    FoldingRanges(assist::FoldingRangesResponse),
    SemanticTokens(assist::SemanticTokensResponse),
    SemanticTokensRange(assist::SemanticTokensResponse),
    DocumentSymbols(navigation::DocumentSymbolsResponse),
    WorkspaceSymbols(navigation::WorkspaceSymbolsResponse),
    DocumentLinks(navigation::DocumentLinksResponse),
    ResolveDocumentLink(navigation::ResolveDocumentLinkResponse),
    DocumentHighlight(navigation::DocumentHighlightResponse),
    SelectionRanges(navigation::SelectionRangesResponse),
    GotoDefinition(navigation::GotoDefinitionResponse),
    GotoDeclaration(navigation::GotoDeclarationResponse),
    GotoTypeDefinition(navigation::GotoTypeDefinitionResponse),
    GotoImplementation(navigation::GotoImplementationResponse),
    FindReferences(navigation::FindReferencesResponse),
    PrepareCallHierarchy(navigation::PrepareCallHierarchyResponse),
    CallHierarchyIncoming(navigation::CallHierarchyIncomingResponse),
    CallHierarchyOutgoing(navigation::CallHierarchyOutgoingResponse),
    PrepareTypeHierarchy(navigation::PrepareTypeHierarchyResponse),
    TypeHierarchySupertypes(navigation::TypeHierarchySupertypesResponse),
    TypeHierarchySubtypes(navigation::TypeHierarchySubtypesResponse),
    PrepareRename(refactor::PrepareRenameResponse),
    Rename(refactor::RenameResponse),
    RenameFiles(refactor::RenameFilesResponse),
    ExtractFunction(refactor::ExtractFunctionResponse),
    ExtractVariable(refactor::ExtractVariableResponse),
    Inline(refactor::InlineResponse),
    ChangeSignature(refactor::ChangeSignatureResponse),
    CodeActions(refactor::CodeActionsResponse),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "params", rename_all = "snake_case")]
enum QueryRequestHuman {
    Completion(assist::CompletionRequest),
    Hover(assist::HoverRequest),
    SignatureHelp(assist::SignatureHelpRequest),
    InlayHints(assist::InlayHintsRequest),
    CodeLenses(assist::CodeLensesRequest),
    ResolveCodeLens(assist::ResolveCodeLensRequest),
    FoldingRanges(assist::FoldingRangesRequest),
    SemanticTokens(assist::SemanticTokensRequest),
    SemanticTokensRange(assist::SemanticTokensRangeRequest),
    DocumentSymbols(navigation::DocumentSymbolsRequest),
    WorkspaceSymbols(navigation::WorkspaceSymbolsRequest),
    DocumentLinks(navigation::DocumentLinksRequest),
    ResolveDocumentLink(navigation::ResolveDocumentLinkRequest),
    DocumentHighlight(navigation::DocumentHighlightRequest),
    SelectionRanges(navigation::SelectionRangesRequest),
    GotoDefinition(navigation::GotoDefinitionRequest),
    GotoDeclaration(navigation::GotoDeclarationRequest),
    GotoTypeDefinition(navigation::GotoTypeDefinitionRequest),
    GotoImplementation(navigation::GotoImplementationRequest),
    FindReferences(navigation::FindReferencesRequest),
    PrepareCallHierarchy(navigation::PrepareCallHierarchyRequest),
    CallHierarchyIncoming(navigation::CallHierarchyIncomingRequest),
    CallHierarchyOutgoing(navigation::CallHierarchyOutgoingRequest),
    PrepareTypeHierarchy(navigation::PrepareTypeHierarchyRequest),
    TypeHierarchySupertypes(navigation::TypeHierarchySupertypesRequest),
    TypeHierarchySubtypes(navigation::TypeHierarchySubtypesRequest),
    PrepareRename(refactor::PrepareRenameRequest),
    Rename(refactor::RenameRequest),
    RenameFiles(refactor::RenameFilesRequest),
    ExtractFunction(refactor::ExtractFunctionRequest),
    ExtractVariable(refactor::ExtractVariableRequest),
    Inline(refactor::InlineRequest),
    ChangeSignature(refactor::ChangeSignatureRequest),
    CodeActions(refactor::CodeActionsRequest),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
enum QueryRequestBinary {
    Completion(assist::CompletionRequest),
    Hover(assist::HoverRequest),
    SignatureHelp(assist::SignatureHelpRequest),
    InlayHints(assist::InlayHintsRequest),
    CodeLenses(assist::CodeLensesRequest),
    ResolveCodeLens(assist::ResolveCodeLensRequest),
    FoldingRanges(assist::FoldingRangesRequest),
    SemanticTokens(assist::SemanticTokensRequest),
    SemanticTokensRange(assist::SemanticTokensRangeRequest),
    DocumentSymbols(navigation::DocumentSymbolsRequest),
    WorkspaceSymbols(navigation::WorkspaceSymbolsRequest),
    DocumentLinks(navigation::DocumentLinksRequest),
    ResolveDocumentLink(navigation::ResolveDocumentLinkRequest),
    DocumentHighlight(navigation::DocumentHighlightRequest),
    SelectionRanges(navigation::SelectionRangesRequest),
    GotoDefinition(navigation::GotoDefinitionRequest),
    GotoDeclaration(navigation::GotoDeclarationRequest),
    GotoTypeDefinition(navigation::GotoTypeDefinitionRequest),
    GotoImplementation(navigation::GotoImplementationRequest),
    FindReferences(navigation::FindReferencesRequest),
    PrepareCallHierarchy(navigation::PrepareCallHierarchyRequest),
    CallHierarchyIncoming(navigation::CallHierarchyIncomingRequest),
    CallHierarchyOutgoing(navigation::CallHierarchyOutgoingRequest),
    PrepareTypeHierarchy(navigation::PrepareTypeHierarchyRequest),
    TypeHierarchySupertypes(navigation::TypeHierarchySupertypesRequest),
    TypeHierarchySubtypes(navigation::TypeHierarchySubtypesRequest),
    PrepareRename(refactor::PrepareRenameRequest),
    Rename(refactor::RenameRequest),
    RenameFiles(refactor::RenameFilesRequest),
    ExtractFunction(refactor::ExtractFunctionRequest),
    ExtractVariable(refactor::ExtractVariableRequest),
    Inline(refactor::InlineRequest),
    ChangeSignature(refactor::ChangeSignatureRequest),
    CodeActions(refactor::CodeActionsRequest),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "result", rename_all = "snake_case")]
enum QueryResponseHuman {
    Completion(assist::CompletionResponse),
    Hover(assist::HoverResponse),
    SignatureHelp(assist::SignatureHelpResponse),
    InlayHints(assist::InlayHintsResponse),
    CodeLenses(assist::CodeLensesResponse),
    ResolveCodeLens(assist::ResolveCodeLensResponse),
    FoldingRanges(assist::FoldingRangesResponse),
    SemanticTokens(assist::SemanticTokensResponse),
    SemanticTokensRange(assist::SemanticTokensResponse),
    DocumentSymbols(navigation::DocumentSymbolsResponse),
    WorkspaceSymbols(navigation::WorkspaceSymbolsResponse),
    DocumentLinks(navigation::DocumentLinksResponse),
    ResolveDocumentLink(navigation::ResolveDocumentLinkResponse),
    DocumentHighlight(navigation::DocumentHighlightResponse),
    SelectionRanges(navigation::SelectionRangesResponse),
    GotoDefinition(navigation::GotoDefinitionResponse),
    GotoDeclaration(navigation::GotoDeclarationResponse),
    GotoTypeDefinition(navigation::GotoTypeDefinitionResponse),
    GotoImplementation(navigation::GotoImplementationResponse),
    FindReferences(navigation::FindReferencesResponse),
    PrepareCallHierarchy(navigation::PrepareCallHierarchyResponse),
    CallHierarchyIncoming(navigation::CallHierarchyIncomingResponse),
    CallHierarchyOutgoing(navigation::CallHierarchyOutgoingResponse),
    PrepareTypeHierarchy(navigation::PrepareTypeHierarchyResponse),
    TypeHierarchySupertypes(navigation::TypeHierarchySupertypesResponse),
    TypeHierarchySubtypes(navigation::TypeHierarchySubtypesResponse),
    PrepareRename(refactor::PrepareRenameResponse),
    Rename(refactor::RenameResponse),
    RenameFiles(refactor::RenameFilesResponse),
    ExtractFunction(refactor::ExtractFunctionResponse),
    ExtractVariable(refactor::ExtractVariableResponse),
    Inline(refactor::InlineResponse),
    ChangeSignature(refactor::ChangeSignatureResponse),
    CodeActions(refactor::CodeActionsResponse),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
enum QueryResponseBinary {
    Completion(assist::CompletionResponse),
    Hover(assist::HoverResponse),
    SignatureHelp(assist::SignatureHelpResponse),
    InlayHints(assist::InlayHintsResponse),
    CodeLenses(assist::CodeLensesResponse),
    ResolveCodeLens(assist::ResolveCodeLensResponse),
    FoldingRanges(assist::FoldingRangesResponse),
    SemanticTokens(assist::SemanticTokensResponse),
    SemanticTokensRange(assist::SemanticTokensResponse),
    DocumentSymbols(navigation::DocumentSymbolsResponse),
    WorkspaceSymbols(navigation::WorkspaceSymbolsResponse),
    DocumentLinks(navigation::DocumentLinksResponse),
    ResolveDocumentLink(navigation::ResolveDocumentLinkResponse),
    DocumentHighlight(navigation::DocumentHighlightResponse),
    SelectionRanges(navigation::SelectionRangesResponse),
    GotoDefinition(navigation::GotoDefinitionResponse),
    GotoDeclaration(navigation::GotoDeclarationResponse),
    GotoTypeDefinition(navigation::GotoTypeDefinitionResponse),
    GotoImplementation(navigation::GotoImplementationResponse),
    FindReferences(navigation::FindReferencesResponse),
    PrepareCallHierarchy(navigation::PrepareCallHierarchyResponse),
    CallHierarchyIncoming(navigation::CallHierarchyIncomingResponse),
    CallHierarchyOutgoing(navigation::CallHierarchyOutgoingResponse),
    PrepareTypeHierarchy(navigation::PrepareTypeHierarchyResponse),
    TypeHierarchySupertypes(navigation::TypeHierarchySupertypesResponse),
    TypeHierarchySubtypes(navigation::TypeHierarchySubtypesResponse),
    PrepareRename(refactor::PrepareRenameResponse),
    Rename(refactor::RenameResponse),
    RenameFiles(refactor::RenameFilesResponse),
    ExtractFunction(refactor::ExtractFunctionResponse),
    ExtractVariable(refactor::ExtractVariableResponse),
    Inline(refactor::InlineResponse),
    ChangeSignature(refactor::ChangeSignatureResponse),
    CodeActions(refactor::CodeActionsResponse),
}

// map request variants across wire enums
macro_rules! map_query_request {
    ($value:expr, $from:ident => $to:ident) => {
        match $value {
            $from::Completion(params) => $to::Completion(params),
            $from::Hover(params) => $to::Hover(params),
            $from::SignatureHelp(params) => $to::SignatureHelp(params),
            $from::InlayHints(params) => $to::InlayHints(params),
            $from::CodeLenses(params) => $to::CodeLenses(params),
            $from::ResolveCodeLens(params) => $to::ResolveCodeLens(params),
            $from::FoldingRanges(params) => $to::FoldingRanges(params),
            $from::SemanticTokens(params) => $to::SemanticTokens(params),
            $from::SemanticTokensRange(params) => $to::SemanticTokensRange(params),
            $from::DocumentSymbols(params) => $to::DocumentSymbols(params),
            $from::WorkspaceSymbols(params) => $to::WorkspaceSymbols(params),
            $from::DocumentLinks(params) => $to::DocumentLinks(params),
            $from::ResolveDocumentLink(params) => $to::ResolveDocumentLink(params),
            $from::DocumentHighlight(params) => $to::DocumentHighlight(params),
            $from::SelectionRanges(params) => $to::SelectionRanges(params),
            $from::GotoDefinition(params) => $to::GotoDefinition(params),
            $from::GotoDeclaration(params) => $to::GotoDeclaration(params),
            $from::GotoTypeDefinition(params) => $to::GotoTypeDefinition(params),
            $from::GotoImplementation(params) => $to::GotoImplementation(params),
            $from::FindReferences(params) => $to::FindReferences(params),
            $from::PrepareCallHierarchy(params) => $to::PrepareCallHierarchy(params),
            $from::CallHierarchyIncoming(params) => $to::CallHierarchyIncoming(params),
            $from::CallHierarchyOutgoing(params) => $to::CallHierarchyOutgoing(params),
            $from::PrepareTypeHierarchy(params) => $to::PrepareTypeHierarchy(params),
            $from::TypeHierarchySupertypes(params) => $to::TypeHierarchySupertypes(params),
            $from::TypeHierarchySubtypes(params) => $to::TypeHierarchySubtypes(params),
            $from::PrepareRename(params) => $to::PrepareRename(params),
            $from::Rename(params) => $to::Rename(params),
            $from::RenameFiles(params) => $to::RenameFiles(params),
            $from::ExtractFunction(params) => $to::ExtractFunction(params),
            $from::ExtractVariable(params) => $to::ExtractVariable(params),
            $from::Inline(params) => $to::Inline(params),
            $from::ChangeSignature(params) => $to::ChangeSignature(params),
            $from::CodeActions(params) => $to::CodeActions(params),
        }
    };
}

// map response variants across wire enums
macro_rules! map_query_response {
    ($value:expr, $from:ident => $to:ident) => {
        match $value {
            $from::Completion(result) => $to::Completion(result),
            $from::Hover(result) => $to::Hover(result),
            $from::SignatureHelp(result) => $to::SignatureHelp(result),
            $from::InlayHints(result) => $to::InlayHints(result),
            $from::CodeLenses(result) => $to::CodeLenses(result),
            $from::ResolveCodeLens(result) => $to::ResolveCodeLens(result),
            $from::FoldingRanges(result) => $to::FoldingRanges(result),
            $from::SemanticTokens(result) => $to::SemanticTokens(result),
            $from::SemanticTokensRange(result) => $to::SemanticTokensRange(result),
            $from::DocumentSymbols(result) => $to::DocumentSymbols(result),
            $from::WorkspaceSymbols(result) => $to::WorkspaceSymbols(result),
            $from::DocumentLinks(result) => $to::DocumentLinks(result),
            $from::ResolveDocumentLink(result) => $to::ResolveDocumentLink(result),
            $from::DocumentHighlight(result) => $to::DocumentHighlight(result),
            $from::SelectionRanges(result) => $to::SelectionRanges(result),
            $from::GotoDefinition(result) => $to::GotoDefinition(result),
            $from::GotoDeclaration(result) => $to::GotoDeclaration(result),
            $from::GotoTypeDefinition(result) => $to::GotoTypeDefinition(result),
            $from::GotoImplementation(result) => $to::GotoImplementation(result),
            $from::FindReferences(result) => $to::FindReferences(result),
            $from::PrepareCallHierarchy(result) => $to::PrepareCallHierarchy(result),
            $from::CallHierarchyIncoming(result) => $to::CallHierarchyIncoming(result),
            $from::CallHierarchyOutgoing(result) => $to::CallHierarchyOutgoing(result),
            $from::PrepareTypeHierarchy(result) => $to::PrepareTypeHierarchy(result),
            $from::TypeHierarchySupertypes(result) => $to::TypeHierarchySupertypes(result),
            $from::TypeHierarchySubtypes(result) => $to::TypeHierarchySubtypes(result),
            $from::PrepareRename(result) => $to::PrepareRename(result),
            $from::Rename(result) => $to::Rename(result),
            $from::RenameFiles(result) => $to::RenameFiles(result),
            $from::ExtractFunction(result) => $to::ExtractFunction(result),
            $from::ExtractVariable(result) => $to::ExtractVariable(result),
            $from::Inline(result) => $to::Inline(result),
            $from::ChangeSignature(result) => $to::ChangeSignature(result),
            $from::CodeActions(result) => $to::CodeActions(result),
        }
    };
}

impl From<QueryRequest> for QueryRequestHuman {
    fn from(request: QueryRequest) -> Self {
        // map request variants into the human readable wire enum

        map_query_request!(request, QueryRequest => QueryRequestHuman)
    }
}

impl From<QueryRequestHuman> for QueryRequest {
    fn from(request: QueryRequestHuman) -> Self {
        // map request variants into the core enum

        map_query_request!(request, QueryRequestHuman => QueryRequest)
    }
}

impl From<QueryRequest> for QueryRequestBinary {
    fn from(request: QueryRequest) -> Self {
        // map request variants into the binary wire enum

        map_query_request!(request, QueryRequest => QueryRequestBinary)
    }
}

impl From<QueryRequestBinary> for QueryRequest {
    fn from(request: QueryRequestBinary) -> Self {
        // map request variants into the core enum

        map_query_request!(request, QueryRequestBinary => QueryRequest)
    }
}

impl From<QueryResponse> for QueryResponseHuman {
    fn from(response: QueryResponse) -> Self {
        // map response variants into the human readable wire enum

        map_query_response!(response, QueryResponse => QueryResponseHuman)
    }
}

impl From<QueryResponseHuman> for QueryResponse {
    fn from(response: QueryResponseHuman) -> Self {
        // map response variants into the core enum

        map_query_response!(response, QueryResponseHuman => QueryResponse)
    }
}

impl From<QueryResponse> for QueryResponseBinary {
    fn from(response: QueryResponse) -> Self {
        // map response variants into the binary wire enum

        map_query_response!(response, QueryResponse => QueryResponseBinary)
    }
}

impl From<QueryResponseBinary> for QueryResponse {
    fn from(response: QueryResponseBinary) -> Self {
        // map response variants into the core enum

        map_query_response!(response, QueryResponseBinary => QueryResponse)
    }
}

impl Serialize for QueryRequest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // serialize as tagged payloads for human readable formats
        if serializer.is_human_readable() {
            let human = QueryRequestHuman::from(self.clone());

            return human.serialize(serializer);
        }

        // serialize as compact payloads for binary formats
        let binary = QueryRequestBinary::from(self.clone());

        binary.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for QueryRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // deserialize tagged payloads for human readable formats
        if deserializer.is_human_readable() {
            let human = QueryRequestHuman::deserialize(deserializer)?;

            return Ok(Self::from(human));
        }

        // deserialize compact payloads for binary formats
        let binary = QueryRequestBinary::deserialize(deserializer)?;

        Ok(Self::from(binary))
    }
}

impl Serialize for QueryResponse {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // serialize as tagged payloads for human readable formats
        if serializer.is_human_readable() {
            let human = QueryResponseHuman::from(self.clone());

            return human.serialize(serializer);
        }

        // serialize as compact payloads for binary formats
        let binary = QueryResponseBinary::from(self.clone());

        binary.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for QueryResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // deserialize tagged payloads for human readable formats
        if deserializer.is_human_readable() {
            let human = QueryResponseHuman::deserialize(deserializer)?;

            return Ok(Self::from(human));
        }

        // deserialize compact payloads for binary formats
        let binary = QueryResponseBinary::deserialize(deserializer)?;

        Ok(Self::from(binary))
    }
}
