# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.query.assist.completion
import destack._generated.query.assist.folding
import destack._generated.query.assist.hover
import destack._generated.query.assist.inlay
import destack._generated.query.assist.lens
import destack._generated.query.assist.semantic
import destack._generated.query.assist.signature
import destack._generated.query.navigation.annotation
import destack._generated.query.navigation.call_hierarchy
import destack._generated.query.navigation.definition
import destack._generated.query.navigation.document_link
import destack._generated.query.navigation.document_symbol
import destack._generated.query.navigation.find_references
import destack._generated.query.navigation.highlight
import destack._generated.query.navigation.implementation
import destack._generated.query.navigation.selection_range
import destack._generated.query.navigation.type_hierarchy
import destack._generated.query.navigation.workspace_symbol
import destack._generated.query.refactor.change_signature
import destack._generated.query.refactor.code_action
import destack._generated.query.refactor.extract_function
import destack._generated.query.refactor.extract_variable
import destack._generated.query.refactor.file_rename
import destack._generated.query.refactor.inline
import destack._generated.query.refactor.rename

@dataclass(frozen=True, slots=True)
class QueryRequestCompletion:
    """Completion request payload."""

    completion: destack._generated.query.assist.completion.CompletionRequest
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
class QueryRequestDocumentSymbols:
    """Document symbols request payload."""

    document_symbols: (
        destack._generated.query.navigation.document_symbol.DocumentSymbolsRequest
    )
    kind: typing.Literal["documentSymbols"] = "documentSymbols"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestWorkspaceSymbols:
    """Workspace symbols request payload."""

    workspace_symbols: (
        destack._generated.query.navigation.workspace_symbol.WorkspaceSymbolsRequest
    )
    kind: typing.Literal["workspaceSymbols"] = "workspaceSymbols"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestDocumentLinks:
    """Document links request payload."""

    document_links: (
        destack._generated.query.navigation.document_link.DocumentLinksRequest
    )
    kind: typing.Literal["documentLinks"] = "documentLinks"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestDocumentHighlight:
    """Document highlight request payload."""

    document_highlight: (
        destack._generated.query.navigation.highlight.DocumentHighlightRequest
    )
    kind: typing.Literal["documentHighlight"] = "documentHighlight"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestSelectionRanges:
    """Selection ranges request payload."""

    selection_ranges: (
        destack._generated.query.navigation.selection_range.SelectionRangesRequest
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

    find_references: (
        destack._generated.query.navigation.find_references.FindReferencesRequest
    )
    kind: typing.Literal["findReferences"] = "findReferences"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestCallHierarchyItem:
    """Call hierarchy item request payload."""

    call_hierarchy_item: (
        destack._generated.query.navigation.call_hierarchy.CallHierarchyItemRequest
    )
    kind: typing.Literal["callHierarchyItem"] = "callHierarchyItem"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestCallHierarchyIncoming:
    """Incoming call hierarchy request payload."""

    call_hierarchy_incoming: (
        destack._generated.query.navigation.call_hierarchy.CallHierarchyIncomingRequest
    )
    kind: typing.Literal["callHierarchyIncoming"] = "callHierarchyIncoming"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestCallHierarchyOutgoing:
    """Outgoing call hierarchy request payload."""

    call_hierarchy_outgoing: (
        destack._generated.query.navigation.call_hierarchy.CallHierarchyOutgoingRequest
    )
    kind: typing.Literal["callHierarchyOutgoing"] = "callHierarchyOutgoing"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestTypeHierarchyItem:
    """Type hierarchy item request payload."""

    type_hierarchy_item: (
        destack._generated.query.navigation.type_hierarchy.TypeHierarchyItemRequest
    )
    kind: typing.Literal["typeHierarchyItem"] = "typeHierarchyItem"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestTypeHierarchySupertypes:
    """Type hierarchy supertypes request payload."""

    type_hierarchy_supertypes: destack._generated.query.navigation.type_hierarchy.TypeHierarchySupertypesRequest
    kind: typing.Literal["typeHierarchySupertypes"] = "typeHierarchySupertypes"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestTypeHierarchySubtypes:
    """Type hierarchy subtypes request payload."""

    type_hierarchy_subtypes: (
        destack._generated.query.navigation.type_hierarchy.TypeHierarchySubtypesRequest
    )
    kind: typing.Literal["typeHierarchySubtypes"] = "typeHierarchySubtypes"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestAnnotations:
    """Annotation request payload."""

    annotations: destack._generated.query.navigation.annotation.AnnotationsRequest
    kind: typing.Literal["annotations"] = "annotations"

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

    rename_files: destack._generated.query.refactor.file_rename.RenameFilesRequest
    kind: typing.Literal["renameFiles"] = "renameFiles"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestExtractFunction:
    """Extract function request payload."""

    extract_function: (
        destack._generated.query.refactor.extract_function.ExtractFunctionRequest
    )
    kind: typing.Literal["extractFunction"] = "extractFunction"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestExtractVariable:
    """Extract variable request payload."""

    extract_variable: (
        destack._generated.query.refactor.extract_variable.ExtractVariableRequest
    )
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

    change_signature: (
        destack._generated.query.refactor.change_signature.ChangeSignatureRequest
    )
    kind: typing.Literal["changeSignature"] = "changeSignature"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryRequestCodeActions:
    """Code actions request payload."""

    code_actions: destack._generated.query.refactor.code_action.CodeActionsRequest
    kind: typing.Literal["codeActions"] = "codeActions"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Query request payload."""
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
    | QueryRequestDocumentSymbols
    | QueryRequestWorkspaceSymbols
    | QueryRequestDocumentLinks
    | QueryRequestDocumentHighlight
    | QueryRequestSelectionRanges
    | QueryRequestGotoDefinition
    | QueryRequestGotoDeclaration
    | QueryRequestGotoTypeDefinition
    | QueryRequestGotoImplementation
    | QueryRequestFindReferences
    | QueryRequestCallHierarchyItem
    | QueryRequestCallHierarchyIncoming
    | QueryRequestCallHierarchyOutgoing
    | QueryRequestTypeHierarchyItem
    | QueryRequestTypeHierarchySupertypes
    | QueryRequestTypeHierarchySubtypes
    | QueryRequestAnnotations
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

    completion: destack._generated.query.assist.completion.CompletionResponse
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
class QueryResponseDocumentSymbols:
    """Document symbols response payload."""

    document_symbols: (
        destack._generated.query.navigation.document_symbol.DocumentSymbolsResponse
    )
    kind: typing.Literal["documentSymbols"] = "documentSymbols"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseWorkspaceSymbols:
    """Workspace symbols response payload."""

    workspace_symbols: (
        destack._generated.query.navigation.workspace_symbol.WorkspaceSymbolsResponse
    )
    kind: typing.Literal["workspaceSymbols"] = "workspaceSymbols"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseDocumentLinks:
    """Document links response payload."""

    document_links: (
        destack._generated.query.navigation.document_link.DocumentLinksResponse
    )
    kind: typing.Literal["documentLinks"] = "documentLinks"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseDocumentHighlight:
    """Document highlight response payload."""

    document_highlight: (
        destack._generated.query.navigation.highlight.DocumentHighlightResponse
    )
    kind: typing.Literal["documentHighlight"] = "documentHighlight"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseSelectionRanges:
    """Selection ranges response payload."""

    selection_ranges: (
        destack._generated.query.navigation.selection_range.SelectionRangesResponse
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
        destack._generated.query.navigation.find_references.FindReferencesResponse
    )
    kind: typing.Literal["findReferences"] = "findReferences"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseCallHierarchyItem:
    """Call hierarchy item response payload."""

    call_hierarchy_item: (
        destack._generated.query.navigation.call_hierarchy.CallHierarchyItemResponse
    )
    kind: typing.Literal["callHierarchyItem"] = "callHierarchyItem"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseCallHierarchyIncoming:
    """Incoming call hierarchy response payload."""

    call_hierarchy_incoming: (
        destack._generated.query.navigation.call_hierarchy.CallHierarchyIncomingResponse
    )
    kind: typing.Literal["callHierarchyIncoming"] = "callHierarchyIncoming"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseCallHierarchyOutgoing:
    """Outgoing call hierarchy response payload."""

    call_hierarchy_outgoing: (
        destack._generated.query.navigation.call_hierarchy.CallHierarchyOutgoingResponse
    )
    kind: typing.Literal["callHierarchyOutgoing"] = "callHierarchyOutgoing"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseTypeHierarchyItem:
    """Type hierarchy item response payload."""

    type_hierarchy_item: (
        destack._generated.query.navigation.type_hierarchy.TypeHierarchyItemResponse
    )
    kind: typing.Literal["typeHierarchyItem"] = "typeHierarchyItem"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseTypeHierarchySupertypes:
    """Type hierarchy supertypes response payload."""

    type_hierarchy_supertypes: destack._generated.query.navigation.type_hierarchy.TypeHierarchySupertypesResponse
    kind: typing.Literal["typeHierarchySupertypes"] = "typeHierarchySupertypes"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseTypeHierarchySubtypes:
    """Type hierarchy subtypes response payload."""

    type_hierarchy_subtypes: (
        destack._generated.query.navigation.type_hierarchy.TypeHierarchySubtypesResponse
    )
    kind: typing.Literal["typeHierarchySubtypes"] = "typeHierarchySubtypes"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseAnnotations:
    """Annotation response payload."""

    annotations: destack._generated.query.navigation.annotation.AnnotationsResponse
    kind: typing.Literal["annotations"] = "annotations"

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

    rename_files: destack._generated.query.refactor.file_rename.RenameFilesResponse
    kind: typing.Literal["renameFiles"] = "renameFiles"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseExtractFunction:
    """Extract function response payload."""

    extract_function: (
        destack._generated.query.refactor.extract_function.ExtractFunctionResponse
    )
    kind: typing.Literal["extractFunction"] = "extractFunction"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseExtractVariable:
    """Extract variable response payload."""

    extract_variable: (
        destack._generated.query.refactor.extract_variable.ExtractVariableResponse
    )
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
        destack._generated.query.refactor.change_signature.ChangeSignatureResponse
    )
    kind: typing.Literal["changeSignature"] = "changeSignature"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class QueryResponseCodeActions:
    """Code actions response payload."""

    code_actions: destack._generated.query.refactor.code_action.CodeActionsResponse
    kind: typing.Literal["codeActions"] = "codeActions"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Query response payload."""
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
    | QueryResponseDocumentSymbols
    | QueryResponseWorkspaceSymbols
    | QueryResponseDocumentLinks
    | QueryResponseDocumentHighlight
    | QueryResponseSelectionRanges
    | QueryResponseGotoDefinition
    | QueryResponseGotoDeclaration
    | QueryResponseGotoTypeDefinition
    | QueryResponseGotoImplementation
    | QueryResponseFindReferences
    | QueryResponseCallHierarchyItem
    | QueryResponseCallHierarchyIncoming
    | QueryResponseCallHierarchyOutgoing
    | QueryResponseTypeHierarchyItem
    | QueryResponseTypeHierarchySupertypes
    | QueryResponseTypeHierarchySubtypes
    | QueryResponseAnnotations
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
    "QueryRequestDocumentSymbols",
    "QueryRequestWorkspaceSymbols",
    "QueryRequestDocumentLinks",
    "QueryRequestDocumentHighlight",
    "QueryRequestSelectionRanges",
    "QueryRequestGotoDefinition",
    "QueryRequestGotoDeclaration",
    "QueryRequestGotoTypeDefinition",
    "QueryRequestGotoImplementation",
    "QueryRequestFindReferences",
    "QueryRequestCallHierarchyItem",
    "QueryRequestCallHierarchyIncoming",
    "QueryRequestCallHierarchyOutgoing",
    "QueryRequestTypeHierarchyItem",
    "QueryRequestTypeHierarchySupertypes",
    "QueryRequestTypeHierarchySubtypes",
    "QueryRequestAnnotations",
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
    "QueryResponseDocumentSymbols",
    "QueryResponseWorkspaceSymbols",
    "QueryResponseDocumentLinks",
    "QueryResponseDocumentHighlight",
    "QueryResponseSelectionRanges",
    "QueryResponseGotoDefinition",
    "QueryResponseGotoDeclaration",
    "QueryResponseGotoTypeDefinition",
    "QueryResponseGotoImplementation",
    "QueryResponseFindReferences",
    "QueryResponseCallHierarchyItem",
    "QueryResponseCallHierarchyIncoming",
    "QueryResponseCallHierarchyOutgoing",
    "QueryResponseTypeHierarchyItem",
    "QueryResponseTypeHierarchySupertypes",
    "QueryResponseTypeHierarchySubtypes",
    "QueryResponseAnnotations",
    "QueryResponseRenameTarget",
    "QueryResponseRename",
    "QueryResponseRenameFiles",
    "QueryResponseExtractFunction",
    "QueryResponseExtractVariable",
    "QueryResponseInline",
    "QueryResponseChangeSignature",
    "QueryResponseCodeActions",
]
