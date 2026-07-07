# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.query.assist.folding
import destack._generated.query.assist.hover
import destack._generated.query.assist.inlay
import destack._generated.query.assist.lens
import destack._generated.query.assist.semantic
import destack._generated.query.assist.signature
import destack._generated.query.completion.item
import destack._generated.query.navigation.call
import destack._generated.query.navigation.decorator
import destack._generated.query.navigation.definition
import destack._generated.query.navigation.highlight
import destack._generated.query.navigation.implementation
import destack._generated.query.navigation.link
import destack._generated.query.navigation.reference
import destack._generated.query.navigation.selection
import destack._generated.query.navigation.symbol
import destack._generated.query.navigation.type
import destack._generated.query.navigation.workspace
import destack._generated.query.refactor.action
import destack._generated.query.refactor.file
import destack._generated.query.refactor.function
import destack._generated.query.refactor.inline
import destack._generated.query.refactor.rename
import destack._generated.query.refactor.signature
import destack._generated.query.refactor.variable

@dataclass(frozen=True, slots=True)
class QueryRequestCompletion:
    """Completion request payload."""

    completion: destack._generated.query.completion.item.CompletionRequest
    kind: typing.Literal["completion"] = "completion"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestHover:
    """Hover request payload."""

    hover: destack._generated.query.assist.hover.HoverRequest
    kind: typing.Literal["hover"] = "hover"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestSignatureHelp:
    """Signature help request payload."""

    signature_help: destack._generated.query.assist.signature.SignatureHelpRequest
    kind: typing.Literal["signatureHelp"] = "signatureHelp"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestInlayHints:
    """Inlay hints request payload."""

    inlay_hints: destack._generated.query.assist.inlay.InlayHintsRequest
    kind: typing.Literal["inlayHints"] = "inlayHints"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestCodeLenses:
    """Code lenses request payload."""

    code_lenses: destack._generated.query.assist.lens.CodeLensesRequest
    kind: typing.Literal["codeLenses"] = "codeLenses"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestResolveCodeLens:
    """Code lens resolve request payload."""

    resolve_code_lens: destack._generated.query.assist.lens.ResolveCodeLensRequest
    kind: typing.Literal["resolveCodeLens"] = "resolveCodeLens"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestFoldingRanges:
    """Folding ranges request payload."""

    folding_ranges: destack._generated.query.assist.folding.FoldingRangesRequest
    kind: typing.Literal["foldingRanges"] = "foldingRanges"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestSemanticTokens:
    """Full-document semantic tokens request payload."""

    semantic_tokens: destack._generated.query.assist.semantic.SemanticTokensRequest
    kind: typing.Literal["semanticTokens"] = "semanticTokens"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestSemanticTokensRange:
    """Range semantic tokens request payload."""

    semantic_tokens_range: (
        destack._generated.query.assist.semantic.SemanticTokensRangeRequest
    )
    kind: typing.Literal["semanticTokensRange"] = "semanticTokensRange"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestOutline:
    """Symbols request payload."""

    outline: destack._generated.query.navigation.symbol.OutlineRequest
    kind: typing.Literal["outline"] = "outline"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestSymbolSearch:
    """Symbol search request payload."""

    symbol_search: destack._generated.query.navigation.workspace.SymbolSearchRequest
    kind: typing.Literal["symbolSearch"] = "symbolSearch"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestLinks:
    """Links request payload."""

    links: destack._generated.query.navigation.link.LinksRequest
    kind: typing.Literal["links"] = "links"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestHighlight:
    """Highlight request payload."""

    highlight: destack._generated.query.navigation.highlight.HighlightRequest
    kind: typing.Literal["highlight"] = "highlight"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestSelectionRanges:
    """Selection ranges request payload."""

    selection_ranges: (
        destack._generated.query.navigation.selection.SelectionRangesRequest
    )
    kind: typing.Literal["selectionRanges"] = "selectionRanges"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestGotoDefinition:
    """Goto definition request payload."""

    goto_definition: (
        destack._generated.query.navigation.definition.GotoDefinitionRequest
    )
    kind: typing.Literal["gotoDefinition"] = "gotoDefinition"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestGotoDeclaration:
    """Goto declaration request payload."""

    goto_declaration: (
        destack._generated.query.navigation.definition.GotoDeclarationRequest
    )
    kind: typing.Literal["gotoDeclaration"] = "gotoDeclaration"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestGotoTypeDefinition:
    """Goto type definition request payload."""

    goto_type_definition: (
        destack._generated.query.navigation.definition.GotoTypeDefinitionRequest
    )
    kind: typing.Literal["gotoTypeDefinition"] = "gotoTypeDefinition"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestGotoImplementation:
    """Goto implementation request payload."""

    goto_implementation: (
        destack._generated.query.navigation.implementation.GotoImplementationRequest
    )
    kind: typing.Literal["gotoImplementation"] = "gotoImplementation"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestFindReferences:
    """Find references request payload."""

    find_references: destack._generated.query.navigation.reference.FindReferencesRequest
    kind: typing.Literal["findReferences"] = "findReferences"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestCallItem:
    """Call item request payload."""

    call_item: destack._generated.query.navigation.call.CallItemRequest
    kind: typing.Literal["callItem"] = "callItem"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestIncomingCalls:
    """Incoming calls request payload."""

    incoming_calls: destack._generated.query.navigation.call.IncomingCallsRequest
    kind: typing.Literal["incomingCalls"] = "incomingCalls"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestOutgoingCalls:
    """Outgoing calls request payload."""

    outgoing_calls: destack._generated.query.navigation.call.OutgoingCallsRequest
    kind: typing.Literal["outgoingCalls"] = "outgoingCalls"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestTypeItem:
    """Type hierarchy item request payload."""

    type_item: destack._generated.query.navigation.type.TypeItemRequest
    kind: typing.Literal["typeItem"] = "typeItem"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestSupertypes:
    """Type hierarchy supertypes request payload."""

    supertypes: destack._generated.query.navigation.type.SupertypesRequest
    kind: typing.Literal["supertypes"] = "supertypes"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestSubtypes:
    """Type hierarchy subtypes request payload."""

    subtypes: destack._generated.query.navigation.type.SubtypesRequest
    kind: typing.Literal["subtypes"] = "subtypes"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestDecorators:
    """Decorator request payload."""

    decorators: destack._generated.query.navigation.decorator.DecoratorsRequest
    kind: typing.Literal["decorators"] = "decorators"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestRenameTarget:
    """Rename target request payload."""

    rename_target: destack._generated.query.refactor.rename.RenameTargetRequest
    kind: typing.Literal["renameTarget"] = "renameTarget"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestRename:
    """Rename request payload."""

    rename: destack._generated.query.refactor.rename.RenameRequest
    kind: typing.Literal["rename"] = "rename"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestRenameFiles:
    """Rename files request payload."""

    rename_files: destack._generated.query.refactor.file.RenameFilesRequest
    kind: typing.Literal["renameFiles"] = "renameFiles"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestExtractFunction:
    """Extract function request payload."""

    extract_function: destack._generated.query.refactor.function.ExtractFunctionRequest
    kind: typing.Literal["extractFunction"] = "extractFunction"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestExtractVariable:
    """Extract variable request payload."""

    extract_variable: destack._generated.query.refactor.variable.ExtractVariableRequest
    kind: typing.Literal["extractVariable"] = "extractVariable"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestInline:
    """Inline request payload."""

    inline: destack._generated.query.refactor.inline.InlineRequest
    kind: typing.Literal["inline"] = "inline"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestChangeSignature:
    """Change signature request payload."""

    change_signature: destack._generated.query.refactor.signature.ChangeSignatureRequest
    kind: typing.Literal["changeSignature"] = "changeSignature"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestCodeActions:
    """Code actions request payload."""

    code_actions: destack._generated.query.refactor.action.CodeActionsRequest
    kind: typing.Literal["codeActions"] = "codeActions"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Query request envelope."""
QueryRequest: typing.TypeAlias = (
    QueryRequestCompletion
    | QueryRequestHover
    | QueryRequestSignatureHelp
    | QueryRequestInlayHints
    | QueryRequestCodeLenses
    | QueryRequestResolveCodeLens
    | QueryRequestFoldingRanges
    | QueryRequestSemanticTokens
    | QueryRequestSemanticTokensRange
    | QueryRequestOutline
    | QueryRequestSymbolSearch
    | QueryRequestLinks
    | QueryRequestHighlight
    | QueryRequestSelectionRanges
    | QueryRequestGotoDefinition
    | QueryRequestGotoDeclaration
    | QueryRequestGotoTypeDefinition
    | QueryRequestGotoImplementation
    | QueryRequestFindReferences
    | QueryRequestCallItem
    | QueryRequestIncomingCalls
    | QueryRequestOutgoingCalls
    | QueryRequestTypeItem
    | QueryRequestSupertypes
    | QueryRequestSubtypes
    | QueryRequestDecorators
    | QueryRequestRenameTarget
    | QueryRequestRename
    | QueryRequestRenameFiles
    | QueryRequestExtractFunction
    | QueryRequestExtractVariable
    | QueryRequestInline
    | QueryRequestChangeSignature
    | QueryRequestCodeActions
)

def encode_query_request(writer: BinaryWriter, value: QueryRequest) -> None: ...
def decode_query_request(reader: BinaryReader) -> QueryRequest: ...
def to_json_query_request(value: QueryRequest) -> Json: ...
def from_json_query_request(value: Json) -> QueryRequest: ...

@dataclass(frozen=True, slots=True)
class QueryResponseCompletion:
    """Completion response payload."""

    completion: destack._generated.query.completion.item.CompletionResponse
    kind: typing.Literal["completion"] = "completion"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseHover:
    """Hover response payload."""

    hover: destack._generated.query.assist.hover.HoverResponse
    kind: typing.Literal["hover"] = "hover"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseSignatureHelp:
    """Signature help response payload."""

    signature_help: destack._generated.query.assist.signature.SignatureHelpResponse
    kind: typing.Literal["signatureHelp"] = "signatureHelp"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseInlayHints:
    """Inlay hints response payload."""

    inlay_hints: destack._generated.query.assist.inlay.InlayHintsResponse
    kind: typing.Literal["inlayHints"] = "inlayHints"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseCodeLenses:
    """Code lenses response payload."""

    code_lenses: destack._generated.query.assist.lens.CodeLensesResponse
    kind: typing.Literal["codeLenses"] = "codeLenses"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseResolveCodeLens:
    """Code lens resolve response payload."""

    resolve_code_lens: destack._generated.query.assist.lens.ResolveCodeLensResponse
    kind: typing.Literal["resolveCodeLens"] = "resolveCodeLens"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseFoldingRanges:
    """Folding ranges response payload."""

    folding_ranges: destack._generated.query.assist.folding.FoldingRangesResponse
    kind: typing.Literal["foldingRanges"] = "foldingRanges"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseSemanticTokens:
    """Full-document semantic tokens response payload."""

    semantic_tokens: destack._generated.query.assist.semantic.SemanticTokensResponse
    kind: typing.Literal["semanticTokens"] = "semanticTokens"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseSemanticTokensRange:
    """Range semantic tokens response payload."""

    semantic_tokens_range: (
        destack._generated.query.assist.semantic.SemanticTokensResponse
    )
    kind: typing.Literal["semanticTokensRange"] = "semanticTokensRange"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseOutline:
    """Symbols response payload."""

    outline: destack._generated.query.navigation.symbol.OutlineResponse
    kind: typing.Literal["outline"] = "outline"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseSymbolSearch:
    """Symbol search response payload."""

    symbol_search: destack._generated.query.navigation.workspace.SymbolSearchResponse
    kind: typing.Literal["symbolSearch"] = "symbolSearch"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseLinks:
    """Links response payload."""

    links: destack._generated.query.navigation.link.LinksResponse
    kind: typing.Literal["links"] = "links"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseHighlight:
    """Highlight response payload."""

    highlight: destack._generated.query.navigation.highlight.HighlightResponse
    kind: typing.Literal["highlight"] = "highlight"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseSelectionRanges:
    """Selection ranges response payload."""

    selection_ranges: (
        destack._generated.query.navigation.selection.SelectionRangesResponse
    )
    kind: typing.Literal["selectionRanges"] = "selectionRanges"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseGotoDefinition:
    """Goto definition response payload."""

    goto_definition: (
        destack._generated.query.navigation.definition.GotoDefinitionResponse
    )
    kind: typing.Literal["gotoDefinition"] = "gotoDefinition"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseGotoDeclaration:
    """Goto declaration response payload."""

    goto_declaration: (
        destack._generated.query.navigation.definition.GotoDeclarationResponse
    )
    kind: typing.Literal["gotoDeclaration"] = "gotoDeclaration"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseGotoTypeDefinition:
    """Goto type definition response payload."""

    goto_type_definition: (
        destack._generated.query.navigation.definition.GotoTypeDefinitionResponse
    )
    kind: typing.Literal["gotoTypeDefinition"] = "gotoTypeDefinition"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseGotoImplementation:
    """Goto implementation response payload."""

    goto_implementation: (
        destack._generated.query.navigation.implementation.GotoImplementationResponse
    )
    kind: typing.Literal["gotoImplementation"] = "gotoImplementation"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseFindReferences:
    """Find references response payload."""

    find_references: (
        destack._generated.query.navigation.reference.FindReferencesResponse
    )
    kind: typing.Literal["findReferences"] = "findReferences"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseCallItem:
    """Call item response payload."""

    call_item: destack._generated.query.navigation.call.CallItemResponse
    kind: typing.Literal["callItem"] = "callItem"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseIncomingCalls:
    """Incoming calls response payload."""

    incoming_calls: destack._generated.query.navigation.call.IncomingCallsResponse
    kind: typing.Literal["incomingCalls"] = "incomingCalls"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseOutgoingCalls:
    """Outgoing calls response payload."""

    outgoing_calls: destack._generated.query.navigation.call.OutgoingCallsResponse
    kind: typing.Literal["outgoingCalls"] = "outgoingCalls"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseTypeItem:
    """Type hierarchy item response payload."""

    type_item: destack._generated.query.navigation.type.TypeItemResponse
    kind: typing.Literal["typeItem"] = "typeItem"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseSupertypes:
    """Type hierarchy supertypes response payload."""

    supertypes: destack._generated.query.navigation.type.SupertypesResponse
    kind: typing.Literal["supertypes"] = "supertypes"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseSubtypes:
    """Type hierarchy subtypes response payload."""

    subtypes: destack._generated.query.navigation.type.SubtypesResponse
    kind: typing.Literal["subtypes"] = "subtypes"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseDecorators:
    """Decorator response payload."""

    decorators: destack._generated.query.navigation.decorator.DecoratorsResponse
    kind: typing.Literal["decorators"] = "decorators"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseRenameTarget:
    """Rename target response payload."""

    rename_target: destack._generated.query.refactor.rename.RenameTargetResponse
    kind: typing.Literal["renameTarget"] = "renameTarget"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseRename:
    """Rename response payload."""

    rename: destack._generated.query.refactor.rename.RenameResponse
    kind: typing.Literal["rename"] = "rename"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseRenameFiles:
    """Rename files response payload."""

    rename_files: destack._generated.query.refactor.file.RenameFilesResponse
    kind: typing.Literal["renameFiles"] = "renameFiles"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseExtractFunction:
    """Extract function response payload."""

    extract_function: destack._generated.query.refactor.function.ExtractFunctionResponse
    kind: typing.Literal["extractFunction"] = "extractFunction"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseExtractVariable:
    """Extract variable response payload."""

    extract_variable: destack._generated.query.refactor.variable.ExtractVariableResponse
    kind: typing.Literal["extractVariable"] = "extractVariable"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseInline:
    """Inline response payload."""

    inline: destack._generated.query.refactor.inline.InlineResponse
    kind: typing.Literal["inline"] = "inline"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseChangeSignature:
    """Change signature response payload."""

    change_signature: (
        destack._generated.query.refactor.signature.ChangeSignatureResponse
    )
    kind: typing.Literal["changeSignature"] = "changeSignature"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseCodeActions:
    """Code actions response payload."""

    code_actions: destack._generated.query.refactor.action.CodeActionsResponse
    kind: typing.Literal["codeActions"] = "codeActions"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Query response envelope."""
