# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.query.assist.completion
import destack._generated.protocol.query.assist.folding
import destack._generated.protocol.query.assist.hover
import destack._generated.protocol.query.assist.inlay
import destack._generated.protocol.query.assist.lens
import destack._generated.protocol.query.assist.semantic
import destack._generated.protocol.query.assist.signature
import destack._generated.protocol.query.navigation.annotation
import destack._generated.protocol.query.navigation.call_hierarchy
import destack._generated.protocol.query.navigation.definition
import destack._generated.protocol.query.navigation.document_link
import destack._generated.protocol.query.navigation.document_symbol
import destack._generated.protocol.query.navigation.find_references
import destack._generated.protocol.query.navigation.highlight
import destack._generated.protocol.query.navigation.implementation
import destack._generated.protocol.query.navigation.selection_range
import destack._generated.protocol.query.navigation.type_hierarchy
import destack._generated.protocol.query.navigation.workspace_symbol
import destack._generated.protocol.query.refactor.change_signature
import destack._generated.protocol.query.refactor.code_action
import destack._generated.protocol.query.refactor.extract_function
import destack._generated.protocol.query.refactor.extract_variable
import destack._generated.protocol.query.refactor.file_rename
import destack._generated.protocol.query.refactor.inline
import destack._generated.protocol.query.refactor.rename

if TYPE_CHECKING:
    from destack._generated.protocol.query.assist.completion import (
        CompletionRequest,
        CompletionResponse,
    )

    from destack._generated.protocol.query.assist.folding import (
        FoldingRangesRequest,
        FoldingRangesResponse,
    )

    from destack._generated.protocol.query.assist.hover import (
        HoverRequest,
        HoverResponse,
    )

    from destack._generated.protocol.query.assist.inlay import (
        InlayHintsRequest,
        InlayHintsResponse,
    )

    from destack._generated.protocol.query.assist.lens import (
        CodeLensesRequest,
        CodeLensesResponse,
        ResolveCodeLensRequest,
        ResolveCodeLensResponse,
    )

    from destack._generated.protocol.query.assist.semantic import (
        SemanticTokensRangeRequest,
        SemanticTokensRequest,
        SemanticTokensResponse,
    )

    from destack._generated.protocol.query.assist.signature import (
        SignatureHelpRequest,
        SignatureHelpResponse,
    )

    from destack._generated.protocol.query.navigation.annotation import (
        AnnotationsRequest,
        AnnotationsResponse,
    )

    from destack._generated.protocol.query.navigation.call_hierarchy import (
        CallHierarchyIncomingRequest,
        CallHierarchyIncomingResponse,
        CallHierarchyItemRequest,
        CallHierarchyItemResponse,
        CallHierarchyOutgoingRequest,
        CallHierarchyOutgoingResponse,
    )

    from destack._generated.protocol.query.navigation.definition import (
        GotoDeclarationRequest,
        GotoDeclarationResponse,
        GotoDefinitionRequest,
        GotoDefinitionResponse,
        GotoTypeDefinitionRequest,
        GotoTypeDefinitionResponse,
    )

    from destack._generated.protocol.query.navigation.document_link import (
        DocumentLinksRequest,
        DocumentLinksResponse,
    )

    from destack._generated.protocol.query.navigation.document_symbol import (
        DocumentSymbolsRequest,
        DocumentSymbolsResponse,
    )

    from destack._generated.protocol.query.navigation.find_references import (
        FindReferencesRequest,
        FindReferencesResponse,
    )

    from destack._generated.protocol.query.navigation.highlight import (
        DocumentHighlightRequest,
        DocumentHighlightResponse,
    )

    from destack._generated.protocol.query.navigation.implementation import (
        GotoImplementationRequest,
        GotoImplementationResponse,
    )

    from destack._generated.protocol.query.navigation.selection_range import (
        SelectionRangesRequest,
        SelectionRangesResponse,
    )

    from destack._generated.protocol.query.navigation.type_hierarchy import (
        TypeHierarchyItemRequest,
        TypeHierarchyItemResponse,
        TypeHierarchySubtypesRequest,
        TypeHierarchySubtypesResponse,
        TypeHierarchySupertypesRequest,
        TypeHierarchySupertypesResponse,
    )

    from destack._generated.protocol.query.navigation.workspace_symbol import (
        WorkspaceSymbolsRequest,
        WorkspaceSymbolsResponse,
    )

    from destack._generated.protocol.query.refactor.change_signature import (
        ChangeSignatureRequest,
        ChangeSignatureResponse,
    )

    from destack._generated.protocol.query.refactor.code_action import (
        CodeActionsRequest,
        CodeActionsResponse,
    )

    from destack._generated.protocol.query.refactor.extract_function import (
        ExtractFunctionRequest,
        ExtractFunctionResponse,
    )

    from destack._generated.protocol.query.refactor.extract_variable import (
        ExtractVariableRequest,
        ExtractVariableResponse,
    )

    from destack._generated.protocol.query.refactor.file_rename import (
        RenameFilesRequest,
        RenameFilesResponse,
    )

    from destack._generated.protocol.query.refactor.inline import (
        InlineRequest,
        InlineResponse,
    )

    from destack._generated.protocol.query.refactor.rename import (
        RenameRequest,
        RenameResponse,
        RenameTargetRequest,
        RenameTargetResponse,
    )


@dataclass(frozen=True, slots=True)
class QueryRequestCompletion:
    """Completion request payload."""

    completion: CompletionRequest
    kind: Literal["completion"] = "completion"


@dataclass(frozen=True, slots=True)
class QueryRequestHover:
    """Hover request payload."""

    hover: HoverRequest
    kind: Literal["hover"] = "hover"


@dataclass(frozen=True, slots=True)
class QueryRequestSignatureHelp:
    """Signature help request payload."""

    signature_help: SignatureHelpRequest
    kind: Literal["signatureHelp"] = "signatureHelp"


@dataclass(frozen=True, slots=True)
class QueryRequestInlayHints:
    """Inlay hints request payload."""

    inlay_hints: InlayHintsRequest
    kind: Literal["inlayHints"] = "inlayHints"


