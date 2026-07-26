use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::QueryMethodId;
use crate::{
    CallItemRequest, CallItemResponse, CodeActionsRequest, CodeActionsResponse, CodeLensesRequest,
    CodeLensesResponse, CompletionRequest, CompletionResponse, DecoratorsRequest,
    DecoratorsResponse, ExtractVariableRequest, ExtractVariableResponse, FindReferencesRequest,
    FindReferencesResponse, FoldingRangesRequest, FoldingRangesResponse, GotoDeclarationRequest,
    GotoDeclarationResponse, GotoDefinitionRequest, GotoDefinitionResponse,
    GotoImplementationRequest, GotoImplementationResponse, GotoTypeDefinitionRequest,
    GotoTypeDefinitionResponse, HighlightRequest, HighlightResponse, HoverRequest, HoverResponse,
    IncomingCallsRequest, IncomingCallsResponse, InlayHintsRequest, InlayHintsResponse,
    InlineRequest, InlineResponse, LinksRequest, LinksResponse, OutgoingCallsRequest,
    OutgoingCallsResponse, OutlineRequest, OutlineResponse, RenameFilesRequest,
    RenameFilesResponse, RenameRequest, RenameResponse, RenameTargetRequest, RenameTargetResponse,
    SearchSymbolsRequest, SearchSymbolsResponse, SelectionRangesRequest, SelectionRangesResponse,
    SemanticTokensRangeRequest, SemanticTokensRangeResponse, SemanticTokensRequest,
    SemanticTokensResponse, SignatureHelpRequest, SignatureHelpResponse, SubtypesRequest,
    SubtypesResponse, SupertypesRequest, SupertypesResponse, TypeItemRequest, TypeItemResponse,
};

/// Query request envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
#[serde(tag = "kind", content = "params", rename_all = "snake_case")]
pub enum QueryRequest {
    /// Completion request payload.
    Completion(CompletionRequest),
    /// Hover request payload.
    Hover(HoverRequest),
    /// Signature help request payload.
    SignatureHelp(SignatureHelpRequest),
    /// Inlay hints request payload.
    InlayHints(InlayHintsRequest),
    /// Code lenses request payload.
    CodeLenses(CodeLensesRequest),
    /// Folding ranges request payload.
    FoldingRanges(FoldingRangesRequest),
    /// Full-document semantic tokens request payload.
    SemanticTokens(SemanticTokensRequest),
    /// Range semantic tokens request payload.
    SemanticTokensRange(SemanticTokensRangeRequest),

    /// Symbols request payload.
    Outline(OutlineRequest),
    /// Symbol search request payload.
    SearchSymbols(SearchSymbolsRequest),
    /// Links request payload.
    Links(LinksRequest),
    /// Highlight request payload.
    Highlight(HighlightRequest),
    /// Selection ranges request payload.
    SelectionRanges(SelectionRangesRequest),
    /// Goto definition request payload.
    GotoDefinition(GotoDefinitionRequest),
    /// Goto declaration request payload.
    GotoDeclaration(GotoDeclarationRequest),
    /// Goto type definition request payload.
    GotoTypeDefinition(GotoTypeDefinitionRequest),
    /// Goto implementation request payload.
    GotoImplementation(GotoImplementationRequest),
    /// Find references request payload.
    FindReferences(FindReferencesRequest),
    /// Call item request payload.
    CallItem(CallItemRequest),
    /// Incoming calls request payload.
    IncomingCalls(IncomingCallsRequest),
    /// Outgoing calls request payload.
    OutgoingCalls(OutgoingCallsRequest),
    /// Type hierarchy item request payload.
    TypeItem(TypeItemRequest),
    /// Type hierarchy supertypes request payload.
    Supertypes(SupertypesRequest),
    /// Type hierarchy subtypes request payload.
    Subtypes(SubtypesRequest),
    /// Decorator request payload.
    Decorators(DecoratorsRequest),

