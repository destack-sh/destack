use std::path::PathBuf;

use destack_artifact::ArtifactKey;
use destack_source::{ModuleId, ProfileId, Uri};
use destack_workspace::Revision;
use serde::{Deserialize, Serialize};

use super::method::QueryMethodId;
use crate::{assist, navigation, refactor};

/// Wire envelope for a single query request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryRequestEnvelope {
    /// Expected workspace semantic revision for mutating requests.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_revision: Option<Revision>,
    /// Query request payload.
    #[serde(flatten)]
    pub request: QueryRequest,
}

/// Wire envelope for a single query response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryResponseEnvelope {
    /// Workspace semantic revision after request execution.
    pub revision: Revision,
    /// Query response payload.
    #[serde(flatten)]
    pub response: QueryResponse,
}

/// Query execution mode used for precondition handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryExecutionMode {
    /// Read-only query that does not produce edits.
    Read,
    /// Query that produces refactor edits.
    Write,
}

/// Query request payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "params", rename_all = "snake_case")]
pub enum QueryRequest {
    /// Completion request payload.
    Completion(assist::CompletionRequest),
    /// Hover request payload.
    Hover(assist::HoverRequest),
    /// Signature help request payload.
    SignatureHelp(assist::SignatureHelpRequest),
    /// Inlay hints request payload.
    InlayHints(assist::InlayHintsRequest),
    /// Code lenses request payload.
    CodeLenses(assist::CodeLensesRequest),
    /// Code lens resolve request payload.
    ResolveCodeLens(assist::ResolveCodeLensRequest),
    /// Folding ranges request payload.
    FoldingRanges(assist::FoldingRangesRequest),
    /// Full-document semantic tokens request payload.
    SemanticTokens(assist::SemanticTokensRequest),
    /// Range semantic tokens request payload.
    SemanticTokensRange(assist::SemanticTokensRangeRequest),

    /// Document symbols request payload.
    DocumentSymbols(navigation::DocumentSymbolsRequest),
    /// Workspace symbols request payload.
    WorkspaceSymbols(navigation::WorkspaceSymbolsRequest),
    /// Document links request payload.
    DocumentLinks(navigation::DocumentLinksRequest),
    /// Document link resolve request payload.
    ResolveDocumentLink(navigation::ResolveDocumentLinkRequest),
    /// Document highlight request payload.
    DocumentHighlight(navigation::DocumentHighlightRequest),
    /// Selection ranges request payload.
    SelectionRanges(navigation::SelectionRangesRequest),
    /// Goto definition request payload.
    GotoDefinition(navigation::GotoDefinitionRequest),
    /// Goto declaration request payload.
    GotoDeclaration(navigation::GotoDeclarationRequest),
    /// Goto type definition request payload.
    GotoTypeDefinition(navigation::GotoTypeDefinitionRequest),
    /// Goto implementation request payload.
    GotoImplementation(navigation::GotoImplementationRequest),
    /// Find references request payload.
    FindReferences(navigation::FindReferencesRequest),
    /// Call hierarchy prepare request payload.
    PrepareCallHierarchy(navigation::PrepareCallHierarchyRequest),
    /// Incoming call hierarchy request payload.
    CallHierarchyIncoming(navigation::CallHierarchyIncomingRequest),
    /// Outgoing call hierarchy request payload.
    CallHierarchyOutgoing(navigation::CallHierarchyOutgoingRequest),
    /// Type hierarchy prepare request payload.
    PrepareTypeHierarchy(navigation::PrepareTypeHierarchyRequest),
    /// Type hierarchy supertypes request payload.
    TypeHierarchySupertypes(navigation::TypeHierarchySupertypesRequest),
    /// Type hierarchy subtypes request payload.
    TypeHierarchySubtypes(navigation::TypeHierarchySubtypesRequest),

    /// Prepare rename request payload.
    PrepareRename(refactor::PrepareRenameRequest),
    /// Rename request payload.
    Rename(refactor::RenameRequest),
    /// Rename files request payload.
    RenameFiles(refactor::RenameFilesRequest),
    /// Extract function request payload.
    ExtractFunction(refactor::ExtractFunctionRequest),
    /// Extract variable request payload.
    ExtractVariable(refactor::ExtractVariableRequest),
    /// Inline request payload.
    Inline(refactor::InlineRequest),
    /// Change signature request payload.
    ChangeSignature(refactor::ChangeSignatureRequest),
    /// Code actions request payload.
    CodeActions(refactor::CodeActionsRequest),
}