@dataclass(frozen=True, slots=True)
class QueryRequestCodeLenses:
    """Code lenses request payload."""

    code_lenses: CodeLensesRequest
    kind: Literal["codeLenses"] = "codeLenses"


@dataclass(frozen=True, slots=True)
class QueryRequestResolveCodeLens:
    """Code lens resolve request payload."""

    resolve_code_lens: ResolveCodeLensRequest
    kind: Literal["resolveCodeLens"] = "resolveCodeLens"


@dataclass(frozen=True, slots=True)
class QueryRequestFoldingRanges:
    """Folding ranges request payload."""

    folding_ranges: FoldingRangesRequest
    kind: Literal["foldingRanges"] = "foldingRanges"


@dataclass(frozen=True, slots=True)
class QueryRequestSemanticTokens:
    """Full-document semantic tokens request payload."""

    semantic_tokens: SemanticTokensRequest
    kind: Literal["semanticTokens"] = "semanticTokens"


@dataclass(frozen=True, slots=True)
class QueryRequestSemanticTokensRange:
    """Range semantic tokens request payload."""

    semantic_tokens_range: SemanticTokensRangeRequest
    kind: Literal["semanticTokensRange"] = "semanticTokensRange"


@dataclass(frozen=True, slots=True)
class QueryRequestDocumentSymbols:
    """Document symbols request payload."""

    document_symbols: DocumentSymbolsRequest
    kind: Literal["documentSymbols"] = "documentSymbols"


@dataclass(frozen=True, slots=True)
class QueryRequestWorkspaceSymbols:
    """Workspace symbols request payload."""

    workspace_symbols: WorkspaceSymbolsRequest
    kind: Literal["workspaceSymbols"] = "workspaceSymbols"


@dataclass(frozen=True, slots=True)
class QueryRequestDocumentLinks:
    """Document links request payload."""

    document_links: DocumentLinksRequest
    kind: Literal["documentLinks"] = "documentLinks"


@dataclass(frozen=True, slots=True)
class QueryRequestDocumentHighlight:
    """Document highlight request payload."""

    document_highlight: DocumentHighlightRequest
    kind: Literal["documentHighlight"] = "documentHighlight"


@dataclass(frozen=True, slots=True)
class QueryRequestSelectionRanges:
    """Selection ranges request payload."""

    selection_ranges: SelectionRangesRequest
    kind: Literal["selectionRanges"] = "selectionRanges"


@dataclass(frozen=True, slots=True)
class QueryRequestGotoDefinition:
    """Goto definition request payload."""

    goto_definition: GotoDefinitionRequest
    kind: Literal["gotoDefinition"] = "gotoDefinition"


@dataclass(frozen=True, slots=True)
class QueryRequestGotoDeclaration:
    """Goto declaration request payload."""

    goto_declaration: GotoDeclarationRequest
    kind: Literal["gotoDeclaration"] = "gotoDeclaration"


@dataclass(frozen=True, slots=True)
class QueryRequestGotoTypeDefinition:
    """Goto type definition request payload."""

    goto_type_definition: GotoTypeDefinitionRequest
    kind: Literal["gotoTypeDefinition"] = "gotoTypeDefinition"


@dataclass(frozen=True, slots=True)
class QueryRequestGotoImplementation:
    """Goto implementation request payload."""

    goto_implementation: GotoImplementationRequest
    kind: Literal["gotoImplementation"] = "gotoImplementation"


@dataclass(frozen=True, slots=True)
class QueryRequestFindReferences:
    """Find references request payload."""

    find_references: FindReferencesRequest
    kind: Literal["findReferences"] = "findReferences"


@dataclass(frozen=True, slots=True)
class QueryRequestCallHierarchyItem:
    """Call hierarchy item request payload."""

    call_hierarchy_item: CallHierarchyItemRequest
    kind: Literal["callHierarchyItem"] = "callHierarchyItem"


@dataclass(frozen=True, slots=True)
class QueryRequestCallHierarchyIncoming:
    """Incoming call hierarchy request payload."""

    call_hierarchy_incoming: CallHierarchyIncomingRequest
    kind: Literal["callHierarchyIncoming"] = "callHierarchyIncoming"


@dataclass(frozen=True, slots=True)
class QueryRequestCallHierarchyOutgoing:
    """Outgoing call hierarchy request payload."""

    call_hierarchy_outgoing: CallHierarchyOutgoingRequest
    kind: Literal["callHierarchyOutgoing"] = "callHierarchyOutgoing"


@dataclass(frozen=True, slots=True)
class QueryRequestTypeHierarchyItem:
    """Type hierarchy item request payload."""

    type_hierarchy_item: TypeHierarchyItemRequest
    kind: Literal["typeHierarchyItem"] = "typeHierarchyItem"


@dataclass(frozen=True, slots=True)
class QueryRequestTypeHierarchySupertypes:
    """Type hierarchy supertypes request payload."""

    type_hierarchy_supertypes: TypeHierarchySupertypesRequest
    kind: Literal["typeHierarchySupertypes"] = "typeHierarchySupertypes"


@dataclass(frozen=True, slots=True)
class QueryRequestTypeHierarchySubtypes:
    """Type hierarchy subtypes request payload."""

    type_hierarchy_subtypes: TypeHierarchySubtypesRequest
    kind: Literal["typeHierarchySubtypes"] = "typeHierarchySubtypes"


@dataclass(frozen=True, slots=True)
class QueryRequestAnnotations:
    """Annotation request payload."""

    annotations: AnnotationsRequest
    kind: Literal["annotations"] = "annotations"


@dataclass(frozen=True, slots=True)
class QueryRequestRenameTarget:
    """Rename target request payload."""

    rename_target: RenameTargetRequest
    kind: Literal["renameTarget"] = "renameTarget"


@dataclass(frozen=True, slots=True)
class QueryRequestRename:
    """Rename request payload."""

    rename: RenameRequest
    kind: Literal["rename"] = "rename"


@dataclass(frozen=True, slots=True)
class QueryRequestRenameFiles:
    """Rename files request payload."""

    rename_files: RenameFilesRequest
    kind: Literal["renameFiles"] = "renameFiles"