    /// Rename target request payload.
    RenameTarget(RenameTargetRequest),
    /// Rename request payload.
    Rename(RenameRequest),
    /// Rename files request payload.
    RenameFiles(RenameFilesRequest),
    /// Extract variable request payload.
    ExtractVariable(ExtractVariableRequest),
    /// Inline request payload.
    Inline(InlineRequest),
    /// Code actions request payload.
    CodeActions(CodeActionsRequest),
}

impl QueryRequest {
    /// Return the method identifier for this request.
    pub fn method_id(&self) -> QueryMethodId {
        // map request variants to protocol method ids
        match self {
            Self::Completion(_) => QueryMethodId::Completion,
            Self::Hover(_) => QueryMethodId::Hover,
            Self::SignatureHelp(_) => QueryMethodId::SignatureHelp,
            Self::InlayHints(_) => QueryMethodId::InlayHints,
            Self::CodeLenses(_) => QueryMethodId::CodeLenses,
            Self::FoldingRanges(_) => QueryMethodId::FoldingRanges,
            Self::SemanticTokens(_) => QueryMethodId::SemanticTokens,
            Self::SemanticTokensRange(_) => QueryMethodId::SemanticTokensRange,
            Self::Outline(_) => QueryMethodId::Outline,
            Self::SearchSymbols(_) => QueryMethodId::SearchSymbols,
            Self::Links(_) => QueryMethodId::Links,
            Self::Highlight(_) => QueryMethodId::Highlight,
            Self::SelectionRanges(_) => QueryMethodId::SelectionRanges,
            Self::GotoDefinition(_) => QueryMethodId::GotoDefinition,
            Self::GotoDeclaration(_) => QueryMethodId::GotoDeclaration,
            Self::GotoTypeDefinition(_) => QueryMethodId::GotoTypeDefinition,
            Self::GotoImplementation(_) => QueryMethodId::GotoImplementation,
            Self::FindReferences(_) => QueryMethodId::FindReferences,
            Self::CallItem(_) => QueryMethodId::CallItem,
            Self::IncomingCalls(_) => QueryMethodId::IncomingCalls,
            Self::OutgoingCalls(_) => QueryMethodId::OutgoingCalls,
            Self::TypeItem(_) => QueryMethodId::TypeItem,
            Self::Supertypes(_) => QueryMethodId::Supertypes,
            Self::Subtypes(_) => QueryMethodId::Subtypes,
            Self::Decorators(_) => QueryMethodId::Decorators,
            Self::RenameTarget(_) => QueryMethodId::RenameTarget,
            Self::Rename(_) => QueryMethodId::Rename,
            Self::RenameFiles(_) => QueryMethodId::RenameFiles,
            Self::ExtractVariable(_) => QueryMethodId::ExtractVariable,
            Self::Inline(_) => QueryMethodId::Inline,
            Self::CodeActions(_) => QueryMethodId::CodeActions,
        }
    }
}

/// Query response envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
#[serde(tag = "kind", content = "result", rename_all = "snake_case")]
pub enum QueryResponse {
    /// Completion response payload.
    Completion(CompletionResponse),
    /// Hover response payload.
    Hover(HoverResponse),
    /// Signature help response payload.
    SignatureHelp(SignatureHelpResponse),
    /// Inlay hints response payload.
    InlayHints(InlayHintsResponse),
    /// Code lenses response payload.
    CodeLenses(CodeLensesResponse),
    /// Folding ranges response payload.
    FoldingRanges(FoldingRangesResponse),
    /// Full-document semantic tokens response payload.
    SemanticTokens(SemanticTokensResponse),
    /// Range semantic tokens response payload.
    SemanticTokensRange(SemanticTokensRangeResponse),

