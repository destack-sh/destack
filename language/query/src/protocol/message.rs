use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::QueryMethodId;
use crate::{assist, completion, navigation, refactor};

/// Query request envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
#[serde(tag = "kind", content = "params", rename_all = "snake_case")]
pub enum QueryRequest {
    /// Completion request payload.
    Completion(completion::CompletionRequest),
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

    /// Symbols request payload.
    Outline(navigation::OutlineRequest),
    /// Symbol search request payload.
    SymbolSearch(navigation::SymbolSearchRequest),
    /// Links request payload.
    Links(navigation::LinksRequest),
    /// Highlight request payload.
    Highlight(navigation::HighlightRequest),
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
    /// Call item request payload.
    CallItem(navigation::CallItemRequest),
    /// Incoming calls request payload.
    IncomingCalls(navigation::IncomingCallsRequest),
    /// Outgoing calls request payload.
    OutgoingCalls(navigation::OutgoingCallsRequest),
    /// Type hierarchy item request payload.
    TypeItem(navigation::TypeItemRequest),
    /// Type hierarchy supertypes request payload.
    Supertypes(navigation::SupertypesRequest),
    /// Type hierarchy subtypes request payload.
    Subtypes(navigation::SubtypesRequest),
    /// Decorator request payload.
    Decorators(navigation::DecoratorsRequest),

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
    /// Return the method identifier for this request.
    pub fn method_id(&self) -> QueryMethodId {
        // map request variants to protocol method ids
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
            Self::Outline(_) => QueryMethodId::Outline,
            Self::SymbolSearch(_) => QueryMethodId::SymbolSearch,
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
            Self::ExtractFunction(_) => QueryMethodId::ExtractFunction,
            Self::ExtractVariable(_) => QueryMethodId::ExtractVariable,
            Self::Inline(_) => QueryMethodId::Inline,
            Self::ChangeSignature(_) => QueryMethodId::ChangeSignature,
            Self::CodeActions(_) => QueryMethodId::CodeActions,
        }
    }
}

/// Query response envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
#[serde(tag = "kind", content = "result", rename_all = "snake_case")]
pub enum QueryResponse {
    /// Completion response payload.
    Completion(completion::CompletionResponse),
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

    /// Symbols response payload.
    Outline(navigation::OutlineResponse),
    /// Symbol search response payload.
    SymbolSearch(navigation::SymbolSearchResponse),
    /// Links response payload.
    Links(navigation::LinksResponse),
    /// Highlight response payload.
    Highlight(navigation::HighlightResponse),
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
    /// Call item response payload.
    CallItem(navigation::CallItemResponse),
    /// Incoming calls response payload.
    IncomingCalls(navigation::IncomingCallsResponse),
    /// Outgoing calls response payload.
    OutgoingCalls(navigation::OutgoingCallsResponse),
    /// Type hierarchy item response payload.
    TypeItem(navigation::TypeItemResponse),
    /// Type hierarchy supertypes response payload.
    Supertypes(navigation::SupertypesResponse),
    /// Type hierarchy subtypes response payload.
    Subtypes(navigation::SubtypesResponse),
    /// Decorator response payload.
    Decorators(navigation::DecoratorsResponse),

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
    /// Return the method identifier for this response.
    pub fn method_id(&self) -> QueryMethodId {
        // map response variants to protocol method ids
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
            Self::Outline(_) => QueryMethodId::Outline,
            Self::SymbolSearch(_) => QueryMethodId::SymbolSearch,
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
            Self::ExtractFunction(_) => QueryMethodId::ExtractFunction,
            Self::ExtractVariable(_) => QueryMethodId::ExtractVariable,
            Self::Inline(_) => QueryMethodId::Inline,
            Self::ChangeSignature(_) => QueryMethodId::ChangeSignature,
            Self::CodeActions(_) => QueryMethodId::CodeActions,
        }
    }
}