impl QueryRequest {
    /// Return the method identifier for this query.
    pub fn method_id(&self) -> QueryMethodId {
        match self {
            Self::Completion(_) => QueryMethodId::Completion,
            Self::Hover(_) => QueryMethodId::Hover,
            Self::SignatureHelp(_) => QueryMethodId::SignatureHelp,
            Self::InlayHints(_) => QueryMethodId::InlayHints,
            Self::CodeLenses(_) => QueryMethodId::CodeLenses,
            Self::ResolveCodeLens(_) => QueryMethodId::ResolveCodeLens,
            Self::FoldingRanges(_) => QueryMethodId::FoldingRanges,
            Self::SemanticTokens(_) => QueryMethodId::SemanticTokens,
            Self::SemanticTokensRange(_) => QueryMethodId::SemanticTokensRange,
            Self::DocumentSymbols(_) => QueryMethodId::DocumentSymbols,
            Self::WorkspaceSymbols(_) => QueryMethodId::WorkspaceSymbols,
            Self::DocumentLinks(_) => QueryMethodId::DocumentLinks,
            Self::ResolveDocumentLink(_) => QueryMethodId::ResolveDocumentLink,
            Self::DocumentHighlight(_) => QueryMethodId::DocumentHighlight,
            Self::SelectionRanges(_) => QueryMethodId::SelectionRanges,
            Self::GotoDefinition(_) => QueryMethodId::GotoDefinition,
            Self::GotoDeclaration(_) => QueryMethodId::GotoDeclaration,
            Self::GotoTypeDefinition(_) => QueryMethodId::GotoTypeDefinition,
            Self::GotoImplementation(_) => QueryMethodId::GotoImplementation,
            Self::FindReferences(_) => QueryMethodId::FindReferences,
            Self::PrepareCallHierarchy(_) => QueryMethodId::PrepareCallHierarchy,
            Self::CallHierarchyIncoming(_) => QueryMethodId::CallHierarchyIncoming,
            Self::CallHierarchyOutgoing(_) => QueryMethodId::CallHierarchyOutgoing,
            Self::PrepareTypeHierarchy(_) => QueryMethodId::PrepareTypeHierarchy,
            Self::TypeHierarchySupertypes(_) => QueryMethodId::TypeHierarchySupertypes,
            Self::TypeHierarchySubtypes(_) => QueryMethodId::TypeHierarchySubtypes,
            Self::PrepareRename(_) => QueryMethodId::PrepareRename,
            Self::Rename(_) => QueryMethodId::Rename,
            Self::RenameFiles(_) => QueryMethodId::RenameFiles,
            Self::ExtractFunction(_) => QueryMethodId::ExtractFunction,
            Self::ExtractVariable(_) => QueryMethodId::ExtractVariable,
            Self::Inline(_) => QueryMethodId::Inline,
            Self::ChangeSignature(_) => QueryMethodId::ChangeSignature,
            Self::CodeActions(_) => QueryMethodId::CodeActions,
        }
    }

    /// Return the execution mode for this query.
    pub fn execution_mode(&self) -> QueryExecutionMode {
        self.method_id().execution_mode()
    }

    /// Return the targeted document path for this query when it has one.
    pub fn path(&self) -> Option<PathBuf> {
        let uri = match self {
            Self::Completion(params) => &params.uri,
            Self::Hover(params) => &params.uri,
            Self::SignatureHelp(params) => &params.uri,
            Self::InlayHints(params) => &params.uri,
            Self::CodeLenses(params) => &params.uri,
            Self::FoldingRanges(params) => &params.uri,
            Self::SemanticTokens(params) => &params.uri,
            Self::SemanticTokensRange(params) => &params.uri,
            Self::DocumentSymbols(params) => &params.uri,
            Self::DocumentLinks(params) => &params.uri,
            Self::DocumentHighlight(params) => &params.uri,
            Self::SelectionRanges(params) => &params.uri,
            Self::GotoDefinition(params) => &params.uri,
            Self::GotoDeclaration(params) => &params.uri,
            Self::GotoTypeDefinition(params) => &params.uri,
            Self::GotoImplementation(params) => &params.uri,
            Self::FindReferences(params) => &params.uri,
            Self::PrepareCallHierarchy(params) => &params.uri,
            Self::PrepareTypeHierarchy(params) => &params.uri,
            Self::PrepareRename(params) => &params.uri,
            Self::Rename(params) => &params.uri,
            Self::ExtractFunction(params) => &params.uri,
            Self::ExtractVariable(params) => &params.uri,
            Self::Inline(params) => &params.uri,
            Self::ChangeSignature(params) => &params.uri,
            Self::CodeActions(params) => &params.uri,
            Self::ResolveCodeLens(_)
            | Self::WorkspaceSymbols(_)
            | Self::ResolveDocumentLink(_)
            | Self::CallHierarchyIncoming(_)
            | Self::CallHierarchyOutgoing(_)
            | Self::TypeHierarchySupertypes(_)
            | Self::TypeHierarchySubtypes(_)
            | Self::RenameFiles(_) => return None,
        };

        query_path_from_uri(uri)
    }