    /// Symbols response payload.
    Outline(OutlineResponse),
    /// Symbol search response payload.
    SearchSymbols(SearchSymbolsResponse),
    /// Links response payload.
    Links(LinksResponse),
    /// Highlight response payload.
    Highlight(HighlightResponse),
    /// Selection ranges response payload.
    SelectionRanges(SelectionRangesResponse),
    /// Goto definition response payload.
    GotoDefinition(GotoDefinitionResponse),
    /// Goto declaration response payload.
    GotoDeclaration(GotoDeclarationResponse),
    /// Goto type definition response payload.
    GotoTypeDefinition(GotoTypeDefinitionResponse),
    /// Goto implementation response payload.
    GotoImplementation(GotoImplementationResponse),
    /// Find references response payload.
    FindReferences(FindReferencesResponse),
    /// Call item response payload.
    CallItem(CallItemResponse),
    /// Incoming calls response payload.
    IncomingCalls(IncomingCallsResponse),
    /// Outgoing calls response payload.
    OutgoingCalls(OutgoingCallsResponse),
    /// Type hierarchy item response payload.
    TypeItem(TypeItemResponse),
    /// Type hierarchy supertypes response payload.
    Supertypes(SupertypesResponse),
    /// Type hierarchy subtypes response payload.
    Subtypes(SubtypesResponse),
    /// Decorator response payload.
    Decorators(DecoratorsResponse),

    /// Rename target response payload.
    RenameTarget(RenameTargetResponse),
    /// Rename response payload.
    Rename(RenameResponse),
    /// Rename files response payload.
    RenameFiles(RenameFilesResponse),
    /// Extract variable response payload.
    ExtractVariable(ExtractVariableResponse),
    /// Inline response payload.
    Inline(InlineResponse),
    /// Code actions response payload.
    CodeActions(CodeActionsResponse),
}

impl QueryResponse {
    /// Return the method identifier for this response.
    pub fn method_id(&self) -> QueryMethodId {
        // map response variants to protocol method ids
        match self {
            Self::Completion(_) => QueryMethodId::Completion,
            Self::Hover(_) => QueryMethodId::Hover,
            Self::SignatureHelp(_) => QueryMethodId::SignatureHelp,
            Self::InlayHints(_) => QueryMethodId::InlayHints,
            Self::CodeLenses(_) => QueryMethodId::CodeLenses,
            Self::FoldingRanges(_) => QueryMethodId::FoldingRanges,
            Self::SemanticTokens(_) => QueryMethodId::SemanticTokens,
            Self::SemanticTokensRange(_) => QueryMethodId::SemanticTokensRange,
            Self::Outline(_) => QueryMethodId::Outline,
            Self::SearchSymbols(_) => QueryMethodId::SearchSymbols,
            Self::Links(_) => QueryMethodId::Links,
            Self::Highlight(_) => QueryMethodId::Highlight,
            Self::SelectionRanges(_) => QueryMethodId::SelectionRanges,
            Self::GotoDefinition(_) => QueryMethodId::GotoDefinition,
            Self::GotoDeclaration(_) => QueryMethodId::GotoDeclaration,
            Self::GotoTypeDefinition(_) => QueryMethodId::GotoTypeDefinition,
            Self::GotoImplementation(_) => QueryMethodId::GotoImplementation,
            Self::FindReferences(_) => QueryMethodId::FindReferences,
            Self::CallItem(_) => QueryMethodId::CallItem,
            Self::IncomingCalls(_) => QueryMethodId::IncomingCalls,
            Self::OutgoingCalls(_) => QueryMethodId::OutgoingCalls,
            Self::TypeItem(_) => QueryMethodId::TypeItem,
            Self::Supertypes(_) => QueryMethodId::Supertypes,
            Self::Subtypes(_) => QueryMethodId::Subtypes,
            Self::Decorators(_) => QueryMethodId::Decorators,
            Self::RenameTarget(_) => QueryMethodId::RenameTarget,
            Self::Rename(_) => QueryMethodId::Rename,
            Self::RenameFiles(_) => QueryMethodId::RenameFiles,
            Self::ExtractVariable(_) => QueryMethodId::ExtractVariable,
            Self::Inline(_) => QueryMethodId::Inline,
            Self::CodeActions(_) => QueryMethodId::CodeActions,
        }
    }
}
