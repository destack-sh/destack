use destack_serde::Reflect;
use destack_source::ProfileId;
use serde::{Deserialize, Serialize};

use super::QueryMethod;
use crate::{
    CallItemRequest, CallItemResponse, CodeActionsRequest, CodeActionsResponse, CodeLensesRequest,
    CodeLensesResponse, CompletionRequest, CompletionResponse, DecoratorScope, DecoratorsRequest,
    DecoratorsResponse, ExtractVariableRequest, ExtractVariableResponse, FindReferencesRequest,
    FindReferencesResponse, FoldingRangesRequest, FoldingRangesResponse, GotoDeclarationRequest,
    GotoDeclarationResponse, GotoDefinitionRequest, GotoDefinitionResponse,
    GotoImplementationRequest, GotoImplementationResponse, GotoTypeDefinitionRequest,
    GotoTypeDefinitionResponse, HighlightRequest, HighlightResponse, HoverRequest, HoverResponse,
    IncomingCallsRequest, IncomingCallsResponse, InlayHintsRequest, InlayHintsResponse,
    InlineRequest, InlineResponse, LinksRequest, LinksResponse, Module, OutgoingCallsRequest,
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
    /// Return the selected program profile when this request reads one program.
    pub fn profile_id(&self) -> Option<ProfileId> {
        match self {
            Self::Completion(params) => Some(params.position.module.profile_id),
            Self::Hover(params) => Some(params.position.module.profile_id),
            Self::SignatureHelp(params) => Some(params.position.module.profile_id),
            Self::InlayHints(params) => Some(params.range.module.profile_id),
            Self::CodeLenses(params) => Some(params.module.profile_id),
            Self::FoldingRanges(params) => Some(params.module.profile_id),
            Self::SemanticTokens(params) => Some(params.module.profile_id),
            Self::SemanticTokensRange(params) => Some(params.range.module.profile_id),
            Self::Outline(params) => Some(params.module.profile_id),
            Self::Links(params) => Some(params.module.profile_id),
            Self::Highlight(params) => Some(params.position.module.profile_id),
            Self::SelectionRanges(params) => Some(params.module.profile_id),
            Self::GotoDefinition(params) => Some(params.position.module.profile_id),
            Self::GotoDeclaration(params) => Some(params.position.module.profile_id),
            Self::GotoTypeDefinition(params) => Some(params.position.module.profile_id),
            Self::GotoImplementation(params) => Some(params.position.module.profile_id),
            Self::FindReferences(params) => Some(params.position.module.profile_id),
            Self::CallItem(params) => Some(params.position.module.profile_id),
            Self::IncomingCalls(params) => Some(params.item.target.module.profile_id),
            Self::OutgoingCalls(params) => Some(params.item.target.module.profile_id),
            Self::TypeItem(params) => Some(params.position.module.profile_id),
            Self::Supertypes(params) => Some(params.item.target.module.profile_id),
            Self::Subtypes(params) => Some(params.item.target.module.profile_id),
            Self::Decorators(params) => match params.scope {
                DecoratorScope::Module(module) => Some(module.profile_id),
                DecoratorScope::Program(profile_id) => Some(profile_id),
            },
            Self::RenameTarget(params) => Some(params.position.module.profile_id),
            Self::Rename(params) => Some(params.position.module.profile_id),
            Self::ExtractVariable(params) => Some(params.range.module.profile_id),
            Self::Inline(params) => Some(params.position.module.profile_id),
            Self::CodeActions(params) => Some(params.range.module.profile_id),
            Self::SearchSymbols(_) | Self::RenameFiles(_) => None,
        }
    }

    /// Return the anchored module context read by this request when one exists.
    pub fn module(&self) -> Option<Module> {
        match self {
            Self::Completion(params) => Some(params.position.module),
            Self::Hover(params) => Some(params.position.module),
            Self::SignatureHelp(params) => Some(params.position.module),
            Self::InlayHints(params) => Some(params.range.module),
            Self::CodeLenses(params) => Some(params.module),
            Self::FoldingRanges(params) => Some(params.module),
            Self::SemanticTokens(params) => Some(params.module),
            Self::SemanticTokensRange(params) => Some(params.range.module),
            Self::Outline(params) => Some(params.module),
            Self::Links(params) => Some(params.module),
            Self::Highlight(params) => Some(params.position.module),
            Self::SelectionRanges(params) => Some(params.module),
            Self::GotoDefinition(params) => Some(params.position.module),
            Self::GotoDeclaration(params) => Some(params.position.module),
            Self::GotoTypeDefinition(params) => Some(params.position.module),
            Self::GotoImplementation(params) => Some(params.position.module),
            Self::FindReferences(params) => Some(params.position.module),
            Self::CallItem(params) => Some(params.position.module),
            Self::TypeItem(params) => Some(params.position.module),
            Self::RenameTarget(params) => Some(params.position.module),
            Self::Rename(params) => Some(params.position.module),
            Self::ExtractVariable(params) => Some(params.range.module),
            Self::Inline(params) => Some(params.position.module),
            Self::CodeActions(params) => Some(params.range.module),

            Self::SearchSymbols(_)
            | Self::IncomingCalls(_)
            | Self::OutgoingCalls(_)
            | Self::Supertypes(_)
            | Self::Subtypes(_)
            | Self::Decorators(_)
            | Self::RenameFiles(_) => None,
        }
    }

    /// Return this request's query method.
    pub fn method(&self) -> QueryMethod {
        // map request variants to query methods
        match self {
            Self::Completion(_) => QueryMethod::Completion,
            Self::Hover(_) => QueryMethod::Hover,
            Self::SignatureHelp(_) => QueryMethod::SignatureHelp,
            Self::InlayHints(_) => QueryMethod::InlayHints,
            Self::CodeLenses(_) => QueryMethod::CodeLenses,
            Self::FoldingRanges(_) => QueryMethod::FoldingRanges,
            Self::SemanticTokens(_) => QueryMethod::SemanticTokens,
            Self::SemanticTokensRange(_) => QueryMethod::SemanticTokensRange,
            Self::Outline(_) => QueryMethod::Outline,
            Self::SearchSymbols(_) => QueryMethod::SearchSymbols,
            Self::Links(_) => QueryMethod::Links,
            Self::Highlight(_) => QueryMethod::Highlight,
            Self::SelectionRanges(_) => QueryMethod::SelectionRanges,
            Self::GotoDefinition(_) => QueryMethod::GotoDefinition,
            Self::GotoDeclaration(_) => QueryMethod::GotoDeclaration,
            Self::GotoTypeDefinition(_) => QueryMethod::GotoTypeDefinition,
            Self::GotoImplementation(_) => QueryMethod::GotoImplementation,
            Self::FindReferences(_) => QueryMethod::FindReferences,
            Self::CallItem(_) => QueryMethod::CallItem,
            Self::IncomingCalls(_) => QueryMethod::IncomingCalls,
            Self::OutgoingCalls(_) => QueryMethod::OutgoingCalls,
            Self::TypeItem(_) => QueryMethod::TypeItem,
            Self::Supertypes(_) => QueryMethod::Supertypes,
            Self::Subtypes(_) => QueryMethod::Subtypes,
            Self::Decorators(_) => QueryMethod::Decorators,
            Self::RenameTarget(_) => QueryMethod::RenameTarget,
            Self::Rename(_) => QueryMethod::Rename,
            Self::RenameFiles(_) => QueryMethod::RenameFiles,
            Self::ExtractVariable(_) => QueryMethod::ExtractVariable,
            Self::Inline(_) => QueryMethod::Inline,
            Self::CodeActions(_) => QueryMethod::CodeActions,
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
    /// Return this response's query method.
    pub fn method(&self) -> QueryMethod {
        // map response variants to query methods
        match self {
            Self::Completion(_) => QueryMethod::Completion,
            Self::Hover(_) => QueryMethod::Hover,
            Self::SignatureHelp(_) => QueryMethod::SignatureHelp,
            Self::InlayHints(_) => QueryMethod::InlayHints,
            Self::CodeLenses(_) => QueryMethod::CodeLenses,
            Self::FoldingRanges(_) => QueryMethod::FoldingRanges,
            Self::SemanticTokens(_) => QueryMethod::SemanticTokens,
            Self::SemanticTokensRange(_) => QueryMethod::SemanticTokensRange,
            Self::Outline(_) => QueryMethod::Outline,
            Self::SearchSymbols(_) => QueryMethod::SearchSymbols,
            Self::Links(_) => QueryMethod::Links,
            Self::Highlight(_) => QueryMethod::Highlight,
            Self::SelectionRanges(_) => QueryMethod::SelectionRanges,
            Self::GotoDefinition(_) => QueryMethod::GotoDefinition,
            Self::GotoDeclaration(_) => QueryMethod::GotoDeclaration,
            Self::GotoTypeDefinition(_) => QueryMethod::GotoTypeDefinition,
            Self::GotoImplementation(_) => QueryMethod::GotoImplementation,
            Self::FindReferences(_) => QueryMethod::FindReferences,
            Self::CallItem(_) => QueryMethod::CallItem,
            Self::IncomingCalls(_) => QueryMethod::IncomingCalls,
            Self::OutgoingCalls(_) => QueryMethod::OutgoingCalls,
            Self::TypeItem(_) => QueryMethod::TypeItem,
            Self::Supertypes(_) => QueryMethod::Supertypes,
            Self::Subtypes(_) => QueryMethod::Subtypes,
            Self::Decorators(_) => QueryMethod::Decorators,
            Self::RenameTarget(_) => QueryMethod::RenameTarget,
            Self::Rename(_) => QueryMethod::Rename,
            Self::RenameFiles(_) => QueryMethod::RenameFiles,
            Self::ExtractVariable(_) => QueryMethod::ExtractVariable,
            Self::Inline(_) => QueryMethod::Inline,
            Self::CodeActions(_) => QueryMethod::CodeActions,
        }
    }
}
