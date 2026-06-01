use serde::{Deserialize, Serialize};

use super::method::QueryMethodId;
use crate::{assist, navigation, refactor};

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
    /// Call hierarchy item request payload.
    CallHierarchyItem(navigation::CallHierarchyItemRequest),
    /// Incoming call hierarchy request payload.
    CallHierarchyIncoming(navigation::CallHierarchyIncomingRequest),
    /// Outgoing call hierarchy request payload.
    CallHierarchyOutgoing(navigation::CallHierarchyOutgoingRequest),
    /// Type hierarchy item request payload.
    TypeHierarchyItem(navigation::TypeHierarchyItemRequest),
    /// Type hierarchy supertypes request payload.
    TypeHierarchySupertypes(navigation::TypeHierarchySupertypesRequest),
    /// Type hierarchy subtypes request payload.
    TypeHierarchySubtypes(navigation::TypeHierarchySubtypesRequest),
    /// Annotation request payload.
    Annotations(navigation::AnnotationsRequest),

    /// Rename target request payload.
    RenameTarget(refactor::RenameTargetRequest),
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
            Self::DocumentHighlight(_) => QueryMethodId::DocumentHighlight,
            Self::SelectionRanges(_) => QueryMethodId::SelectionRanges,
            Self::GotoDefinition(_) => QueryMethodId::GotoDefinition,
            Self::GotoDeclaration(_) => QueryMethodId::GotoDeclaration,
            Self::GotoTypeDefinition(_) => QueryMethodId::GotoTypeDefinition,
            Self::GotoImplementation(_) => QueryMethodId::GotoImplementation,
            Self::FindReferences(_) => QueryMethodId::FindReferences,
            Self::CallHierarchyItem(_) => QueryMethodId::CallHierarchyItem,
            Self::CallHierarchyIncoming(_) => QueryMethodId::CallHierarchyIncoming,
            Self::CallHierarchyOutgoing(_) => QueryMethodId::CallHierarchyOutgoing,
            Self::TypeHierarchyItem(_) => QueryMethodId::TypeHierarchyItem,
            Self::TypeHierarchySupertypes(_) => QueryMethodId::TypeHierarchySupertypes,
            Self::TypeHierarchySubtypes(_) => QueryMethodId::TypeHierarchySubtypes,
            Self::Annotations(_) => QueryMethodId::Annotations,
            Self::RenameTarget(_) => QueryMethodId::RenameTarget,
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
    /// Call hierarchy item response payload.
    CallHierarchyItem(navigation::CallHierarchyItemResponse),
    /// Incoming call hierarchy response payload.
    CallHierarchyIncoming(navigation::CallHierarchyIncomingResponse),
    /// Outgoing call hierarchy response payload.
    CallHierarchyOutgoing(navigation::CallHierarchyOutgoingResponse),
    /// Type hierarchy item response payload.
    TypeHierarchyItem(navigation::TypeHierarchyItemResponse),
    /// Type hierarchy supertypes response payload.
    TypeHierarchySupertypes(navigation::TypeHierarchySupertypesResponse),
    /// Type hierarchy subtypes response payload.
    TypeHierarchySubtypes(navigation::TypeHierarchySubtypesResponse),
    /// Annotation response payload.
    Annotations(navigation::AnnotationsResponse),

    /// Rename target response payload.
    RenameTarget(refactor::RenameTargetResponse),
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
            Self::DocumentHighlight(_) => QueryMethodId::DocumentHighlight,
            Self::SelectionRanges(_) => QueryMethodId::SelectionRanges,
            Self::GotoDefinition(_) => QueryMethodId::GotoDefinition,
            Self::GotoDeclaration(_) => QueryMethodId::GotoDeclaration,
            Self::GotoTypeDefinition(_) => QueryMethodId::GotoTypeDefinition,
            Self::GotoImplementation(_) => QueryMethodId::GotoImplementation,
            Self::FindReferences(_) => QueryMethodId::FindReferences,
            Self::CallHierarchyItem(_) => QueryMethodId::CallHierarchyItem,
            Self::CallHierarchyIncoming(_) => QueryMethodId::CallHierarchyIncoming,
            Self::CallHierarchyOutgoing(_) => QueryMethodId::CallHierarchyOutgoing,
            Self::TypeHierarchyItem(_) => QueryMethodId::TypeHierarchyItem,
            Self::TypeHierarchySupertypes(_) => QueryMethodId::TypeHierarchySupertypes,
            Self::TypeHierarchySubtypes(_) => QueryMethodId::TypeHierarchySubtypes,
            Self::Annotations(_) => QueryMethodId::Annotations,
            Self::RenameTarget(_) => QueryMethodId::RenameTarget,
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