QueryResponse: typing.TypeAlias = (
    QueryResponseCompletion
    | QueryResponseHover
    | QueryResponseSignatureHelp
    | QueryResponseInlayHints
    | QueryResponseCodeLenses
    | QueryResponseResolveCodeLens
    | QueryResponseFoldingRanges
    | QueryResponseSemanticTokens
    | QueryResponseSemanticTokensRange
    | QueryResponseOutline
    | QueryResponseSymbolSearch
    | QueryResponseLinks
    | QueryResponseHighlight
    | QueryResponseSelectionRanges
    | QueryResponseGotoDefinition
    | QueryResponseGotoDeclaration
    | QueryResponseGotoTypeDefinition
    | QueryResponseGotoImplementation
    | QueryResponseFindReferences
    | QueryResponseCallItem
    | QueryResponseIncomingCalls
    | QueryResponseOutgoingCalls
    | QueryResponseTypeItem
    | QueryResponseSupertypes
    | QueryResponseSubtypes
    | QueryResponseDecorators
    | QueryResponseRenameTarget
    | QueryResponseRename
    | QueryResponseRenameFiles
    | QueryResponseExtractFunction
    | QueryResponseExtractVariable
    | QueryResponseInline
    | QueryResponseChangeSignature
    | QueryResponseCodeActions
)