    /// Return the default prepared artifact root for this query when it needs one.
    pub fn default_artifact_key(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> Option<ArtifactKey> {
        let has_document_root = matches!(
            self,
            Self::Completion(_)
                | Self::Hover(_)
                | Self::SignatureHelp(_)
                | Self::InlayHints(_)
                | Self::CodeLenses(_)
                | Self::FoldingRanges(_)
                | Self::SemanticTokens(_)
                | Self::SemanticTokensRange(_)
                | Self::DocumentSymbols(_)
                | Self::DocumentLinks(_)
                | Self::DocumentHighlight(_)
                | Self::SelectionRanges(_)
                | Self::GotoDefinition(_)
                | Self::GotoDeclaration(_)
                | Self::GotoTypeDefinition(_)
                | Self::GotoImplementation(_)
                | Self::FindReferences(_)
                | Self::PrepareCallHierarchy(_)
                | Self::PrepareTypeHierarchy(_)
                | Self::PrepareRename(_)
                | Self::Rename(_)
                | Self::ExtractFunction(_)
                | Self::ExtractVariable(_)
                | Self::Inline(_)
                | Self::ChangeSignature(_)
                | Self::CodeActions(_)
        );

        if !has_document_root {
            return None;
        }

        Some(default_document_artifact_key(module_id, profile_id))
    }
}

/// Return the default prepared artifact root for one document query.
pub fn default_document_artifact_key(module_id: ModuleId, profile_id: ProfileId) -> ArtifactKey {
    ArtifactKey::dir_analyzed(module_id, profile_id)
}

/// Return a filesystem path for one query uri when it points at a file.
fn query_path_from_uri(uri: &Uri) -> Option<PathBuf> {
    uri.to_path_buf()
}

/// Query response payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "result", rename_all = "snake_case")]
pub enum QueryResponse {
    /// Completion response payload.
    Completion(assist::CompletionResponse),
    /// Hover response payload.
    Hover(assist::HoverResponse),
    /// Signature help response payload.
    SignatureHelp(assist::SignatureHelpResponse),
    /// Inlay hints response payload.
    InlayHints(assist::InlayHintsResponse),
    /// Code lenses response payload.
    CodeLenses(assist::CodeLensesResponse),
    /// Code lens resolve response payload.
    ResolveCodeLens(assist::ResolveCodeLensResponse),
    /// Folding ranges response payload.
    FoldingRanges(assist::FoldingRangesResponse),
    /// Full-document semantic tokens response payload.
    SemanticTokens(assist::SemanticTokensResponse),
    /// Range semantic tokens response payload.
    SemanticTokensRange(assist::SemanticTokensResponse),

    /// Document symbols response payload.
    DocumentSymbols(navigation::DocumentSymbolsResponse),
    /// Workspace symbols response payload.
    WorkspaceSymbols(navigation::WorkspaceSymbolsResponse),
    /// Document links response payload.
    DocumentLinks(navigation::DocumentLinksResponse),
    /// Document link resolve response payload.
    ResolveDocumentLink(navigation::ResolveDocumentLinkResponse),
    /// Document highlight response payload.
    DocumentHighlight(navigation::DocumentHighlightResponse),
    /// Selection ranges response payload.
    SelectionRanges(navigation::SelectionRangesResponse),
    /// Goto definition response payload.
    GotoDefinition(navigation::GotoDefinitionResponse),
    /// Goto declaration response payload.
    GotoDeclaration(navigation::GotoDeclarationResponse),
    /// Goto type definition response payload.
    GotoTypeDefinition(navigation::GotoTypeDefinitionResponse),
    /// Goto implementation response payload.
    GotoImplementation(navigation::GotoImplementationResponse),
    /// Find references response payload.
    FindReferences(navigation::FindReferencesResponse),
    /// Call hierarchy prepare response payload.
    PrepareCallHierarchy(navigation::PrepareCallHierarchyResponse),
    /// Incoming call hierarchy response payload.
    CallHierarchyIncoming(navigation::CallHierarchyIncomingResponse),
    /// Outgoing call hierarchy response payload.
    CallHierarchyOutgoing(navigation::CallHierarchyOutgoingResponse),
    /// Type hierarchy prepare response payload.
    PrepareTypeHierarchy(navigation::PrepareTypeHierarchyResponse),
    /// Type hierarchy supertypes response payload.
    TypeHierarchySupertypes(navigation::TypeHierarchySupertypesResponse),
    /// Type hierarchy subtypes response payload.
    TypeHierarchySubtypes(navigation::TypeHierarchySubtypesResponse),

