use serde::{Deserialize, Serialize};

use super::{assist, navigation, refactor};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryRequestEnvelope {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_id: Option<String>,
    #[serde(flatten)]
    pub request: QueryRequest,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryResponseEnvelope {
    pub snapshot_id: String,
    #[serde(flatten)]
    pub response: QueryResponse,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "params", rename_all = "snake_case")]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "result", rename_all = "snake_case")]
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