def encode_query_response(writer: BinaryWriter, value: QueryResponse) -> None: ...
def decode_query_response(reader: BinaryReader) -> QueryResponse: ...
def to_json_query_response(value: QueryResponse) -> Json: ...
def from_json_query_response(value: Json) -> QueryResponse: ...

__all__ = [
    "QueryRequest",
    "encode_query_request",
    "decode_query_request",
    "to_json_query_request",
    "from_json_query_request",
    "QueryRequestCompletion",
    "QueryRequestHover",
    "QueryRequestSignatureHelp",
    "QueryRequestInlayHints",
    "QueryRequestCodeLenses",
    "QueryRequestResolveCodeLens",
    "QueryRequestFoldingRanges",
    "QueryRequestSemanticTokens",
    "QueryRequestSemanticTokensRange",
    "QueryRequestOutline",
    "QueryRequestSymbolSearch",
    "QueryRequestLinks",
    "QueryRequestHighlight",
    "QueryRequestSelectionRanges",
    "QueryRequestGotoDefinition",
    "QueryRequestGotoDeclaration",
    "QueryRequestGotoTypeDefinition",
    "QueryRequestGotoImplementation",
    "QueryRequestFindReferences",
    "QueryRequestCallItem",
    "QueryRequestIncomingCalls",
    "QueryRequestOutgoingCalls",
    "QueryRequestTypeItem",
    "QueryRequestSupertypes",
    "QueryRequestSubtypes",
    "QueryRequestDecorators",
    "QueryRequestRenameTarget",
    "QueryRequestRename",
    "QueryRequestRenameFiles",
    "QueryRequestExtractFunction",
    "QueryRequestExtractVariable",
    "QueryRequestInline",
    "QueryRequestChangeSignature",
    "QueryRequestCodeActions",
    "QueryResponse",
    "encode_query_response",
    "decode_query_response",
    "to_json_query_response",
    "from_json_query_response",
    "QueryResponseCompletion",
    "QueryResponseHover",
    "QueryResponseSignatureHelp",
    "QueryResponseInlayHints",
    "QueryResponseCodeLenses",
    "QueryResponseResolveCodeLens",
    "QueryResponseFoldingRanges",
    "QueryResponseSemanticTokens",
    "QueryResponseSemanticTokensRange",
    "QueryResponseOutline",
    "QueryResponseSymbolSearch",
    "QueryResponseLinks",
    "QueryResponseHighlight",
    "QueryResponseSelectionRanges",
    "QueryResponseGotoDefinition",
    "QueryResponseGotoDeclaration",
    "QueryResponseGotoTypeDefinition",
    "QueryResponseGotoImplementation",
    "QueryResponseFindReferences",
    "QueryResponseCallItem",
    "QueryResponseIncomingCalls",
    "QueryResponseOutgoingCalls",
    "QueryResponseTypeItem",
    "QueryResponseSupertypes",
    "QueryResponseSubtypes",
    "QueryResponseDecorators",
    "QueryResponseRenameTarget",
    "QueryResponseRename",
    "QueryResponseRenameFiles",
    "QueryResponseExtractFunction",
    "QueryResponseExtractVariable",
    "QueryResponseInline",
    "QueryResponseChangeSignature",
    "QueryResponseCodeActions",
]