    /// Prepare rename response payload.
    PrepareRename(refactor::PrepareRenameResponse),
    /// Rename response payload.
    Rename(refactor::RenameResponse),
    /// Rename files response payload.
    RenameFiles(refactor::RenameFilesResponse),
    /// Extract function response payload.
    ExtractFunction(refactor::ExtractFunctionResponse),
    /// Extract variable response payload.
    ExtractVariable(refactor::ExtractVariableResponse),
    /// Inline response payload.
    Inline(refactor::InlineResponse),
    /// Change signature response payload.
    ChangeSignature(refactor::ChangeSignatureResponse),
    /// Code actions response payload.
    CodeActions(refactor::CodeActionsResponse),
}

impl QueryResponse {
    /// Return the method identifier for this query response.
    pub fn method_id(&self) -> QueryMethodId {
        match self {
            Self::Completion(_) => QueryMethodId::Completion,
            Self::Hover(_) => QueryMethodId::Hover,
            Self::SignatureHelp(_) => QueryMethodId::SignatureHelp,
            Self::InlayHints(_) => QueryMethodId::InlayHints,
            Self::CodeLenses(_) => QueryMethodId::CodeLenses,
            Self::ResolveCodeLens(_) => QueryMethodId::ResolveCodeLens,
            Self::FoldingRanges(_) => QueryMethodId::FoldingRanges,
            Self::SemanticTokens(_) => QueryMethodId::SemanticTokens,
            Self::SemanticTokensRange(_) => QueryMethodId::SemanticTokensRange,
            Self::DocumentSymbols(_) => QueryMethodId::DocumentSymbols,
            Self::WorkspaceSymbols(_) => QueryMethodId::WorkspaceSymbols,
            Self::DocumentLinks(_) => QueryMethodId::DocumentLinks,
            Self::ResolveDocumentLink(_) => QueryMethodId::ResolveDocumentLink,
            Self::DocumentHighlight(_) => QueryMethodId::DocumentHighlight,
            Self::SelectionRanges(_) => QueryMethodId::SelectionRanges,
            Self::GotoDefinition(_) => QueryMethodId::GotoDefinition,
            Self::GotoDeclaration(_) => QueryMethodId::GotoDeclaration,
            Self::GotoTypeDefinition(_) => QueryMethodId::GotoTypeDefinition,
            Self::GotoImplementation(_) => QueryMethodId::GotoImplementation,
            Self::FindReferences(_) => QueryMethodId::FindReferences,
            Self::PrepareCallHierarchy(_) => QueryMethodId::PrepareCallHierarchy,
            Self::CallHierarchyIncoming(_) => QueryMethodId::CallHierarchyIncoming,
            Self::CallHierarchyOutgoing(_) => QueryMethodId::CallHierarchyOutgoing,
            Self::PrepareTypeHierarchy(_) => QueryMethodId::PrepareTypeHierarchy,
            Self::TypeHierarchySupertypes(_) => QueryMethodId::TypeHierarchySupertypes,
            Self::TypeHierarchySubtypes(_) => QueryMethodId::TypeHierarchySubtypes,
            Self::PrepareRename(_) => QueryMethodId::PrepareRename,
            Self::Rename(_) => QueryMethodId::Rename,
            Self::RenameFiles(_) => QueryMethodId::RenameFiles,
            Self::ExtractFunction(_) => QueryMethodId::ExtractFunction,
            Self::ExtractVariable(_) => QueryMethodId::ExtractVariable,
            Self::Inline(_) => QueryMethodId::Inline,
            Self::ChangeSignature(_) => QueryMethodId::ChangeSignature,
            Self::CodeActions(_) => QueryMethodId::CodeActions,
        }
    }
}