@dataclass(frozen=True, slots=True)
class QueryRequestExtractFunction:
    """Extract function request payload."""

    extract_function: ExtractFunctionRequest
    kind: Literal["extractFunction"] = "extractFunction"


@dataclass(frozen=True, slots=True)
class QueryRequestExtractVariable:
    """Extract variable request payload."""

    extract_variable: ExtractVariableRequest
    kind: Literal["extractVariable"] = "extractVariable"


@dataclass(frozen=True, slots=True)
class QueryRequestInline:
    """Inline request payload."""

    inline: InlineRequest
    kind: Literal["inline"] = "inline"


@dataclass(frozen=True, slots=True)
class QueryRequestChangeSignature:
    """Change signature request payload."""

    change_signature: ChangeSignatureRequest
    kind: Literal["changeSignature"] = "changeSignature"


@dataclass(frozen=True, slots=True)
class QueryRequestCodeActions:
    """Code actions request payload."""

    code_actions: CodeActionsRequest
    kind: Literal["codeActions"] = "codeActions"


"""Query request payload."""
QueryRequest: TypeAlias = (
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


def encode_query_request(writer: Writer, value: QueryRequest) -> None:
    if value.kind == "completion":
        writer.write_unsigned(0)
        destack._generated.protocol.query.assist.completion.encode_completion_request(
            writer, value.completion
        )
    elif value.kind == "hover":
        writer.write_unsigned(1)
        destack._generated.protocol.query.assist.hover.encode_hover_request(
            writer, value.hover
        )
    elif value.kind == "signatureHelp":
        writer.write_unsigned(2)
        destack._generated.protocol.query.assist.signature.encode_signature_help_request(
            writer, value.signature_help
        )
    elif value.kind == "inlayHints":
        writer.write_unsigned(3)
        destack._generated.protocol.query.assist.inlay.encode_inlay_hints_request(
            writer, value.inlay_hints
        )
    elif value.kind == "codeLenses":
        writer.write_unsigned(4)
        destack._generated.protocol.query.assist.lens.encode_code_lenses_request(
            writer, value.code_lenses
        )
    elif value.kind == "resolveCodeLens":
        writer.write_unsigned(5)
        destack._generated.protocol.query.assist.lens.encode_resolve_code_lens_request(
            writer, value.resolve_code_lens
        )
    elif value.kind == "foldingRanges":
        writer.write_unsigned(6)
        destack._generated.protocol.query.assist.folding.encode_folding_ranges_request(
            writer, value.folding_ranges
        )
    elif value.kind == "semanticTokens":
        writer.write_unsigned(7)
        destack._generated.protocol.query.assist.semantic.encode_semantic_tokens_request(
            writer, value.semantic_tokens
        )
    elif value.kind == "semanticTokensRange":
        writer.write_unsigned(8)
        destack._generated.protocol.query.assist.semantic.encode_semantic_tokens_range_request(
            writer, value.semantic_tokens_range
        )
    elif value.kind == "documentSymbols":
        writer.write_unsigned(9)
        destack._generated.protocol.query.navigation.document_symbol.encode_document_symbols_request(
            writer, value.document_symbols
        )
    elif value.kind == "workspaceSymbols":
        writer.write_unsigned(10)
        destack._generated.protocol.query.navigation.workspace_symbol.encode_workspace_symbols_request(
            writer, value.workspace_symbols
        )
    elif value.kind == "documentLinks":
        writer.write_unsigned(11)
        destack._generated.protocol.query.navigation.document_link.encode_document_links_request(
            writer, value.document_links
        )
    elif value.kind == "documentHighlight":
        writer.write_unsigned(12)
        destack._generated.protocol.query.navigation.highlight.encode_document_highlight_request(
            writer, value.document_highlight
        )
    elif value.kind == "selectionRanges":
        writer.write_unsigned(13)
        destack._generated.protocol.query.navigation.selection_range.encode_selection_ranges_request(
            writer, value.selection_ranges
        )
    elif value.kind == "gotoDefinition":
        writer.write_unsigned(14)
        destack._generated.protocol.query.navigation.definition.encode_goto_definition_request(
            writer, value.goto_definition
        )
    elif value.kind == "gotoDeclaration":
        writer.write_unsigned(15)
        destack._generated.protocol.query.navigation.definition.encode_goto_declaration_request(
            writer, value.goto_declaration
        )
    elif value.kind == "gotoTypeDefinition":
        writer.write_unsigned(16)
        destack._generated.protocol.query.navigation.definition.encode_goto_type_definition_request(
            writer, value.goto_type_definition
        )
    elif value.kind == "gotoImplementation":
        writer.write_unsigned(17)
        destack._generated.protocol.query.navigation.implementation.encode_goto_implementation_request(
            writer, value.goto_implementation
        )
    elif value.kind == "findReferences":
        writer.write_unsigned(18)
        destack._generated.protocol.query.navigation.find_references.encode_find_references_request(
            writer, value.find_references
        )
    elif value.kind == "callHierarchyItem":
        writer.write_unsigned(19)
        destack._generated.protocol.query.navigation.call_hierarchy.encode_call_hierarchy_item_request(
            writer, value.call_hierarchy_item
        )
    elif value.kind == "callHierarchyIncoming":
        writer.write_unsigned(20)
        destack._generated.protocol.query.navigation.call_hierarchy.encode_call_hierarchy_incoming_request(
            writer, value.call_hierarchy_incoming
        )
    elif value.kind == "callHierarchyOutgoing":
        writer.write_unsigned(21)
        destack._generated.protocol.query.navigation.call_hierarchy.encode_call_hierarchy_outgoing_request(
            writer, value.call_hierarchy_outgoing
        )
    elif value.kind == "typeHierarchyItem":
        writer.write_unsigned(22)
        destack._generated.protocol.query.navigation.type_hierarchy.encode_type_hierarchy_item_request(
            writer, value.type_hierarchy_item
        )
    elif value.kind == "typeHierarchySupertypes":
        writer.write_unsigned(23)
        destack._generated.protocol.query.navigation.type_hierarchy.encode_type_hierarchy_supertypes_request(
            writer, value.type_hierarchy_supertypes
        )
    elif value.kind == "typeHierarchySubtypes":
        writer.write_unsigned(24)
        destack._generated.protocol.query.navigation.type_hierarchy.encode_type_hierarchy_subtypes_request(
            writer, value.type_hierarchy_subtypes
        )
    elif value.kind == "annotations":
        writer.write_unsigned(25)
        destack._generated.protocol.query.navigation.annotation.encode_annotations_request(
            writer, value.annotations
        )
    elif value.kind == "renameTarget":
        writer.write_unsigned(26)
        destack._generated.protocol.query.refactor.rename.encode_rename_target_request(
            writer, value.rename_target
        )
    elif value.kind == "rename":
        writer.write_unsigned(27)
        destack._generated.protocol.query.refactor.rename.encode_rename_request(
            writer, value.rename
        )
    elif value.kind == "renameFiles":
        writer.write_unsigned(28)
        destack._generated.protocol.query.refactor.file_rename.encode_rename_files_request(
            writer, value.rename_files
        )
    elif value.kind == "extractFunction":
        writer.write_unsigned(29)
        destack._generated.protocol.query.refactor.extract_function.encode_extract_function_request(
            writer, value.extract_function
        )
    elif value.kind == "extractVariable":
        writer.write_unsigned(30)
        destack._generated.protocol.query.refactor.extract_variable.encode_extract_variable_request(
            writer, value.extract_variable
        )
    elif value.kind == "inline":
        writer.write_unsigned(31)
        destack._generated.protocol.query.refactor.inline.encode_inline_request(
            writer, value.inline
        )
    elif value.kind == "changeSignature":
        writer.write_unsigned(32)
        destack._generated.protocol.query.refactor.change_signature.encode_change_signature_request(
            writer, value.change_signature
        )
    elif value.kind == "codeActions":
        writer.write_unsigned(33)
        destack._generated.protocol.query.refactor.code_action.encode_code_actions_request(
            writer, value.code_actions
        )
    else:
        raise SerdeError("unknown enum variant")


def decode_query_request(reader: Reader) -> QueryRequest:
    variant = reader.read_number()

    if variant == 0:
        return QueryRequestCompletion(
            completion=destack._generated.protocol.query.assist.completion.decode_completion_request(
                reader
            )
        )
    elif variant == 1:
        return QueryRequestHover(
            hover=destack._generated.protocol.query.assist.hover.decode_hover_request(
                reader
            )
        )
    elif variant == 2:
        return QueryRequestSignatureHelp(
            signature_help=destack._generated.protocol.query.assist.signature.decode_signature_help_request(
                reader
            )
        )
    elif variant == 3:
        return QueryRequestInlayHints(
            inlay_hints=destack._generated.protocol.query.assist.inlay.decode_inlay_hints_request(
                reader
            )
        )
    elif variant == 4:
        return QueryRequestCodeLenses(
            code_lenses=destack._generated.protocol.query.assist.lens.decode_code_lenses_request(
                reader
            )
        )
    elif variant == 5:
        return QueryRequestResolveCodeLens(
            resolve_code_lens=destack._generated.protocol.query.assist.lens.decode_resolve_code_lens_request(
                reader
            )
        )
    elif variant == 6:
        return QueryRequestFoldingRanges(
            folding_ranges=destack._generated.protocol.query.assist.folding.decode_folding_ranges_request(
                reader
            )
        )
    elif variant == 7:
        return QueryRequestSemanticTokens(
            semantic_tokens=destack._generated.protocol.query.assist.semantic.decode_semantic_tokens_request(
                reader
            )
        )
    elif variant == 8:
        return QueryRequestSemanticTokensRange(
            semantic_tokens_range=destack._generated.protocol.query.assist.semantic.decode_semantic_tokens_range_request(
                reader
            )
        )
    elif variant == 9:
        return QueryRequestDocumentSymbols(
            document_symbols=destack._generated.protocol.query.navigation.document_symbol.decode_document_symbols_request(
                reader
            )
        )
    elif variant == 10:
        return QueryRequestWorkspaceSymbols(
            workspace_symbols=destack._generated.protocol.query.navigation.workspace_symbol.decode_workspace_symbols_request(
                reader
            )
        )
    elif variant == 11:
        return QueryRequestDocumentLinks(
            document_links=destack._generated.protocol.query.navigation.document_link.decode_document_links_request(
                reader
            )
        )
    elif variant == 12:
        return QueryRequestDocumentHighlight(
            document_highlight=destack._generated.protocol.query.navigation.highlight.decode_document_highlight_request(
                reader
            )
        )
    elif variant == 13:
        return QueryRequestSelectionRanges(
            selection_ranges=destack._generated.protocol.query.navigation.selection_range.decode_selection_ranges_request(
                reader
            )
        )
    elif variant == 14:
        return QueryRequestGotoDefinition(
            goto_definition=destack._generated.protocol.query.navigation.definition.decode_goto_definition_request(
                reader
            )
        )
    elif variant == 15:
        return QueryRequestGotoDeclaration(
            goto_declaration=destack._generated.protocol.query.navigation.definition.decode_goto_declaration_request(
                reader
            )
        )
    elif variant == 16:
        return QueryRequestGotoTypeDefinition(
            goto_type_definition=destack._generated.protocol.query.navigation.definition.decode_goto_type_definition_request(
                reader
            )
        )
    elif variant == 17:
        return QueryRequestGotoImplementation(
            goto_implementation=destack._generated.protocol.query.navigation.implementation.decode_goto_implementation_request(
                reader
            )
        )
    elif variant == 18:
        return QueryRequestFindReferences(
            find_references=destack._generated.protocol.query.navigation.find_references.decode_find_references_request(
                reader
            )
        )
    elif variant == 19:
        return QueryRequestCallHierarchyItem(
            call_hierarchy_item=destack._generated.protocol.query.navigation.call_hierarchy.decode_call_hierarchy_item_request(
                reader
            )
        )
    elif variant == 20:
        return QueryRequestCallHierarchyIncoming(
            call_hierarchy_incoming=destack._generated.protocol.query.navigation.call_hierarchy.decode_call_hierarchy_incoming_request(
                reader
            )
        )
    elif variant == 21:
        return QueryRequestCallHierarchyOutgoing(
            call_hierarchy_outgoing=destack._generated.protocol.query.navigation.call_hierarchy.decode_call_hierarchy_outgoing_request(
                reader
            )
        )
    elif variant == 22:
        return QueryRequestTypeHierarchyItem(
            type_hierarchy_item=destack._generated.protocol.query.navigation.type_hierarchy.decode_type_hierarchy_item_request(
                reader
            )
        )
    elif variant == 23:
        return QueryRequestTypeHierarchySupertypes(
            type_hierarchy_supertypes=destack._generated.protocol.query.navigation.type_hierarchy.decode_type_hierarchy_supertypes_request(
                reader
            )
        )
    elif variant == 24:
        return QueryRequestTypeHierarchySubtypes(
            type_hierarchy_subtypes=destack._generated.protocol.query.navigation.type_hierarchy.decode_type_hierarchy_subtypes_request(
                reader
            )
        )
    elif variant == 25:
        return QueryRequestAnnotations(
            annotations=destack._generated.protocol.query.navigation.annotation.decode_annotations_request(
                reader
            )
        )
    elif variant == 26:
        return QueryRequestRenameTarget(
            rename_target=destack._generated.protocol.query.refactor.rename.decode_rename_target_request(
                reader
            )
        )
    elif variant == 27:
        return QueryRequestRename(
            rename=destack._generated.protocol.query.refactor.rename.decode_rename_request(
                reader
            )
        )
    elif variant == 28:
        return QueryRequestRenameFiles(
            rename_files=destack._generated.protocol.query.refactor.file_rename.decode_rename_files_request(
                reader
            )
        )
    elif variant == 29:
        return QueryRequestExtractFunction(
            extract_function=destack._generated.protocol.query.refactor.extract_function.decode_extract_function_request(
                reader
            )
        )
    elif variant == 30:
        return QueryRequestExtractVariable(
            extract_variable=destack._generated.protocol.query.refactor.extract_variable.decode_extract_variable_request(
                reader
            )
        )
    elif variant == 31:
        return QueryRequestInline(
            inline=destack._generated.protocol.query.refactor.inline.decode_inline_request(
                reader
            )
        )
    elif variant == 32:
        return QueryRequestChangeSignature(
            change_signature=destack._generated.protocol.query.refactor.change_signature.decode_change_signature_request(
                reader
            )
        )
    elif variant == 33:
        return QueryRequestCodeActions(
            code_actions=destack._generated.protocol.query.refactor.code_action.decode_code_actions_request(
                reader
            )
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


@dataclass(frozen=True, slots=True)
class QueryResponseCompletion:
    """Completion response payload."""

    completion: CompletionResponse
    kind: Literal["completion"] = "completion"


@dataclass(frozen=True, slots=True)
class QueryResponseHover:
    """Hover response payload."""

    hover: HoverResponse
    kind: Literal["hover"] = "hover"


@dataclass(frozen=True, slots=True)
class QueryResponseSignatureHelp:
    """Signature help response payload."""

    signature_help: SignatureHelpResponse
    kind: Literal["signatureHelp"] = "signatureHelp"


@dataclass(frozen=True, slots=True)
class QueryResponseInlayHints:
    """Inlay hints response payload."""

    inlay_hints: InlayHintsResponse
    kind: Literal["inlayHints"] = "inlayHints"


@dataclass(frozen=True, slots=True)
class QueryResponseCodeLenses:
    """Code lenses response payload."""

    code_lenses: CodeLensesResponse
    kind: Literal["codeLenses"] = "codeLenses"


@dataclass(frozen=True, slots=True)
class QueryResponseResolveCodeLens:
    """Code lens resolve response payload."""

    resolve_code_lens: ResolveCodeLensResponse
    kind: Literal["resolveCodeLens"] = "resolveCodeLens"


@dataclass(frozen=True, slots=True)
class QueryResponseFoldingRanges:
    """Folding ranges response payload."""

    folding_ranges: FoldingRangesResponse
    kind: Literal["foldingRanges"] = "foldingRanges"


@dataclass(frozen=True, slots=True)
class QueryResponseSemanticTokens:
    """Full-document semantic tokens response payload."""

    semantic_tokens: SemanticTokensResponse
    kind: Literal["semanticTokens"] = "semanticTokens"


@dataclass(frozen=True, slots=True)
class QueryResponseSemanticTokensRange:
    """Range semantic tokens response payload."""

    semantic_tokens_range: SemanticTokensResponse
    kind: Literal["semanticTokensRange"] = "semanticTokensRange"


@dataclass(frozen=True, slots=True)
class QueryResponseDocumentSymbols:
    """Document symbols response payload."""

    document_symbols: DocumentSymbolsResponse
    kind: Literal["documentSymbols"] = "documentSymbols"


@dataclass(frozen=True, slots=True)
class QueryResponseWorkspaceSymbols:
    """Workspace symbols response payload."""

    workspace_symbols: WorkspaceSymbolsResponse
    kind: Literal["workspaceSymbols"] = "workspaceSymbols"


@dataclass(frozen=True, slots=True)
class QueryResponseDocumentLinks:
    """Document links response payload."""

    document_links: DocumentLinksResponse
    kind: Literal["documentLinks"] = "documentLinks"


@dataclass(frozen=True, slots=True)
class QueryResponseDocumentHighlight:
    """Document highlight response payload."""

    document_highlight: DocumentHighlightResponse
    kind: Literal["documentHighlight"] = "documentHighlight"


@dataclass(frozen=True, slots=True)
class QueryResponseSelectionRanges:
    """Selection ranges response payload."""

    selection_ranges: SelectionRangesResponse
    kind: Literal["selectionRanges"] = "selectionRanges"


@dataclass(frozen=True, slots=True)
class QueryResponseGotoDefinition:
    """Goto definition response payload."""

    goto_definition: GotoDefinitionResponse
    kind: Literal["gotoDefinition"] = "gotoDefinition"


@dataclass(frozen=True, slots=True)
class QueryResponseGotoDeclaration:
    """Goto declaration response payload."""

    goto_declaration: GotoDeclarationResponse
    kind: Literal["gotoDeclaration"] = "gotoDeclaration"


@dataclass(frozen=True, slots=True)
class QueryResponseGotoTypeDefinition:
    """Goto type definition response payload."""

    goto_type_definition: GotoTypeDefinitionResponse
    kind: Literal["gotoTypeDefinition"] = "gotoTypeDefinition"


@dataclass(frozen=True, slots=True)
class QueryResponseGotoImplementation:
    """Goto implementation response payload."""

    goto_implementation: GotoImplementationResponse
    kind: Literal["gotoImplementation"] = "gotoImplementation"


@dataclass(frozen=True, slots=True)
class QueryResponseFindReferences:
    """Find references response payload."""

    find_references: FindReferencesResponse
    kind: Literal["findReferences"] = "findReferences"


@dataclass(frozen=True, slots=True)
class QueryResponseCallHierarchyItem:
    """Call hierarchy item response payload."""

    call_hierarchy_item: CallHierarchyItemResponse
    kind: Literal["callHierarchyItem"] = "callHierarchyItem"


@dataclass(frozen=True, slots=True)
class QueryResponseCallHierarchyIncoming:
    """Incoming call hierarchy response payload."""

    call_hierarchy_incoming: CallHierarchyIncomingResponse
    kind: Literal["callHierarchyIncoming"] = "callHierarchyIncoming"


@dataclass(frozen=True, slots=True)
class QueryResponseCallHierarchyOutgoing:
    """Outgoing call hierarchy response payload."""

    call_hierarchy_outgoing: CallHierarchyOutgoingResponse
    kind: Literal["callHierarchyOutgoing"] = "callHierarchyOutgoing"


@dataclass(frozen=True, slots=True)
class QueryResponseTypeHierarchyItem:
    """Type hierarchy item response payload."""

    type_hierarchy_item: TypeHierarchyItemResponse
    kind: Literal["typeHierarchyItem"] = "typeHierarchyItem"


@dataclass(frozen=True, slots=True)
class QueryResponseTypeHierarchySupertypes:
    """Type hierarchy supertypes response payload."""

    type_hierarchy_supertypes: TypeHierarchySupertypesResponse
    kind: Literal["typeHierarchySupertypes"] = "typeHierarchySupertypes"


@dataclass(frozen=True, slots=True)
class QueryResponseTypeHierarchySubtypes:
    """Type hierarchy subtypes response payload."""

    type_hierarchy_subtypes: TypeHierarchySubtypesResponse
    kind: Literal["typeHierarchySubtypes"] = "typeHierarchySubtypes"


@dataclass(frozen=True, slots=True)
class QueryResponseAnnotations:
    """Annotation response payload."""

    annotations: AnnotationsResponse
    kind: Literal["annotations"] = "annotations"


@dataclass(frozen=True, slots=True)
class QueryResponseRenameTarget:
    """Rename target response payload."""

    rename_target: RenameTargetResponse
    kind: Literal["renameTarget"] = "renameTarget"


@dataclass(frozen=True, slots=True)
class QueryResponseRename:
    """Rename response payload."""

    rename: RenameResponse
    kind: Literal["rename"] = "rename"


@dataclass(frozen=True, slots=True)
class QueryResponseRenameFiles:
    """Rename files response payload."""

    rename_files: RenameFilesResponse
    kind: Literal["renameFiles"] = "renameFiles"


@dataclass(frozen=True, slots=True)
class QueryResponseExtractFunction:
    """Extract function response payload."""

    extract_function: ExtractFunctionResponse
    kind: Literal["extractFunction"] = "extractFunction"


@dataclass(frozen=True, slots=True)
class QueryResponseExtractVariable:
    """Extract variable response payload."""

    extract_variable: ExtractVariableResponse
    kind: Literal["extractVariable"] = "extractVariable"


@dataclass(frozen=True, slots=True)
class QueryResponseInline:
    """Inline response payload."""

    inline: InlineResponse
    kind: Literal["inline"] = "inline"


@dataclass(frozen=True, slots=True)
class QueryResponseChangeSignature:
    """Change signature response payload."""

    change_signature: ChangeSignatureResponse
    kind: Literal["changeSignature"] = "changeSignature"


@dataclass(frozen=True, slots=True)
class QueryResponseCodeActions:
    """Code actions response payload."""

    code_actions: CodeActionsResponse
    kind: Literal["codeActions"] = "codeActions"


"""Query response payload."""
QueryResponse: TypeAlias = (
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


def encode_query_response(writer: Writer, value: QueryResponse) -> None:
    if value.kind == "completion":
        writer.write_unsigned(0)
        destack._generated.protocol.query.assist.completion.encode_completion_response(
            writer, value.completion
        )
    elif value.kind == "hover":
        writer.write_unsigned(1)
        destack._generated.protocol.query.assist.hover.encode_hover_response(
            writer, value.hover
        )
    elif value.kind == "signatureHelp":
        writer.write_unsigned(2)
        destack._generated.protocol.query.assist.signature.encode_signature_help_response(
            writer, value.signature_help
        )
    elif value.kind == "inlayHints":
        writer.write_unsigned(3)
        destack._generated.protocol.query.assist.inlay.encode_inlay_hints_response(
            writer, value.inlay_hints
        )
    elif value.kind == "codeLenses":
        writer.write_unsigned(4)
        destack._generated.protocol.query.assist.lens.encode_code_lenses_response(
            writer, value.code_lenses
        )
    elif value.kind == "resolveCodeLens":
        writer.write_unsigned(5)
        destack._generated.protocol.query.assist.lens.encode_resolve_code_lens_response(
            writer, value.resolve_code_lens
        )
    elif value.kind == "foldingRanges":
        writer.write_unsigned(6)
        destack._generated.protocol.query.assist.folding.encode_folding_ranges_response(
            writer, value.folding_ranges
        )
    elif value.kind == "semanticTokens":
        writer.write_unsigned(7)
        destack._generated.protocol.query.assist.semantic.encode_semantic_tokens_response(
            writer, value.semantic_tokens
        )
    elif value.kind == "semanticTokensRange":
        writer.write_unsigned(8)
        destack._generated.protocol.query.assist.semantic.encode_semantic_tokens_response(
            writer, value.semantic_tokens_range
        )
    elif value.kind == "documentSymbols":
        writer.write_unsigned(9)
        destack._generated.protocol.query.navigation.document_symbol.encode_document_symbols_response(
            writer, value.document_symbols
        )
    elif value.kind == "workspaceSymbols":
        writer.write_unsigned(10)
        destack._generated.protocol.query.navigation.workspace_symbol.encode_workspace_symbols_response(
            writer, value.workspace_symbols
        )
    elif value.kind == "documentLinks":
        writer.write_unsigned(11)
        destack._generated.protocol.query.navigation.document_link.encode_document_links_response(
            writer, value.document_links
        )
    elif value.kind == "documentHighlight":
        writer.write_unsigned(12)
        destack._generated.protocol.query.navigation.highlight.encode_document_highlight_response(
            writer, value.document_highlight
        )
    elif value.kind == "selectionRanges":
        writer.write_unsigned(13)
        destack._generated.protocol.query.navigation.selection_range.encode_selection_ranges_response(
            writer, value.selection_ranges
        )
    elif value.kind == "gotoDefinition":
        writer.write_unsigned(14)
        destack._generated.protocol.query.navigation.definition.encode_goto_definition_response(
            writer, value.goto_definition
        )
    elif value.kind == "gotoDeclaration":
        writer.write_unsigned(15)
        destack._generated.protocol.query.navigation.definition.encode_goto_declaration_response(
            writer, value.goto_declaration
        )
    elif value.kind == "gotoTypeDefinition":
        writer.write_unsigned(16)
        destack._generated.protocol.query.navigation.definition.encode_goto_type_definition_response(
            writer, value.goto_type_definition
        )
    elif value.kind == "gotoImplementation":
        writer.write_unsigned(17)
        destack._generated.protocol.query.navigation.implementation.encode_goto_implementation_response(
            writer, value.goto_implementation
        )
    elif value.kind == "findReferences":
        writer.write_unsigned(18)
        destack._generated.protocol.query.navigation.find_references.encode_find_references_response(
            writer, value.find_references
        )
    elif value.kind == "callHierarchyItem":
        writer.write_unsigned(19)
        destack._generated.protocol.query.navigation.call_hierarchy.encode_call_hierarchy_item_response(
            writer, value.call_hierarchy_item
        )
    elif value.kind == "callHierarchyIncoming":
        writer.write_unsigned(20)
        destack._generated.protocol.query.navigation.call_hierarchy.encode_call_hierarchy_incoming_response(
            writer, value.call_hierarchy_incoming
        )
    elif value.kind == "callHierarchyOutgoing":
        writer.write_unsigned(21)
        destack._generated.protocol.query.navigation.call_hierarchy.encode_call_hierarchy_outgoing_response(
            writer, value.call_hierarchy_outgoing
        )
    elif value.kind == "typeHierarchyItem":
        writer.write_unsigned(22)
        destack._generated.protocol.query.navigation.type_hierarchy.encode_type_hierarchy_item_response(
            writer, value.type_hierarchy_item
        )
    elif value.kind == "typeHierarchySupertypes":
        writer.write_unsigned(23)
        destack._generated.protocol.query.navigation.type_hierarchy.encode_type_hierarchy_supertypes_response(
            writer, value.type_hierarchy_supertypes
        )
    elif value.kind == "typeHierarchySubtypes":
        writer.write_unsigned(24)
        destack._generated.protocol.query.navigation.type_hierarchy.encode_type_hierarchy_subtypes_response(
            writer, value.type_hierarchy_subtypes
        )
    elif value.kind == "annotations":
        writer.write_unsigned(25)
        destack._generated.protocol.query.navigation.annotation.encode_annotations_response(
            writer, value.annotations
        )
    elif value.kind == "renameTarget":
        writer.write_unsigned(26)
        destack._generated.protocol.query.refactor.rename.encode_rename_target_response(
            writer, value.rename_target
        )
    elif value.kind == "rename":
        writer.write_unsigned(27)
        destack._generated.protocol.query.refactor.rename.encode_rename_response(
            writer, value.rename
        )
    elif value.kind == "renameFiles":
        writer.write_unsigned(28)
        destack._generated.protocol.query.refactor.file_rename.encode_rename_files_response(
            writer, value.rename_files
        )
    elif value.kind == "extractFunction":
        writer.write_unsigned(29)
        destack._generated.protocol.query.refactor.extract_function.encode_extract_function_response(
            writer, value.extract_function
        )
    elif value.kind == "extractVariable":
        writer.write_unsigned(30)
        destack._generated.protocol.query.refactor.extract_variable.encode_extract_variable_response(
            writer, value.extract_variable
        )
    elif value.kind == "inline":
        writer.write_unsigned(31)
        destack._generated.protocol.query.refactor.inline.encode_inline_response(
            writer, value.inline
        )
    elif value.kind == "changeSignature":
        writer.write_unsigned(32)
        destack._generated.protocol.query.refactor.change_signature.encode_change_signature_response(
            writer, value.change_signature
        )
    elif value.kind == "codeActions":
        writer.write_unsigned(33)
        destack._generated.protocol.query.refactor.code_action.encode_code_actions_response(
            writer, value.code_actions
        )
    else:
        raise SerdeError("unknown enum variant")


def decode_query_response(reader: Reader) -> QueryResponse:
    variant = reader.read_number()

    if variant == 0:
        return QueryResponseCompletion(
            completion=destack._generated.protocol.query.assist.completion.decode_completion_response(
                reader
            )
        )
    elif variant == 1:
        return QueryResponseHover(
            hover=destack._generated.protocol.query.assist.hover.decode_hover_response(
                reader
            )
        )
    elif variant == 2:
        return QueryResponseSignatureHelp(
            signature_help=destack._generated.protocol.query.assist.signature.decode_signature_help_response(
                reader
            )
        )
    elif variant == 3:
        return QueryResponseInlayHints(
            inlay_hints=destack._generated.protocol.query.assist.inlay.decode_inlay_hints_response(
                reader
            )
        )
    elif variant == 4:
        return QueryResponseCodeLenses(
            code_lenses=destack._generated.protocol.query.assist.lens.decode_code_lenses_response(
                reader
            )
        )
    elif variant == 5:
        return QueryResponseResolveCodeLens(
            resolve_code_lens=destack._generated.protocol.query.assist.lens.decode_resolve_code_lens_response(
                reader
            )
        )
    elif variant == 6:
        return QueryResponseFoldingRanges(
            folding_ranges=destack._generated.protocol.query.assist.folding.decode_folding_ranges_response(
                reader
            )
        )
    elif variant == 7:
        return QueryResponseSemanticTokens(
            semantic_tokens=destack._generated.protocol.query.assist.semantic.decode_semantic_tokens_response(
                reader
            )
        )
    elif variant == 8:
        return QueryResponseSemanticTokensRange(
            semantic_tokens_range=destack._generated.protocol.query.assist.semantic.decode_semantic_tokens_response(
                reader
            )
        )
    elif variant == 9:
        return QueryResponseDocumentSymbols(
            document_symbols=destack._generated.protocol.query.navigation.document_symbol.decode_document_symbols_response(
                reader
            )
        )
    elif variant == 10:
        return QueryResponseWorkspaceSymbols(
            workspace_symbols=destack._generated.protocol.query.navigation.workspace_symbol.decode_workspace_symbols_response(
                reader
            )
        )
    elif variant == 11:
        return QueryResponseDocumentLinks(
            document_links=destack._generated.protocol.query.navigation.document_link.decode_document_links_response(
                reader
            )
        )
    elif variant == 12:
        return QueryResponseDocumentHighlight(
            document_highlight=destack._generated.protocol.query.navigation.highlight.decode_document_highlight_response(
                reader
            )
        )
    elif variant == 13:
        return QueryResponseSelectionRanges(
            selection_ranges=destack._generated.protocol.query.navigation.selection_range.decode_selection_ranges_response(
                reader
            )
        )
    elif variant == 14:
        return QueryResponseGotoDefinition(
            goto_definition=destack._generated.protocol.query.navigation.definition.decode_goto_definition_response(
                reader
            )
        )
    elif variant == 15:
        return QueryResponseGotoDeclaration(
            goto_declaration=destack._generated.protocol.query.navigation.definition.decode_goto_declaration_response(
                reader
            )
        )
    elif variant == 16:
        return QueryResponseGotoTypeDefinition(
            goto_type_definition=destack._generated.protocol.query.navigation.definition.decode_goto_type_definition_response(
                reader
            )
        )
    elif variant == 17:
        return QueryResponseGotoImplementation(
            goto_implementation=destack._generated.protocol.query.navigation.implementation.decode_goto_implementation_response(
                reader
            )
        )
    elif variant == 18:
        return QueryResponseFindReferences(
            find_references=destack._generated.protocol.query.navigation.find_references.decode_find_references_response(
                reader
            )
        )
    elif variant == 19:
        return QueryResponseCallHierarchyItem(
            call_hierarchy_item=destack._generated.protocol.query.navigation.call_hierarchy.decode_call_hierarchy_item_response(
                reader
            )
        )
    elif variant == 20:
        return QueryResponseCallHierarchyIncoming(
            call_hierarchy_incoming=destack._generated.protocol.query.navigation.call_hierarchy.decode_call_hierarchy_incoming_response(
                reader
            )
        )
    elif variant == 21:
        return QueryResponseCallHierarchyOutgoing(
            call_hierarchy_outgoing=destack._generated.protocol.query.navigation.call_hierarchy.decode_call_hierarchy_outgoing_response(
                reader
            )
        )
    elif variant == 22:
        return QueryResponseTypeHierarchyItem(
            type_hierarchy_item=destack._generated.protocol.query.navigation.type_hierarchy.decode_type_hierarchy_item_response(
                reader
            )
        )
    elif variant == 23:
        return QueryResponseTypeHierarchySupertypes(
            type_hierarchy_supertypes=destack._generated.protocol.query.navigation.type_hierarchy.decode_type_hierarchy_supertypes_response(
                reader
            )
        )
    elif variant == 24:
        return QueryResponseTypeHierarchySubtypes(
            type_hierarchy_subtypes=destack._generated.protocol.query.navigation.type_hierarchy.decode_type_hierarchy_subtypes_response(
                reader
            )
        )
    elif variant == 25:
        return QueryResponseAnnotations(
            annotations=destack._generated.protocol.query.navigation.annotation.decode_annotations_response(
                reader
            )
        )
    elif variant == 26:
        return QueryResponseRenameTarget(
            rename_target=destack._generated.protocol.query.refactor.rename.decode_rename_target_response(
                reader
            )
        )
    elif variant == 27:
        return QueryResponseRename(
            rename=destack._generated.protocol.query.refactor.rename.decode_rename_response(
                reader
            )
        )
    elif variant == 28:
        return QueryResponseRenameFiles(
            rename_files=destack._generated.protocol.query.refactor.file_rename.decode_rename_files_response(
                reader
            )
        )
    elif variant == 29:
        return QueryResponseExtractFunction(
            extract_function=destack._generated.protocol.query.refactor.extract_function.decode_extract_function_response(
                reader
            )
        )
    elif variant == 30:
        return QueryResponseExtractVariable(
            extract_variable=destack._generated.protocol.query.refactor.extract_variable.decode_extract_variable_response(
                reader
            )
        )
    elif variant == 31:
        return QueryResponseInline(
            inline=destack._generated.protocol.query.refactor.inline.decode_inline_response(
                reader
            )
        )
    elif variant == 32:
        return QueryResponseChangeSignature(
            change_signature=destack._generated.protocol.query.refactor.change_signature.decode_change_signature_response(
                reader
            )
        )
    elif variant == 33:
        return QueryResponseCodeActions(
            code_actions=destack._generated.protocol.query.refactor.code_action.decode_code_actions_response(
                reader
            )
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


__all__ = [
    "QueryRequest",
    "encode_query_request",
    "decode_query_request",
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
