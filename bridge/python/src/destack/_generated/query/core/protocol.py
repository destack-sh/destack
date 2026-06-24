# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_field,
    json_object,
    json_string,
)

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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request(self)


@dataclass(frozen=True, slots=True)
class QueryRequestHover:
    """Hover request payload."""

    hover: destack._generated.query.assist.hover.HoverRequest
    kind: typing.Literal["hover"] = "hover"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request(self)


@dataclass(frozen=True, slots=True)
class QueryRequestSignatureHelp:
    """Signature help request payload."""

    signature_help: destack._generated.query.assist.signature.SignatureHelpRequest
    kind: typing.Literal["signatureHelp"] = "signatureHelp"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request(self)


@dataclass(frozen=True, slots=True)
class QueryRequestInlayHints:
    """Inlay hints request payload."""

    inlay_hints: destack._generated.query.assist.inlay.InlayHintsRequest
    kind: typing.Literal["inlayHints"] = "inlayHints"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request(self)


@dataclass(frozen=True, slots=True)
class QueryRequestCodeLenses:
    """Code lenses request payload."""

    code_lenses: destack._generated.query.assist.lens.CodeLensesRequest
    kind: typing.Literal["codeLenses"] = "codeLenses"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request(self)


@dataclass(frozen=True, slots=True)
class QueryRequestResolveCodeLens:
    """Code lens resolve request payload."""

    resolve_code_lens: destack._generated.query.assist.lens.ResolveCodeLensRequest
    kind: typing.Literal["resolveCodeLens"] = "resolveCodeLens"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request(self)


@dataclass(frozen=True, slots=True)
class QueryRequestFoldingRanges:
    """Folding ranges request payload."""

    folding_ranges: destack._generated.query.assist.folding.FoldingRangesRequest
    kind: typing.Literal["foldingRanges"] = "foldingRanges"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request(self)


@dataclass(frozen=True, slots=True)
class QueryRequestSemanticTokens:
    """Full-document semantic tokens request payload."""

    semantic_tokens: destack._generated.query.assist.semantic.SemanticTokensRequest
    kind: typing.Literal["semanticTokens"] = "semanticTokens"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request(self)


@dataclass(frozen=True, slots=True)
class QueryRequestSemanticTokensRange:
    """Range semantic tokens request payload."""

    semantic_tokens_range: (
        destack._generated.query.assist.semantic.SemanticTokensRangeRequest
    )
    kind: typing.Literal["semanticTokensRange"] = "semanticTokensRange"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request(self)


@dataclass(frozen=True, slots=True)
class QueryRequestDocumentSymbols:
    """Document symbols request payload."""

    document_symbols: (
        destack._generated.query.navigation.document_symbol.DocumentSymbolsRequest
    )
    kind: typing.Literal["documentSymbols"] = "documentSymbols"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request(self)


@dataclass(frozen=True, slots=True)
class QueryRequestWorkspaceSymbols:
    """Workspace symbols request payload."""

    workspace_symbols: (
        destack._generated.query.navigation.workspace_symbol.WorkspaceSymbolsRequest
    )
    kind: typing.Literal["workspaceSymbols"] = "workspaceSymbols"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request(self)


@dataclass(frozen=True, slots=True)
class QueryRequestDocumentLinks:
    """Document links request payload."""

    document_links: (
        destack._generated.query.navigation.document_link.DocumentLinksRequest
    )
    kind: typing.Literal["documentLinks"] = "documentLinks"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request(self)


@dataclass(frozen=True, slots=True)
class QueryRequestDocumentHighlight:
    """Document highlight request payload."""

    document_highlight: (
        destack._generated.query.navigation.highlight.DocumentHighlightRequest
    )
    kind: typing.Literal["documentHighlight"] = "documentHighlight"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request(self)


@dataclass(frozen=True, slots=True)
class QueryRequestSelectionRanges:
    """Selection ranges request payload."""

    selection_ranges: (
        destack._generated.query.navigation.selection_range.SelectionRangesRequest
    )
    kind: typing.Literal["selectionRanges"] = "selectionRanges"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request(self)


@dataclass(frozen=True, slots=True)
class QueryRequestGotoDefinition:
    """Goto definition request payload."""

    goto_definition: (
        destack._generated.query.navigation.definition.GotoDefinitionRequest
    )
    kind: typing.Literal["gotoDefinition"] = "gotoDefinition"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request(self)


@dataclass(frozen=True, slots=True)
class QueryRequestGotoDeclaration:
    """Goto declaration request payload."""

    goto_declaration: (
        destack._generated.query.navigation.definition.GotoDeclarationRequest
    )
    kind: typing.Literal["gotoDeclaration"] = "gotoDeclaration"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request(self)


@dataclass(frozen=True, slots=True)
class QueryRequestGotoTypeDefinition:
    """Goto type definition request payload."""

    goto_type_definition: (
        destack._generated.query.navigation.definition.GotoTypeDefinitionRequest
    )
    kind: typing.Literal["gotoTypeDefinition"] = "gotoTypeDefinition"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request(self)


@dataclass(frozen=True, slots=True)
class QueryRequestGotoImplementation:
    """Goto implementation request payload."""

    goto_implementation: (
        destack._generated.query.navigation.implementation.GotoImplementationRequest
    )
    kind: typing.Literal["gotoImplementation"] = "gotoImplementation"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request(self)


@dataclass(frozen=True, slots=True)
class QueryRequestFindReferences:
    """Find references request payload."""

    find_references: (
        destack._generated.query.navigation.find_references.FindReferencesRequest
    )
    kind: typing.Literal["findReferences"] = "findReferences"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request(self)


@dataclass(frozen=True, slots=True)
class QueryRequestCallHierarchyItem:
    """Call hierarchy item request payload."""

    call_hierarchy_item: (
        destack._generated.query.navigation.call_hierarchy.CallHierarchyItemRequest
    )
    kind: typing.Literal["callHierarchyItem"] = "callHierarchyItem"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request(self)


@dataclass(frozen=True, slots=True)
class QueryRequestCallHierarchyIncoming:
    """Incoming call hierarchy request payload."""

    call_hierarchy_incoming: (
        destack._generated.query.navigation.call_hierarchy.CallHierarchyIncomingRequest
    )
    kind: typing.Literal["callHierarchyIncoming"] = "callHierarchyIncoming"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request(self)


@dataclass(frozen=True, slots=True)
class QueryRequestCallHierarchyOutgoing:
    """Outgoing call hierarchy request payload."""

    call_hierarchy_outgoing: (
        destack._generated.query.navigation.call_hierarchy.CallHierarchyOutgoingRequest
    )
    kind: typing.Literal["callHierarchyOutgoing"] = "callHierarchyOutgoing"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request(self)


@dataclass(frozen=True, slots=True)
class QueryRequestTypeHierarchyItem:
    """Type hierarchy item request payload."""

    type_hierarchy_item: (
        destack._generated.query.navigation.type_hierarchy.TypeHierarchyItemRequest
    )
    kind: typing.Literal["typeHierarchyItem"] = "typeHierarchyItem"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request(self)


@dataclass(frozen=True, slots=True)
class QueryRequestTypeHierarchySupertypes:
    """Type hierarchy supertypes request payload."""

    type_hierarchy_supertypes: destack._generated.query.navigation.type_hierarchy.TypeHierarchySupertypesRequest
    kind: typing.Literal["typeHierarchySupertypes"] = "typeHierarchySupertypes"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request(self)


@dataclass(frozen=True, slots=True)
class QueryRequestTypeHierarchySubtypes:
    """Type hierarchy subtypes request payload."""

    type_hierarchy_subtypes: (
        destack._generated.query.navigation.type_hierarchy.TypeHierarchySubtypesRequest
    )
    kind: typing.Literal["typeHierarchySubtypes"] = "typeHierarchySubtypes"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request(self)


@dataclass(frozen=True, slots=True)
class QueryRequestAnnotations:
    """Annotation request payload."""

    annotations: destack._generated.query.navigation.annotation.AnnotationsRequest
    kind: typing.Literal["annotations"] = "annotations"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request(self)


@dataclass(frozen=True, slots=True)
class QueryRequestRenameTarget:
    """Rename target request payload."""

    rename_target: destack._generated.query.refactor.rename.RenameTargetRequest
    kind: typing.Literal["renameTarget"] = "renameTarget"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request(self)


@dataclass(frozen=True, slots=True)
class QueryRequestRename:
    """Rename request payload."""

    rename: destack._generated.query.refactor.rename.RenameRequest
    kind: typing.Literal["rename"] = "rename"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request(self)


@dataclass(frozen=True, slots=True)
class QueryRequestRenameFiles:
    """Rename files request payload."""

    rename_files: destack._generated.query.refactor.file_rename.RenameFilesRequest
    kind: typing.Literal["renameFiles"] = "renameFiles"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request(self)


@dataclass(frozen=True, slots=True)
class QueryRequestExtractFunction:
    """Extract function request payload."""

    extract_function: (
        destack._generated.query.refactor.extract_function.ExtractFunctionRequest
    )
    kind: typing.Literal["extractFunction"] = "extractFunction"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request(self)


@dataclass(frozen=True, slots=True)
class QueryRequestExtractVariable:
    """Extract variable request payload."""

    extract_variable: (
        destack._generated.query.refactor.extract_variable.ExtractVariableRequest
    )
    kind: typing.Literal["extractVariable"] = "extractVariable"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request(self)


@dataclass(frozen=True, slots=True)
class QueryRequestInline:
    """Inline request payload."""

    inline: destack._generated.query.refactor.inline.InlineRequest
    kind: typing.Literal["inline"] = "inline"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request(self)


@dataclass(frozen=True, slots=True)
class QueryRequestChangeSignature:
    """Change signature request payload."""

    change_signature: (
        destack._generated.query.refactor.change_signature.ChangeSignatureRequest
    )
    kind: typing.Literal["changeSignature"] = "changeSignature"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request(self)


@dataclass(frozen=True, slots=True)
class QueryRequestCodeActions:
    """Code actions request payload."""

    code_actions: destack._generated.query.refactor.code_action.CodeActionsRequest
    kind: typing.Literal["codeActions"] = "codeActions"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request(self)


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


def encode_query_request(writer: BinaryWriter, value: QueryRequest) -> None:
    """Encode one QueryRequest."""
    if value.kind == "completion":
        writer.write_unsigned(0)
        destack._generated.query.assist.completion.encode_completion_request(
            writer, value.completion
        )
    elif value.kind == "hover":
        writer.write_unsigned(1)
        destack._generated.query.assist.hover.encode_hover_request(writer, value.hover)
    elif value.kind == "signatureHelp":
        writer.write_unsigned(2)
        destack._generated.query.assist.signature.encode_signature_help_request(
            writer, value.signature_help
        )
    elif value.kind == "inlayHints":
        writer.write_unsigned(3)
        destack._generated.query.assist.inlay.encode_inlay_hints_request(
            writer, value.inlay_hints
        )
    elif value.kind == "codeLenses":
        writer.write_unsigned(4)
        destack._generated.query.assist.lens.encode_code_lenses_request(
            writer, value.code_lenses
        )
    elif value.kind == "resolveCodeLens":
        writer.write_unsigned(5)
        destack._generated.query.assist.lens.encode_resolve_code_lens_request(
            writer, value.resolve_code_lens
        )
    elif value.kind == "foldingRanges":
        writer.write_unsigned(6)
        destack._generated.query.assist.folding.encode_folding_ranges_request(
            writer, value.folding_ranges
        )
    elif value.kind == "semanticTokens":
        writer.write_unsigned(7)
        destack._generated.query.assist.semantic.encode_semantic_tokens_request(
            writer, value.semantic_tokens
        )
    elif value.kind == "semanticTokensRange":
        writer.write_unsigned(8)
        destack._generated.query.assist.semantic.encode_semantic_tokens_range_request(
            writer, value.semantic_tokens_range
        )
    elif value.kind == "documentSymbols":
        writer.write_unsigned(9)
        destack._generated.query.navigation.document_symbol.encode_document_symbols_request(
            writer, value.document_symbols
        )
    elif value.kind == "workspaceSymbols":
        writer.write_unsigned(10)
        destack._generated.query.navigation.workspace_symbol.encode_workspace_symbols_request(
            writer, value.workspace_symbols
        )
    elif value.kind == "documentLinks":
        writer.write_unsigned(11)
        destack._generated.query.navigation.document_link.encode_document_links_request(
            writer, value.document_links
        )
    elif value.kind == "documentHighlight":
        writer.write_unsigned(12)
        destack._generated.query.navigation.highlight.encode_document_highlight_request(
            writer, value.document_highlight
        )
    elif value.kind == "selectionRanges":
        writer.write_unsigned(13)
        destack._generated.query.navigation.selection_range.encode_selection_ranges_request(
            writer, value.selection_ranges
        )
    elif value.kind == "gotoDefinition":
        writer.write_unsigned(14)
        destack._generated.query.navigation.definition.encode_goto_definition_request(
            writer, value.goto_definition
        )
    elif value.kind == "gotoDeclaration":
        writer.write_unsigned(15)
        destack._generated.query.navigation.definition.encode_goto_declaration_request(
            writer, value.goto_declaration
        )
    elif value.kind == "gotoTypeDefinition":
        writer.write_unsigned(16)
        destack._generated.query.navigation.definition.encode_goto_type_definition_request(
            writer, value.goto_type_definition
        )
    elif value.kind == "gotoImplementation":
        writer.write_unsigned(17)
        destack._generated.query.navigation.implementation.encode_goto_implementation_request(
            writer, value.goto_implementation
        )
    elif value.kind == "findReferences":
        writer.write_unsigned(18)
        destack._generated.query.navigation.find_references.encode_find_references_request(
            writer, value.find_references
        )
    elif value.kind == "callHierarchyItem":
        writer.write_unsigned(19)
        destack._generated.query.navigation.call_hierarchy.encode_call_hierarchy_item_request(
            writer, value.call_hierarchy_item
        )
    elif value.kind == "callHierarchyIncoming":
        writer.write_unsigned(20)
        destack._generated.query.navigation.call_hierarchy.encode_call_hierarchy_incoming_request(
            writer, value.call_hierarchy_incoming
        )
    elif value.kind == "callHierarchyOutgoing":
        writer.write_unsigned(21)
        destack._generated.query.navigation.call_hierarchy.encode_call_hierarchy_outgoing_request(
            writer, value.call_hierarchy_outgoing
        )
    elif value.kind == "typeHierarchyItem":
        writer.write_unsigned(22)
        destack._generated.query.navigation.type_hierarchy.encode_type_hierarchy_item_request(
            writer, value.type_hierarchy_item
        )
    elif value.kind == "typeHierarchySupertypes":
        writer.write_unsigned(23)
        destack._generated.query.navigation.type_hierarchy.encode_type_hierarchy_supertypes_request(
            writer, value.type_hierarchy_supertypes
        )
    elif value.kind == "typeHierarchySubtypes":
        writer.write_unsigned(24)
        destack._generated.query.navigation.type_hierarchy.encode_type_hierarchy_subtypes_request(
            writer, value.type_hierarchy_subtypes
        )
    elif value.kind == "annotations":
        writer.write_unsigned(25)
        destack._generated.query.navigation.annotation.encode_annotations_request(
            writer, value.annotations
        )
    elif value.kind == "renameTarget":
        writer.write_unsigned(26)
        destack._generated.query.refactor.rename.encode_rename_target_request(
            writer, value.rename_target
        )
    elif value.kind == "rename":
        writer.write_unsigned(27)
        destack._generated.query.refactor.rename.encode_rename_request(
            writer, value.rename
        )
    elif value.kind == "renameFiles":
        writer.write_unsigned(28)
        destack._generated.query.refactor.file_rename.encode_rename_files_request(
            writer, value.rename_files
        )
    elif value.kind == "extractFunction":
        writer.write_unsigned(29)
        destack._generated.query.refactor.extract_function.encode_extract_function_request(
            writer, value.extract_function
        )
    elif value.kind == "extractVariable":
        writer.write_unsigned(30)
        destack._generated.query.refactor.extract_variable.encode_extract_variable_request(
            writer, value.extract_variable
        )
    elif value.kind == "inline":
        writer.write_unsigned(31)
        destack._generated.query.refactor.inline.encode_inline_request(
            writer, value.inline
        )
    elif value.kind == "changeSignature":
        writer.write_unsigned(32)
        destack._generated.query.refactor.change_signature.encode_change_signature_request(
            writer, value.change_signature
        )
    elif value.kind == "codeActions":
        writer.write_unsigned(33)
        destack._generated.query.refactor.code_action.encode_code_actions_request(
            writer, value.code_actions
        )
    else:
        raise SerdeError("unknown enum variant")


def decode_query_request(reader: BinaryReader) -> QueryRequest:
    """Decode one QueryRequest."""
    variant = reader.read_number()

    if variant == 0:
        completion = (
            destack._generated.query.assist.completion.decode_completion_request(reader)
        )

        return QueryRequestCompletion(completion=completion)
    elif variant == 1:
        hover = destack._generated.query.assist.hover.decode_hover_request(reader)

        return QueryRequestHover(hover=hover)
    elif variant == 2:
        signature_help = (
            destack._generated.query.assist.signature.decode_signature_help_request(
                reader
            )
        )

        return QueryRequestSignatureHelp(signature_help=signature_help)
    elif variant == 3:
        inlay_hints = destack._generated.query.assist.inlay.decode_inlay_hints_request(
            reader
        )

        return QueryRequestInlayHints(inlay_hints=inlay_hints)
    elif variant == 4:
        code_lenses = destack._generated.query.assist.lens.decode_code_lenses_request(
            reader
        )

        return QueryRequestCodeLenses(code_lenses=code_lenses)
    elif variant == 5:
        resolve_code_lens = (
            destack._generated.query.assist.lens.decode_resolve_code_lens_request(
                reader
            )
        )

        return QueryRequestResolveCodeLens(resolve_code_lens=resolve_code_lens)
    elif variant == 6:
        folding_ranges = (
            destack._generated.query.assist.folding.decode_folding_ranges_request(
                reader
            )
        )

        return QueryRequestFoldingRanges(folding_ranges=folding_ranges)
    elif variant == 7:
        semantic_tokens = (
            destack._generated.query.assist.semantic.decode_semantic_tokens_request(
                reader
            )
        )

        return QueryRequestSemanticTokens(semantic_tokens=semantic_tokens)
    elif variant == 8:
        semantic_tokens_range = destack._generated.query.assist.semantic.decode_semantic_tokens_range_request(
            reader
        )

        return QueryRequestSemanticTokensRange(
            semantic_tokens_range=semantic_tokens_range
        )
    elif variant == 9:
        document_symbols = destack._generated.query.navigation.document_symbol.decode_document_symbols_request(
            reader
        )

        return QueryRequestDocumentSymbols(document_symbols=document_symbols)
    elif variant == 10:
        workspace_symbols = destack._generated.query.navigation.workspace_symbol.decode_workspace_symbols_request(
            reader
        )

        return QueryRequestWorkspaceSymbols(workspace_symbols=workspace_symbols)
    elif variant == 11:
        document_links = destack._generated.query.navigation.document_link.decode_document_links_request(
            reader
        )

        return QueryRequestDocumentLinks(document_links=document_links)
    elif variant == 12:
        document_highlight = destack._generated.query.navigation.highlight.decode_document_highlight_request(
            reader
        )

        return QueryRequestDocumentHighlight(document_highlight=document_highlight)
    elif variant == 13:
        selection_ranges = destack._generated.query.navigation.selection_range.decode_selection_ranges_request(
            reader
        )

        return QueryRequestSelectionRanges(selection_ranges=selection_ranges)
    elif variant == 14:
        goto_definition = destack._generated.query.navigation.definition.decode_goto_definition_request(
            reader
        )

        return QueryRequestGotoDefinition(goto_definition=goto_definition)
    elif variant == 15:
        goto_declaration = destack._generated.query.navigation.definition.decode_goto_declaration_request(
            reader
        )

        return QueryRequestGotoDeclaration(goto_declaration=goto_declaration)
    elif variant == 16:
        goto_type_definition = destack._generated.query.navigation.definition.decode_goto_type_definition_request(
            reader
        )

        return QueryRequestGotoTypeDefinition(goto_type_definition=goto_type_definition)
    elif variant == 17:
        goto_implementation = destack._generated.query.navigation.implementation.decode_goto_implementation_request(
            reader
        )

        return QueryRequestGotoImplementation(goto_implementation=goto_implementation)
    elif variant == 18:
        find_references = destack._generated.query.navigation.find_references.decode_find_references_request(
            reader
        )

        return QueryRequestFindReferences(find_references=find_references)
    elif variant == 19:
        call_hierarchy_item = destack._generated.query.navigation.call_hierarchy.decode_call_hierarchy_item_request(
            reader
        )

        return QueryRequestCallHierarchyItem(call_hierarchy_item=call_hierarchy_item)
    elif variant == 20:
        call_hierarchy_incoming = destack._generated.query.navigation.call_hierarchy.decode_call_hierarchy_incoming_request(
            reader
        )

        return QueryRequestCallHierarchyIncoming(
            call_hierarchy_incoming=call_hierarchy_incoming
        )
    elif variant == 21:
        call_hierarchy_outgoing = destack._generated.query.navigation.call_hierarchy.decode_call_hierarchy_outgoing_request(
            reader
        )

        return QueryRequestCallHierarchyOutgoing(
            call_hierarchy_outgoing=call_hierarchy_outgoing
        )
    elif variant == 22:
        type_hierarchy_item = destack._generated.query.navigation.type_hierarchy.decode_type_hierarchy_item_request(
            reader
        )

        return QueryRequestTypeHierarchyItem(type_hierarchy_item=type_hierarchy_item)
    elif variant == 23:
        type_hierarchy_supertypes = destack._generated.query.navigation.type_hierarchy.decode_type_hierarchy_supertypes_request(
            reader
        )

        return QueryRequestTypeHierarchySupertypes(
            type_hierarchy_supertypes=type_hierarchy_supertypes
        )
    elif variant == 24:
        type_hierarchy_subtypes = destack._generated.query.navigation.type_hierarchy.decode_type_hierarchy_subtypes_request(
            reader
        )

        return QueryRequestTypeHierarchySubtypes(
            type_hierarchy_subtypes=type_hierarchy_subtypes
        )
    elif variant == 25:
        annotations = (
            destack._generated.query.navigation.annotation.decode_annotations_request(
                reader
            )
        )

        return QueryRequestAnnotations(annotations=annotations)
    elif variant == 26:
        rename_target = (
            destack._generated.query.refactor.rename.decode_rename_target_request(
                reader
            )
        )

        return QueryRequestRenameTarget(rename_target=rename_target)
    elif variant == 27:
        rename = destack._generated.query.refactor.rename.decode_rename_request(reader)

        return QueryRequestRename(rename=rename)
    elif variant == 28:
        rename_files = (
            destack._generated.query.refactor.file_rename.decode_rename_files_request(
                reader
            )
        )

        return QueryRequestRenameFiles(rename_files=rename_files)
    elif variant == 29:
        extract_function = destack._generated.query.refactor.extract_function.decode_extract_function_request(
            reader
        )

        return QueryRequestExtractFunction(extract_function=extract_function)
    elif variant == 30:
        extract_variable = destack._generated.query.refactor.extract_variable.decode_extract_variable_request(
            reader
        )

        return QueryRequestExtractVariable(extract_variable=extract_variable)
    elif variant == 31:
        inline = destack._generated.query.refactor.inline.decode_inline_request(reader)

        return QueryRequestInline(inline=inline)
    elif variant == 32:
        change_signature = destack._generated.query.refactor.change_signature.decode_change_signature_request(
            reader
        )

        return QueryRequestChangeSignature(change_signature=change_signature)
    elif variant == 33:
        code_actions = (
            destack._generated.query.refactor.code_action.decode_code_actions_request(
                reader
            )
        )

        return QueryRequestCodeActions(code_actions=code_actions)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_query_request(value: QueryRequest) -> Json:
    """Return one JSON value for one QueryRequest."""
    if value.kind == "completion":
        return {
            "kind": "completion",
            "completion": destack._generated.query.assist.completion.to_json_completion_request(
                value.completion
            ),
        }
    elif value.kind == "hover":
        return {
            "kind": "hover",
            "hover": destack._generated.query.assist.hover.to_json_hover_request(
                value.hover
            ),
        }
    elif value.kind == "signatureHelp":
        return {
            "kind": "signatureHelp",
            "signature_help": destack._generated.query.assist.signature.to_json_signature_help_request(
                value.signature_help
            ),
        }
    elif value.kind == "inlayHints":
        return {
            "kind": "inlayHints",
            "inlay_hints": destack._generated.query.assist.inlay.to_json_inlay_hints_request(
                value.inlay_hints
            ),
        }
    elif value.kind == "codeLenses":
        return {
            "kind": "codeLenses",
            "code_lenses": destack._generated.query.assist.lens.to_json_code_lenses_request(
                value.code_lenses
            ),
        }
    elif value.kind == "resolveCodeLens":
        return {
            "kind": "resolveCodeLens",
            "resolve_code_lens": destack._generated.query.assist.lens.to_json_resolve_code_lens_request(
                value.resolve_code_lens
            ),
        }
    elif value.kind == "foldingRanges":
        return {
            "kind": "foldingRanges",
            "folding_ranges": destack._generated.query.assist.folding.to_json_folding_ranges_request(
                value.folding_ranges
            ),
        }
    elif value.kind == "semanticTokens":
        return {
            "kind": "semanticTokens",
            "semantic_tokens": destack._generated.query.assist.semantic.to_json_semantic_tokens_request(
                value.semantic_tokens
            ),
        }
    elif value.kind == "semanticTokensRange":
        return {
            "kind": "semanticTokensRange",
            "semantic_tokens_range": destack._generated.query.assist.semantic.to_json_semantic_tokens_range_request(
                value.semantic_tokens_range
            ),
        }
    elif value.kind == "documentSymbols":
        return {
            "kind": "documentSymbols",
            "document_symbols": destack._generated.query.navigation.document_symbol.to_json_document_symbols_request(
                value.document_symbols
            ),
        }
    elif value.kind == "workspaceSymbols":
        return {
            "kind": "workspaceSymbols",
            "workspace_symbols": destack._generated.query.navigation.workspace_symbol.to_json_workspace_symbols_request(
                value.workspace_symbols
            ),
        }
    elif value.kind == "documentLinks":
        return {
            "kind": "documentLinks",
            "document_links": destack._generated.query.navigation.document_link.to_json_document_links_request(
                value.document_links
            ),
        }
    elif value.kind == "documentHighlight":
        return {
            "kind": "documentHighlight",
            "document_highlight": destack._generated.query.navigation.highlight.to_json_document_highlight_request(
                value.document_highlight
            ),
        }
    elif value.kind == "selectionRanges":
        return {
            "kind": "selectionRanges",
            "selection_ranges": destack._generated.query.navigation.selection_range.to_json_selection_ranges_request(
                value.selection_ranges
            ),
        }
    elif value.kind == "gotoDefinition":
        return {
            "kind": "gotoDefinition",
            "goto_definition": destack._generated.query.navigation.definition.to_json_goto_definition_request(
                value.goto_definition
            ),
        }
    elif value.kind == "gotoDeclaration":
        return {
            "kind": "gotoDeclaration",
            "goto_declaration": destack._generated.query.navigation.definition.to_json_goto_declaration_request(
                value.goto_declaration
            ),
        }
    elif value.kind == "gotoTypeDefinition":
        return {
            "kind": "gotoTypeDefinition",
            "goto_type_definition": destack._generated.query.navigation.definition.to_json_goto_type_definition_request(
                value.goto_type_definition
            ),
        }
    elif value.kind == "gotoImplementation":
        return {
            "kind": "gotoImplementation",
            "goto_implementation": destack._generated.query.navigation.implementation.to_json_goto_implementation_request(
                value.goto_implementation
            ),
        }
    elif value.kind == "findReferences":
        return {
            "kind": "findReferences",
            "find_references": destack._generated.query.navigation.find_references.to_json_find_references_request(
                value.find_references
            ),
        }
    elif value.kind == "callHierarchyItem":
        return {
            "kind": "callHierarchyItem",
            "call_hierarchy_item": destack._generated.query.navigation.call_hierarchy.to_json_call_hierarchy_item_request(
                value.call_hierarchy_item
            ),
        }
    elif value.kind == "callHierarchyIncoming":
        return {
            "kind": "callHierarchyIncoming",
            "call_hierarchy_incoming": destack._generated.query.navigation.call_hierarchy.to_json_call_hierarchy_incoming_request(
                value.call_hierarchy_incoming
            ),
        }
    elif value.kind == "callHierarchyOutgoing":
        return {
            "kind": "callHierarchyOutgoing",
            "call_hierarchy_outgoing": destack._generated.query.navigation.call_hierarchy.to_json_call_hierarchy_outgoing_request(
                value.call_hierarchy_outgoing
            ),
        }
    elif value.kind == "typeHierarchyItem":
        return {
            "kind": "typeHierarchyItem",
            "type_hierarchy_item": destack._generated.query.navigation.type_hierarchy.to_json_type_hierarchy_item_request(
                value.type_hierarchy_item
            ),
        }
    elif value.kind == "typeHierarchySupertypes":
        return {
            "kind": "typeHierarchySupertypes",
            "type_hierarchy_supertypes": destack._generated.query.navigation.type_hierarchy.to_json_type_hierarchy_supertypes_request(
                value.type_hierarchy_supertypes
            ),
        }
    elif value.kind == "typeHierarchySubtypes":
        return {
            "kind": "typeHierarchySubtypes",
            "type_hierarchy_subtypes": destack._generated.query.navigation.type_hierarchy.to_json_type_hierarchy_subtypes_request(
                value.type_hierarchy_subtypes
            ),
        }
    elif value.kind == "annotations":
        return {
            "kind": "annotations",
            "annotations": destack._generated.query.navigation.annotation.to_json_annotations_request(
                value.annotations
            ),
        }
    elif value.kind == "renameTarget":
        return {
            "kind": "renameTarget",
            "rename_target": destack._generated.query.refactor.rename.to_json_rename_target_request(
                value.rename_target
            ),
        }
    elif value.kind == "rename":
        return {
            "kind": "rename",
            "rename": destack._generated.query.refactor.rename.to_json_rename_request(
                value.rename
            ),
        }
    elif value.kind == "renameFiles":
        return {
            "kind": "renameFiles",
            "rename_files": destack._generated.query.refactor.file_rename.to_json_rename_files_request(
                value.rename_files
            ),
        }
    elif value.kind == "extractFunction":
        return {
            "kind": "extractFunction",
            "extract_function": destack._generated.query.refactor.extract_function.to_json_extract_function_request(
                value.extract_function
            ),
        }
    elif value.kind == "extractVariable":
        return {
            "kind": "extractVariable",
            "extract_variable": destack._generated.query.refactor.extract_variable.to_json_extract_variable_request(
                value.extract_variable
            ),
        }
    elif value.kind == "inline":
        return {
            "kind": "inline",
            "inline": destack._generated.query.refactor.inline.to_json_inline_request(
                value.inline
            ),
        }
    elif value.kind == "changeSignature":
        return {
            "kind": "changeSignature",
            "change_signature": destack._generated.query.refactor.change_signature.to_json_change_signature_request(
                value.change_signature
            ),
        }
    elif value.kind == "codeActions":
        return {
            "kind": "codeActions",
            "code_actions": destack._generated.query.refactor.code_action.to_json_code_actions_request(
                value.code_actions
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_query_request(value: Json) -> QueryRequest:
    """Return one QueryRequest from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "completion":
        return QueryRequestCompletion(
            completion=destack._generated.query.assist.completion.from_json_completion_request(
                json_field(object_, "completion")
            )
        )
    elif kind == "hover":
        return QueryRequestHover(
            hover=destack._generated.query.assist.hover.from_json_hover_request(
                json_field(object_, "hover")
            )
        )
    elif kind == "signatureHelp":
        return QueryRequestSignatureHelp(
            signature_help=destack._generated.query.assist.signature.from_json_signature_help_request(
                json_field(object_, "signature_help")
            )
        )
    elif kind == "inlayHints":
        return QueryRequestInlayHints(
            inlay_hints=destack._generated.query.assist.inlay.from_json_inlay_hints_request(
                json_field(object_, "inlay_hints")
            )
        )
    elif kind == "codeLenses":
        return QueryRequestCodeLenses(
            code_lenses=destack._generated.query.assist.lens.from_json_code_lenses_request(
                json_field(object_, "code_lenses")
            )
        )
    elif kind == "resolveCodeLens":
        return QueryRequestResolveCodeLens(
            resolve_code_lens=destack._generated.query.assist.lens.from_json_resolve_code_lens_request(
                json_field(object_, "resolve_code_lens")
            )
        )
    elif kind == "foldingRanges":
        return QueryRequestFoldingRanges(
            folding_ranges=destack._generated.query.assist.folding.from_json_folding_ranges_request(
                json_field(object_, "folding_ranges")
            )
        )
    elif kind == "semanticTokens":
        return QueryRequestSemanticTokens(
            semantic_tokens=destack._generated.query.assist.semantic.from_json_semantic_tokens_request(
                json_field(object_, "semantic_tokens")
            )
        )
    elif kind == "semanticTokensRange":
        return QueryRequestSemanticTokensRange(
            semantic_tokens_range=destack._generated.query.assist.semantic.from_json_semantic_tokens_range_request(
                json_field(object_, "semantic_tokens_range")
            )
        )
    elif kind == "documentSymbols":
        return QueryRequestDocumentSymbols(
            document_symbols=destack._generated.query.navigation.document_symbol.from_json_document_symbols_request(
                json_field(object_, "document_symbols")
            )
        )
    elif kind == "workspaceSymbols":
        return QueryRequestWorkspaceSymbols(
            workspace_symbols=destack._generated.query.navigation.workspace_symbol.from_json_workspace_symbols_request(
                json_field(object_, "workspace_symbols")
            )
        )
    elif kind == "documentLinks":
        return QueryRequestDocumentLinks(
            document_links=destack._generated.query.navigation.document_link.from_json_document_links_request(
                json_field(object_, "document_links")
            )
        )
    elif kind == "documentHighlight":
        return QueryRequestDocumentHighlight(
            document_highlight=destack._generated.query.navigation.highlight.from_json_document_highlight_request(
                json_field(object_, "document_highlight")
            )
        )
    elif kind == "selectionRanges":
        return QueryRequestSelectionRanges(
            selection_ranges=destack._generated.query.navigation.selection_range.from_json_selection_ranges_request(
                json_field(object_, "selection_ranges")
            )
        )
    elif kind == "gotoDefinition":
        return QueryRequestGotoDefinition(
            goto_definition=destack._generated.query.navigation.definition.from_json_goto_definition_request(
                json_field(object_, "goto_definition")
            )
        )
    elif kind == "gotoDeclaration":
        return QueryRequestGotoDeclaration(
            goto_declaration=destack._generated.query.navigation.definition.from_json_goto_declaration_request(
                json_field(object_, "goto_declaration")
            )
        )
    elif kind == "gotoTypeDefinition":
        return QueryRequestGotoTypeDefinition(
            goto_type_definition=destack._generated.query.navigation.definition.from_json_goto_type_definition_request(
                json_field(object_, "goto_type_definition")
            )
        )
    elif kind == "gotoImplementation":
        return QueryRequestGotoImplementation(
            goto_implementation=destack._generated.query.navigation.implementation.from_json_goto_implementation_request(
                json_field(object_, "goto_implementation")
            )
        )
    elif kind == "findReferences":
        return QueryRequestFindReferences(
            find_references=destack._generated.query.navigation.find_references.from_json_find_references_request(
                json_field(object_, "find_references")
            )
        )
    elif kind == "callHierarchyItem":
        return QueryRequestCallHierarchyItem(
            call_hierarchy_item=destack._generated.query.navigation.call_hierarchy.from_json_call_hierarchy_item_request(
                json_field(object_, "call_hierarchy_item")
            )
        )
    elif kind == "callHierarchyIncoming":
        return QueryRequestCallHierarchyIncoming(
            call_hierarchy_incoming=destack._generated.query.navigation.call_hierarchy.from_json_call_hierarchy_incoming_request(
                json_field(object_, "call_hierarchy_incoming")
            )
        )
    elif kind == "callHierarchyOutgoing":
        return QueryRequestCallHierarchyOutgoing(
            call_hierarchy_outgoing=destack._generated.query.navigation.call_hierarchy.from_json_call_hierarchy_outgoing_request(
                json_field(object_, "call_hierarchy_outgoing")
            )
        )
    elif kind == "typeHierarchyItem":
        return QueryRequestTypeHierarchyItem(
            type_hierarchy_item=destack._generated.query.navigation.type_hierarchy.from_json_type_hierarchy_item_request(
                json_field(object_, "type_hierarchy_item")
            )
        )
    elif kind == "typeHierarchySupertypes":
        return QueryRequestTypeHierarchySupertypes(
            type_hierarchy_supertypes=destack._generated.query.navigation.type_hierarchy.from_json_type_hierarchy_supertypes_request(
                json_field(object_, "type_hierarchy_supertypes")
            )
        )
    elif kind == "typeHierarchySubtypes":
        return QueryRequestTypeHierarchySubtypes(
            type_hierarchy_subtypes=destack._generated.query.navigation.type_hierarchy.from_json_type_hierarchy_subtypes_request(
                json_field(object_, "type_hierarchy_subtypes")
            )
        )
    elif kind == "annotations":
        return QueryRequestAnnotations(
            annotations=destack._generated.query.navigation.annotation.from_json_annotations_request(
                json_field(object_, "annotations")
            )
        )
    elif kind == "renameTarget":
        return QueryRequestRenameTarget(
            rename_target=destack._generated.query.refactor.rename.from_json_rename_target_request(
                json_field(object_, "rename_target")
            )
        )
    elif kind == "rename":
        return QueryRequestRename(
            rename=destack._generated.query.refactor.rename.from_json_rename_request(
                json_field(object_, "rename")
            )
        )
    elif kind == "renameFiles":
        return QueryRequestRenameFiles(
            rename_files=destack._generated.query.refactor.file_rename.from_json_rename_files_request(
                json_field(object_, "rename_files")
            )
        )
    elif kind == "extractFunction":
        return QueryRequestExtractFunction(
            extract_function=destack._generated.query.refactor.extract_function.from_json_extract_function_request(
                json_field(object_, "extract_function")
            )
        )
    elif kind == "extractVariable":
        return QueryRequestExtractVariable(
            extract_variable=destack._generated.query.refactor.extract_variable.from_json_extract_variable_request(
                json_field(object_, "extract_variable")
            )
        )
    elif kind == "inline":
        return QueryRequestInline(
            inline=destack._generated.query.refactor.inline.from_json_inline_request(
                json_field(object_, "inline")
            )
        )
    elif kind == "changeSignature":
        return QueryRequestChangeSignature(
            change_signature=destack._generated.query.refactor.change_signature.from_json_change_signature_request(
                json_field(object_, "change_signature")
            )
        )
    elif kind == "codeActions":
        return QueryRequestCodeActions(
            code_actions=destack._generated.query.refactor.code_action.from_json_code_actions_request(
                json_field(object_, "code_actions")
            )
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class QueryResponseCompletion:
    """Completion response payload."""

    completion: destack._generated.query.assist.completion.CompletionResponse
    kind: typing.Literal["completion"] = "completion"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response(self)


@dataclass(frozen=True, slots=True)
class QueryResponseHover:
    """Hover response payload."""

    hover: destack._generated.query.assist.hover.HoverResponse
    kind: typing.Literal["hover"] = "hover"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response(self)


@dataclass(frozen=True, slots=True)
class QueryResponseSignatureHelp:
    """Signature help response payload."""

    signature_help: destack._generated.query.assist.signature.SignatureHelpResponse
    kind: typing.Literal["signatureHelp"] = "signatureHelp"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response(self)


@dataclass(frozen=True, slots=True)
class QueryResponseInlayHints:
    """Inlay hints response payload."""

    inlay_hints: destack._generated.query.assist.inlay.InlayHintsResponse
    kind: typing.Literal["inlayHints"] = "inlayHints"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response(self)


@dataclass(frozen=True, slots=True)
class QueryResponseCodeLenses:
    """Code lenses response payload."""

    code_lenses: destack._generated.query.assist.lens.CodeLensesResponse
    kind: typing.Literal["codeLenses"] = "codeLenses"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response(self)


@dataclass(frozen=True, slots=True)
class QueryResponseResolveCodeLens:
    """Code lens resolve response payload."""

    resolve_code_lens: destack._generated.query.assist.lens.ResolveCodeLensResponse
    kind: typing.Literal["resolveCodeLens"] = "resolveCodeLens"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response(self)


@dataclass(frozen=True, slots=True)
class QueryResponseFoldingRanges:
    """Folding ranges response payload."""

    folding_ranges: destack._generated.query.assist.folding.FoldingRangesResponse
    kind: typing.Literal["foldingRanges"] = "foldingRanges"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response(self)


@dataclass(frozen=True, slots=True)
class QueryResponseSemanticTokens:
    """Full-document semantic tokens response payload."""

    semantic_tokens: destack._generated.query.assist.semantic.SemanticTokensResponse
    kind: typing.Literal["semanticTokens"] = "semanticTokens"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response(self)


@dataclass(frozen=True, slots=True)
class QueryResponseSemanticTokensRange:
    """Range semantic tokens response payload."""

    semantic_tokens_range: (
        destack._generated.query.assist.semantic.SemanticTokensResponse
    )
    kind: typing.Literal["semanticTokensRange"] = "semanticTokensRange"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response(self)


@dataclass(frozen=True, slots=True)
class QueryResponseDocumentSymbols:
    """Document symbols response payload."""

    document_symbols: (
        destack._generated.query.navigation.document_symbol.DocumentSymbolsResponse
    )
    kind: typing.Literal["documentSymbols"] = "documentSymbols"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response(self)


@dataclass(frozen=True, slots=True)
class QueryResponseWorkspaceSymbols:
    """Workspace symbols response payload."""

    workspace_symbols: (
        destack._generated.query.navigation.workspace_symbol.WorkspaceSymbolsResponse
    )
    kind: typing.Literal["workspaceSymbols"] = "workspaceSymbols"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response(self)


@dataclass(frozen=True, slots=True)
class QueryResponseDocumentLinks:
    """Document links response payload."""

    document_links: (
        destack._generated.query.navigation.document_link.DocumentLinksResponse
    )
    kind: typing.Literal["documentLinks"] = "documentLinks"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response(self)


@dataclass(frozen=True, slots=True)
class QueryResponseDocumentHighlight:
    """Document highlight response payload."""

    document_highlight: (
        destack._generated.query.navigation.highlight.DocumentHighlightResponse
    )
    kind: typing.Literal["documentHighlight"] = "documentHighlight"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response(self)


@dataclass(frozen=True, slots=True)
class QueryResponseSelectionRanges:
    """Selection ranges response payload."""

    selection_ranges: (
        destack._generated.query.navigation.selection_range.SelectionRangesResponse
    )
    kind: typing.Literal["selectionRanges"] = "selectionRanges"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response(self)


@dataclass(frozen=True, slots=True)
class QueryResponseGotoDefinition:
    """Goto definition response payload."""

    goto_definition: (
        destack._generated.query.navigation.definition.GotoDefinitionResponse
    )
    kind: typing.Literal["gotoDefinition"] = "gotoDefinition"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response(self)


@dataclass(frozen=True, slots=True)
class QueryResponseGotoDeclaration:
    """Goto declaration response payload."""

    goto_declaration: (
        destack._generated.query.navigation.definition.GotoDeclarationResponse
    )
    kind: typing.Literal["gotoDeclaration"] = "gotoDeclaration"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response(self)


@dataclass(frozen=True, slots=True)
class QueryResponseGotoTypeDefinition:
    """Goto type definition response payload."""

    goto_type_definition: (
        destack._generated.query.navigation.definition.GotoTypeDefinitionResponse
    )
    kind: typing.Literal["gotoTypeDefinition"] = "gotoTypeDefinition"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response(self)


@dataclass(frozen=True, slots=True)
class QueryResponseGotoImplementation:
    """Goto implementation response payload."""

    goto_implementation: (
        destack._generated.query.navigation.implementation.GotoImplementationResponse
    )
    kind: typing.Literal["gotoImplementation"] = "gotoImplementation"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response(self)


@dataclass(frozen=True, slots=True)
class QueryResponseFindReferences:
    """Find references response payload."""

    find_references: (
        destack._generated.query.navigation.find_references.FindReferencesResponse
    )
    kind: typing.Literal["findReferences"] = "findReferences"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response(self)


@dataclass(frozen=True, slots=True)
class QueryResponseCallHierarchyItem:
    """Call hierarchy item response payload."""

    call_hierarchy_item: (
        destack._generated.query.navigation.call_hierarchy.CallHierarchyItemResponse
    )
    kind: typing.Literal["callHierarchyItem"] = "callHierarchyItem"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response(self)


@dataclass(frozen=True, slots=True)
class QueryResponseCallHierarchyIncoming:
    """Incoming call hierarchy response payload."""

    call_hierarchy_incoming: (
        destack._generated.query.navigation.call_hierarchy.CallHierarchyIncomingResponse
    )
    kind: typing.Literal["callHierarchyIncoming"] = "callHierarchyIncoming"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response(self)


@dataclass(frozen=True, slots=True)
class QueryResponseCallHierarchyOutgoing:
    """Outgoing call hierarchy response payload."""

    call_hierarchy_outgoing: (
        destack._generated.query.navigation.call_hierarchy.CallHierarchyOutgoingResponse
    )
    kind: typing.Literal["callHierarchyOutgoing"] = "callHierarchyOutgoing"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response(self)


@dataclass(frozen=True, slots=True)
class QueryResponseTypeHierarchyItem:
    """Type hierarchy item response payload."""

    type_hierarchy_item: (
        destack._generated.query.navigation.type_hierarchy.TypeHierarchyItemResponse
    )
    kind: typing.Literal["typeHierarchyItem"] = "typeHierarchyItem"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response(self)


@dataclass(frozen=True, slots=True)
class QueryResponseTypeHierarchySupertypes:
    """Type hierarchy supertypes response payload."""

    type_hierarchy_supertypes: destack._generated.query.navigation.type_hierarchy.TypeHierarchySupertypesResponse
    kind: typing.Literal["typeHierarchySupertypes"] = "typeHierarchySupertypes"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response(self)


@dataclass(frozen=True, slots=True)
class QueryResponseTypeHierarchySubtypes:
    """Type hierarchy subtypes response payload."""

    type_hierarchy_subtypes: (
        destack._generated.query.navigation.type_hierarchy.TypeHierarchySubtypesResponse
    )
    kind: typing.Literal["typeHierarchySubtypes"] = "typeHierarchySubtypes"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response(self)


@dataclass(frozen=True, slots=True)
class QueryResponseAnnotations:
    """Annotation response payload."""

    annotations: destack._generated.query.navigation.annotation.AnnotationsResponse
    kind: typing.Literal["annotations"] = "annotations"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response(self)


@dataclass(frozen=True, slots=True)
class QueryResponseRenameTarget:
    """Rename target response payload."""

    rename_target: destack._generated.query.refactor.rename.RenameTargetResponse
    kind: typing.Literal["renameTarget"] = "renameTarget"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response(self)


@dataclass(frozen=True, slots=True)
class QueryResponseRename:
    """Rename response payload."""

    rename: destack._generated.query.refactor.rename.RenameResponse
    kind: typing.Literal["rename"] = "rename"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response(self)


@dataclass(frozen=True, slots=True)
class QueryResponseRenameFiles:
    """Rename files response payload."""

    rename_files: destack._generated.query.refactor.file_rename.RenameFilesResponse
    kind: typing.Literal["renameFiles"] = "renameFiles"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response(self)


@dataclass(frozen=True, slots=True)
class QueryResponseExtractFunction:
    """Extract function response payload."""

    extract_function: (
        destack._generated.query.refactor.extract_function.ExtractFunctionResponse
    )
    kind: typing.Literal["extractFunction"] = "extractFunction"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response(self)


@dataclass(frozen=True, slots=True)
class QueryResponseExtractVariable:
    """Extract variable response payload."""

    extract_variable: (
        destack._generated.query.refactor.extract_variable.ExtractVariableResponse
    )
    kind: typing.Literal["extractVariable"] = "extractVariable"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response(self)


@dataclass(frozen=True, slots=True)
class QueryResponseInline:
    """Inline response payload."""

    inline: destack._generated.query.refactor.inline.InlineResponse
    kind: typing.Literal["inline"] = "inline"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response(self)


@dataclass(frozen=True, slots=True)
class QueryResponseChangeSignature:
    """Change signature response payload."""

    change_signature: (
        destack._generated.query.refactor.change_signature.ChangeSignatureResponse
    )
    kind: typing.Literal["changeSignature"] = "changeSignature"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response(self)


@dataclass(frozen=True, slots=True)
class QueryResponseCodeActions:
    """Code actions response payload."""

    code_actions: destack._generated.query.refactor.code_action.CodeActionsResponse
    kind: typing.Literal["codeActions"] = "codeActions"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response(self)


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


def encode_query_response(writer: BinaryWriter, value: QueryResponse) -> None:
    """Encode one QueryResponse."""
    if value.kind == "completion":
        writer.write_unsigned(0)
        destack._generated.query.assist.completion.encode_completion_response(
            writer, value.completion
        )
    elif value.kind == "hover":
        writer.write_unsigned(1)
        destack._generated.query.assist.hover.encode_hover_response(writer, value.hover)
    elif value.kind == "signatureHelp":
        writer.write_unsigned(2)
        destack._generated.query.assist.signature.encode_signature_help_response(
            writer, value.signature_help
        )
    elif value.kind == "inlayHints":
        writer.write_unsigned(3)
        destack._generated.query.assist.inlay.encode_inlay_hints_response(
            writer, value.inlay_hints
        )
    elif value.kind == "codeLenses":
        writer.write_unsigned(4)
        destack._generated.query.assist.lens.encode_code_lenses_response(
            writer, value.code_lenses
        )
    elif value.kind == "resolveCodeLens":
        writer.write_unsigned(5)
        destack._generated.query.assist.lens.encode_resolve_code_lens_response(
            writer, value.resolve_code_lens
        )
    elif value.kind == "foldingRanges":
        writer.write_unsigned(6)
        destack._generated.query.assist.folding.encode_folding_ranges_response(
            writer, value.folding_ranges
        )
    elif value.kind == "semanticTokens":
        writer.write_unsigned(7)
        destack._generated.query.assist.semantic.encode_semantic_tokens_response(
            writer, value.semantic_tokens
        )
    elif value.kind == "semanticTokensRange":
        writer.write_unsigned(8)
        destack._generated.query.assist.semantic.encode_semantic_tokens_response(
            writer, value.semantic_tokens_range
        )
    elif value.kind == "documentSymbols":
        writer.write_unsigned(9)
        destack._generated.query.navigation.document_symbol.encode_document_symbols_response(
            writer, value.document_symbols
        )
    elif value.kind == "workspaceSymbols":
        writer.write_unsigned(10)
        destack._generated.query.navigation.workspace_symbol.encode_workspace_symbols_response(
            writer, value.workspace_symbols
        )
    elif value.kind == "documentLinks":
        writer.write_unsigned(11)
        destack._generated.query.navigation.document_link.encode_document_links_response(
            writer, value.document_links
        )
    elif value.kind == "documentHighlight":
        writer.write_unsigned(12)
        destack._generated.query.navigation.highlight.encode_document_highlight_response(
            writer, value.document_highlight
        )
    elif value.kind == "selectionRanges":
        writer.write_unsigned(13)
        destack._generated.query.navigation.selection_range.encode_selection_ranges_response(
            writer, value.selection_ranges
        )
    elif value.kind == "gotoDefinition":
        writer.write_unsigned(14)
        destack._generated.query.navigation.definition.encode_goto_definition_response(
            writer, value.goto_definition
        )
    elif value.kind == "gotoDeclaration":
        writer.write_unsigned(15)
        destack._generated.query.navigation.definition.encode_goto_declaration_response(
            writer, value.goto_declaration
        )
    elif value.kind == "gotoTypeDefinition":
        writer.write_unsigned(16)
        destack._generated.query.navigation.definition.encode_goto_type_definition_response(
            writer, value.goto_type_definition
        )
    elif value.kind == "gotoImplementation":
        writer.write_unsigned(17)
        destack._generated.query.navigation.implementation.encode_goto_implementation_response(
            writer, value.goto_implementation
        )
    elif value.kind == "findReferences":
        writer.write_unsigned(18)
        destack._generated.query.navigation.find_references.encode_find_references_response(
            writer, value.find_references
        )
    elif value.kind == "callHierarchyItem":
        writer.write_unsigned(19)
        destack._generated.query.navigation.call_hierarchy.encode_call_hierarchy_item_response(
            writer, value.call_hierarchy_item
        )
    elif value.kind == "callHierarchyIncoming":
        writer.write_unsigned(20)
        destack._generated.query.navigation.call_hierarchy.encode_call_hierarchy_incoming_response(
            writer, value.call_hierarchy_incoming
        )
    elif value.kind == "callHierarchyOutgoing":
        writer.write_unsigned(21)
        destack._generated.query.navigation.call_hierarchy.encode_call_hierarchy_outgoing_response(
            writer, value.call_hierarchy_outgoing
        )
    elif value.kind == "typeHierarchyItem":
        writer.write_unsigned(22)
        destack._generated.query.navigation.type_hierarchy.encode_type_hierarchy_item_response(
            writer, value.type_hierarchy_item
        )
    elif value.kind == "typeHierarchySupertypes":
        writer.write_unsigned(23)
        destack._generated.query.navigation.type_hierarchy.encode_type_hierarchy_supertypes_response(
            writer, value.type_hierarchy_supertypes
        )
    elif value.kind == "typeHierarchySubtypes":
        writer.write_unsigned(24)
        destack._generated.query.navigation.type_hierarchy.encode_type_hierarchy_subtypes_response(
            writer, value.type_hierarchy_subtypes
        )
    elif value.kind == "annotations":
        writer.write_unsigned(25)
        destack._generated.query.navigation.annotation.encode_annotations_response(
            writer, value.annotations
        )
    elif value.kind == "renameTarget":
        writer.write_unsigned(26)
        destack._generated.query.refactor.rename.encode_rename_target_response(
            writer, value.rename_target
        )
    elif value.kind == "rename":
        writer.write_unsigned(27)
        destack._generated.query.refactor.rename.encode_rename_response(
            writer, value.rename
        )
    elif value.kind == "renameFiles":
        writer.write_unsigned(28)
        destack._generated.query.refactor.file_rename.encode_rename_files_response(
            writer, value.rename_files
        )
    elif value.kind == "extractFunction":
        writer.write_unsigned(29)
        destack._generated.query.refactor.extract_function.encode_extract_function_response(
            writer, value.extract_function
        )
    elif value.kind == "extractVariable":
        writer.write_unsigned(30)
        destack._generated.query.refactor.extract_variable.encode_extract_variable_response(
            writer, value.extract_variable
        )
    elif value.kind == "inline":
        writer.write_unsigned(31)
        destack._generated.query.refactor.inline.encode_inline_response(
            writer, value.inline
        )
    elif value.kind == "changeSignature":
        writer.write_unsigned(32)
        destack._generated.query.refactor.change_signature.encode_change_signature_response(
            writer, value.change_signature
        )
    elif value.kind == "codeActions":
        writer.write_unsigned(33)
        destack._generated.query.refactor.code_action.encode_code_actions_response(
            writer, value.code_actions
        )
    else:
        raise SerdeError("unknown enum variant")


def decode_query_response(reader: BinaryReader) -> QueryResponse:
    """Decode one QueryResponse."""
    variant = reader.read_number()

    if variant == 0:
        completion = (
            destack._generated.query.assist.completion.decode_completion_response(
                reader
            )
        )

        return QueryResponseCompletion(completion=completion)
    elif variant == 1:
        hover = destack._generated.query.assist.hover.decode_hover_response(reader)

        return QueryResponseHover(hover=hover)
    elif variant == 2:
        signature_help = (
            destack._generated.query.assist.signature.decode_signature_help_response(
                reader
            )
        )

        return QueryResponseSignatureHelp(signature_help=signature_help)
    elif variant == 3:
        inlay_hints = destack._generated.query.assist.inlay.decode_inlay_hints_response(
            reader
        )

        return QueryResponseInlayHints(inlay_hints=inlay_hints)
    elif variant == 4:
        code_lenses = destack._generated.query.assist.lens.decode_code_lenses_response(
            reader
        )

        return QueryResponseCodeLenses(code_lenses=code_lenses)
    elif variant == 5:
        resolve_code_lens = (
            destack._generated.query.assist.lens.decode_resolve_code_lens_response(
                reader
            )
        )

        return QueryResponseResolveCodeLens(resolve_code_lens=resolve_code_lens)
    elif variant == 6:
        folding_ranges = (
            destack._generated.query.assist.folding.decode_folding_ranges_response(
                reader
            )
        )

        return QueryResponseFoldingRanges(folding_ranges=folding_ranges)
    elif variant == 7:
        semantic_tokens = (
            destack._generated.query.assist.semantic.decode_semantic_tokens_response(
                reader
            )
        )

        return QueryResponseSemanticTokens(semantic_tokens=semantic_tokens)
    elif variant == 8:
        semantic_tokens_range = (
            destack._generated.query.assist.semantic.decode_semantic_tokens_response(
                reader
            )
        )

        return QueryResponseSemanticTokensRange(
            semantic_tokens_range=semantic_tokens_range
        )
    elif variant == 9:
        document_symbols = destack._generated.query.navigation.document_symbol.decode_document_symbols_response(
            reader
        )

        return QueryResponseDocumentSymbols(document_symbols=document_symbols)
    elif variant == 10:
        workspace_symbols = destack._generated.query.navigation.workspace_symbol.decode_workspace_symbols_response(
            reader
        )

        return QueryResponseWorkspaceSymbols(workspace_symbols=workspace_symbols)
    elif variant == 11:
        document_links = destack._generated.query.navigation.document_link.decode_document_links_response(
            reader
        )

        return QueryResponseDocumentLinks(document_links=document_links)
    elif variant == 12:
        document_highlight = destack._generated.query.navigation.highlight.decode_document_highlight_response(
            reader
        )

        return QueryResponseDocumentHighlight(document_highlight=document_highlight)
    elif variant == 13:
        selection_ranges = destack._generated.query.navigation.selection_range.decode_selection_ranges_response(
            reader
        )

        return QueryResponseSelectionRanges(selection_ranges=selection_ranges)
    elif variant == 14:
        goto_definition = destack._generated.query.navigation.definition.decode_goto_definition_response(
            reader
        )

        return QueryResponseGotoDefinition(goto_definition=goto_definition)
    elif variant == 15:
        goto_declaration = destack._generated.query.navigation.definition.decode_goto_declaration_response(
            reader
        )

        return QueryResponseGotoDeclaration(goto_declaration=goto_declaration)
    elif variant == 16:
        goto_type_definition = destack._generated.query.navigation.definition.decode_goto_type_definition_response(
            reader
        )

        return QueryResponseGotoTypeDefinition(
            goto_type_definition=goto_type_definition
        )
    elif variant == 17:
        goto_implementation = destack._generated.query.navigation.implementation.decode_goto_implementation_response(
            reader
        )

        return QueryResponseGotoImplementation(goto_implementation=goto_implementation)
    elif variant == 18:
        find_references = destack._generated.query.navigation.find_references.decode_find_references_response(
            reader
        )

        return QueryResponseFindReferences(find_references=find_references)
    elif variant == 19:
        call_hierarchy_item = destack._generated.query.navigation.call_hierarchy.decode_call_hierarchy_item_response(
            reader
        )

        return QueryResponseCallHierarchyItem(call_hierarchy_item=call_hierarchy_item)
    elif variant == 20:
        call_hierarchy_incoming = destack._generated.query.navigation.call_hierarchy.decode_call_hierarchy_incoming_response(
            reader
        )

        return QueryResponseCallHierarchyIncoming(
            call_hierarchy_incoming=call_hierarchy_incoming
        )
    elif variant == 21:
        call_hierarchy_outgoing = destack._generated.query.navigation.call_hierarchy.decode_call_hierarchy_outgoing_response(
            reader
        )

        return QueryResponseCallHierarchyOutgoing(
            call_hierarchy_outgoing=call_hierarchy_outgoing
        )
    elif variant == 22:
        type_hierarchy_item = destack._generated.query.navigation.type_hierarchy.decode_type_hierarchy_item_response(
            reader
        )

        return QueryResponseTypeHierarchyItem(type_hierarchy_item=type_hierarchy_item)
    elif variant == 23:
        type_hierarchy_supertypes = destack._generated.query.navigation.type_hierarchy.decode_type_hierarchy_supertypes_response(
            reader
        )

        return QueryResponseTypeHierarchySupertypes(
            type_hierarchy_supertypes=type_hierarchy_supertypes
        )
    elif variant == 24:
        type_hierarchy_subtypes = destack._generated.query.navigation.type_hierarchy.decode_type_hierarchy_subtypes_response(
            reader
        )

        return QueryResponseTypeHierarchySubtypes(
            type_hierarchy_subtypes=type_hierarchy_subtypes
        )
    elif variant == 25:
        annotations = (
            destack._generated.query.navigation.annotation.decode_annotations_response(
                reader
            )
        )

        return QueryResponseAnnotations(annotations=annotations)
    elif variant == 26:
        rename_target = (
            destack._generated.query.refactor.rename.decode_rename_target_response(
                reader
            )
        )

        return QueryResponseRenameTarget(rename_target=rename_target)
    elif variant == 27:
        rename = destack._generated.query.refactor.rename.decode_rename_response(reader)

        return QueryResponseRename(rename=rename)
    elif variant == 28:
        rename_files = (
            destack._generated.query.refactor.file_rename.decode_rename_files_response(
                reader
            )
        )

        return QueryResponseRenameFiles(rename_files=rename_files)
    elif variant == 29:
        extract_function = destack._generated.query.refactor.extract_function.decode_extract_function_response(
            reader
        )

        return QueryResponseExtractFunction(extract_function=extract_function)
    elif variant == 30:
        extract_variable = destack._generated.query.refactor.extract_variable.decode_extract_variable_response(
            reader
        )

        return QueryResponseExtractVariable(extract_variable=extract_variable)
    elif variant == 31:
        inline = destack._generated.query.refactor.inline.decode_inline_response(reader)

        return QueryResponseInline(inline=inline)
    elif variant == 32:
        change_signature = destack._generated.query.refactor.change_signature.decode_change_signature_response(
            reader
        )

        return QueryResponseChangeSignature(change_signature=change_signature)
    elif variant == 33:
        code_actions = (
            destack._generated.query.refactor.code_action.decode_code_actions_response(
                reader
            )
        )

        return QueryResponseCodeActions(code_actions=code_actions)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_query_response(value: QueryResponse) -> Json:
    """Return one JSON value for one QueryResponse."""
    if value.kind == "completion":
        return {
            "kind": "completion",
            "completion": destack._generated.query.assist.completion.to_json_completion_response(
                value.completion
            ),
        }
    elif value.kind == "hover":
        return {
            "kind": "hover",
            "hover": destack._generated.query.assist.hover.to_json_hover_response(
                value.hover
            ),
        }
    elif value.kind == "signatureHelp":
        return {
            "kind": "signatureHelp",
            "signature_help": destack._generated.query.assist.signature.to_json_signature_help_response(
                value.signature_help
            ),
        }
    elif value.kind == "inlayHints":
        return {
            "kind": "inlayHints",
            "inlay_hints": destack._generated.query.assist.inlay.to_json_inlay_hints_response(
                value.inlay_hints
            ),
        }
    elif value.kind == "codeLenses":
        return {
            "kind": "codeLenses",
            "code_lenses": destack._generated.query.assist.lens.to_json_code_lenses_response(
                value.code_lenses
            ),
        }
    elif value.kind == "resolveCodeLens":
        return {
            "kind": "resolveCodeLens",
            "resolve_code_lens": destack._generated.query.assist.lens.to_json_resolve_code_lens_response(
                value.resolve_code_lens
            ),
        }
    elif value.kind == "foldingRanges":
        return {
            "kind": "foldingRanges",
            "folding_ranges": destack._generated.query.assist.folding.to_json_folding_ranges_response(
                value.folding_ranges
            ),
        }
    elif value.kind == "semanticTokens":
        return {
            "kind": "semanticTokens",
            "semantic_tokens": destack._generated.query.assist.semantic.to_json_semantic_tokens_response(
                value.semantic_tokens
            ),
        }
    elif value.kind == "semanticTokensRange":
        return {
            "kind": "semanticTokensRange",
            "semantic_tokens_range": destack._generated.query.assist.semantic.to_json_semantic_tokens_response(
                value.semantic_tokens_range
            ),
        }
    elif value.kind == "documentSymbols":
        return {
            "kind": "documentSymbols",
            "document_symbols": destack._generated.query.navigation.document_symbol.to_json_document_symbols_response(
                value.document_symbols
            ),
        }
    elif value.kind == "workspaceSymbols":
        return {
            "kind": "workspaceSymbols",
            "workspace_symbols": destack._generated.query.navigation.workspace_symbol.to_json_workspace_symbols_response(
                value.workspace_symbols
            ),
        }
    elif value.kind == "documentLinks":
        return {
            "kind": "documentLinks",
            "document_links": destack._generated.query.navigation.document_link.to_json_document_links_response(
                value.document_links
            ),
        }
    elif value.kind == "documentHighlight":
        return {
            "kind": "documentHighlight",
            "document_highlight": destack._generated.query.navigation.highlight.to_json_document_highlight_response(
                value.document_highlight
            ),
        }
    elif value.kind == "selectionRanges":
        return {
            "kind": "selectionRanges",
            "selection_ranges": destack._generated.query.navigation.selection_range.to_json_selection_ranges_response(
                value.selection_ranges
            ),
        }
    elif value.kind == "gotoDefinition":
        return {
            "kind": "gotoDefinition",
            "goto_definition": destack._generated.query.navigation.definition.to_json_goto_definition_response(
                value.goto_definition
            ),
        }
    elif value.kind == "gotoDeclaration":
        return {
            "kind": "gotoDeclaration",
            "goto_declaration": destack._generated.query.navigation.definition.to_json_goto_declaration_response(
                value.goto_declaration
            ),
        }
    elif value.kind == "gotoTypeDefinition":
        return {
            "kind": "gotoTypeDefinition",
            "goto_type_definition": destack._generated.query.navigation.definition.to_json_goto_type_definition_response(
                value.goto_type_definition
            ),
        }
    elif value.kind == "gotoImplementation":
        return {
            "kind": "gotoImplementation",
            "goto_implementation": destack._generated.query.navigation.implementation.to_json_goto_implementation_response(
                value.goto_implementation
            ),
        }
    elif value.kind == "findReferences":
        return {
            "kind": "findReferences",
            "find_references": destack._generated.query.navigation.find_references.to_json_find_references_response(
                value.find_references
            ),
        }
    elif value.kind == "callHierarchyItem":
        return {
            "kind": "callHierarchyItem",
            "call_hierarchy_item": destack._generated.query.navigation.call_hierarchy.to_json_call_hierarchy_item_response(
                value.call_hierarchy_item
            ),
        }
    elif value.kind == "callHierarchyIncoming":
        return {
            "kind": "callHierarchyIncoming",
            "call_hierarchy_incoming": destack._generated.query.navigation.call_hierarchy.to_json_call_hierarchy_incoming_response(
                value.call_hierarchy_incoming
            ),
        }
    elif value.kind == "callHierarchyOutgoing":
        return {
            "kind": "callHierarchyOutgoing",
            "call_hierarchy_outgoing": destack._generated.query.navigation.call_hierarchy.to_json_call_hierarchy_outgoing_response(
                value.call_hierarchy_outgoing
            ),
        }
    elif value.kind == "typeHierarchyItem":
        return {
            "kind": "typeHierarchyItem",
            "type_hierarchy_item": destack._generated.query.navigation.type_hierarchy.to_json_type_hierarchy_item_response(
                value.type_hierarchy_item
            ),
        }
    elif value.kind == "typeHierarchySupertypes":
        return {
            "kind": "typeHierarchySupertypes",
            "type_hierarchy_supertypes": destack._generated.query.navigation.type_hierarchy.to_json_type_hierarchy_supertypes_response(
                value.type_hierarchy_supertypes
            ),
        }
    elif value.kind == "typeHierarchySubtypes":
        return {
            "kind": "typeHierarchySubtypes",
            "type_hierarchy_subtypes": destack._generated.query.navigation.type_hierarchy.to_json_type_hierarchy_subtypes_response(
                value.type_hierarchy_subtypes
            ),
        }
    elif value.kind == "annotations":
        return {
            "kind": "annotations",
            "annotations": destack._generated.query.navigation.annotation.to_json_annotations_response(
                value.annotations
            ),
        }
    elif value.kind == "renameTarget":
        return {
            "kind": "renameTarget",
            "rename_target": destack._generated.query.refactor.rename.to_json_rename_target_response(
                value.rename_target
            ),
        }
    elif value.kind == "rename":
        return {
            "kind": "rename",
            "rename": destack._generated.query.refactor.rename.to_json_rename_response(
                value.rename
            ),
        }
    elif value.kind == "renameFiles":
        return {
            "kind": "renameFiles",
            "rename_files": destack._generated.query.refactor.file_rename.to_json_rename_files_response(
                value.rename_files
            ),
        }
    elif value.kind == "extractFunction":
        return {
            "kind": "extractFunction",
            "extract_function": destack._generated.query.refactor.extract_function.to_json_extract_function_response(
                value.extract_function
            ),
        }
    elif value.kind == "extractVariable":
        return {
            "kind": "extractVariable",
            "extract_variable": destack._generated.query.refactor.extract_variable.to_json_extract_variable_response(
                value.extract_variable
            ),
        }
    elif value.kind == "inline":
        return {
            "kind": "inline",
            "inline": destack._generated.query.refactor.inline.to_json_inline_response(
                value.inline
            ),
        }
    elif value.kind == "changeSignature":
        return {
            "kind": "changeSignature",
            "change_signature": destack._generated.query.refactor.change_signature.to_json_change_signature_response(
                value.change_signature
            ),
        }
    elif value.kind == "codeActions":
        return {
            "kind": "codeActions",
            "code_actions": destack._generated.query.refactor.code_action.to_json_code_actions_response(
                value.code_actions
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_query_response(value: Json) -> QueryResponse:
    """Return one QueryResponse from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "completion":
        return QueryResponseCompletion(
            completion=destack._generated.query.assist.completion.from_json_completion_response(
                json_field(object_, "completion")
            )
        )
    elif kind == "hover":
        return QueryResponseHover(
            hover=destack._generated.query.assist.hover.from_json_hover_response(
                json_field(object_, "hover")
            )
        )
    elif kind == "signatureHelp":
        return QueryResponseSignatureHelp(
            signature_help=destack._generated.query.assist.signature.from_json_signature_help_response(
                json_field(object_, "signature_help")
            )
        )
    elif kind == "inlayHints":
        return QueryResponseInlayHints(
            inlay_hints=destack._generated.query.assist.inlay.from_json_inlay_hints_response(
                json_field(object_, "inlay_hints")
            )
        )
    elif kind == "codeLenses":
        return QueryResponseCodeLenses(
            code_lenses=destack._generated.query.assist.lens.from_json_code_lenses_response(
                json_field(object_, "code_lenses")
            )
        )
    elif kind == "resolveCodeLens":
        return QueryResponseResolveCodeLens(
            resolve_code_lens=destack._generated.query.assist.lens.from_json_resolve_code_lens_response(
                json_field(object_, "resolve_code_lens")
            )
        )
    elif kind == "foldingRanges":
        return QueryResponseFoldingRanges(
            folding_ranges=destack._generated.query.assist.folding.from_json_folding_ranges_response(
                json_field(object_, "folding_ranges")
            )
        )
    elif kind == "semanticTokens":
        return QueryResponseSemanticTokens(
            semantic_tokens=destack._generated.query.assist.semantic.from_json_semantic_tokens_response(
                json_field(object_, "semantic_tokens")
            )
        )
    elif kind == "semanticTokensRange":
        return QueryResponseSemanticTokensRange(
            semantic_tokens_range=destack._generated.query.assist.semantic.from_json_semantic_tokens_response(
                json_field(object_, "semantic_tokens_range")
            )
        )
    elif kind == "documentSymbols":
        return QueryResponseDocumentSymbols(
            document_symbols=destack._generated.query.navigation.document_symbol.from_json_document_symbols_response(
                json_field(object_, "document_symbols")
            )
        )
    elif kind == "workspaceSymbols":
        return QueryResponseWorkspaceSymbols(
            workspace_symbols=destack._generated.query.navigation.workspace_symbol.from_json_workspace_symbols_response(
                json_field(object_, "workspace_symbols")
            )
        )
    elif kind == "documentLinks":
        return QueryResponseDocumentLinks(
            document_links=destack._generated.query.navigation.document_link.from_json_document_links_response(
                json_field(object_, "document_links")
            )
        )
    elif kind == "documentHighlight":
        return QueryResponseDocumentHighlight(
            document_highlight=destack._generated.query.navigation.highlight.from_json_document_highlight_response(
                json_field(object_, "document_highlight")
            )
        )
    elif kind == "selectionRanges":
        return QueryResponseSelectionRanges(
            selection_ranges=destack._generated.query.navigation.selection_range.from_json_selection_ranges_response(
                json_field(object_, "selection_ranges")
            )
        )
    elif kind == "gotoDefinition":
        return QueryResponseGotoDefinition(
            goto_definition=destack._generated.query.navigation.definition.from_json_goto_definition_response(
                json_field(object_, "goto_definition")
            )
        )
    elif kind == "gotoDeclaration":
        return QueryResponseGotoDeclaration(
            goto_declaration=destack._generated.query.navigation.definition.from_json_goto_declaration_response(
                json_field(object_, "goto_declaration")
            )
        )
    elif kind == "gotoTypeDefinition":
        return QueryResponseGotoTypeDefinition(
            goto_type_definition=destack._generated.query.navigation.definition.from_json_goto_type_definition_response(
                json_field(object_, "goto_type_definition")
            )
        )
    elif kind == "gotoImplementation":
        return QueryResponseGotoImplementation(
            goto_implementation=destack._generated.query.navigation.implementation.from_json_goto_implementation_response(
                json_field(object_, "goto_implementation")
            )
        )
    elif kind == "findReferences":
        return QueryResponseFindReferences(
            find_references=destack._generated.query.navigation.find_references.from_json_find_references_response(
                json_field(object_, "find_references")
            )
        )
    elif kind == "callHierarchyItem":
        return QueryResponseCallHierarchyItem(
            call_hierarchy_item=destack._generated.query.navigation.call_hierarchy.from_json_call_hierarchy_item_response(
                json_field(object_, "call_hierarchy_item")
            )
        )
    elif kind == "callHierarchyIncoming":
        return QueryResponseCallHierarchyIncoming(
            call_hierarchy_incoming=destack._generated.query.navigation.call_hierarchy.from_json_call_hierarchy_incoming_response(
                json_field(object_, "call_hierarchy_incoming")
            )
        )
    elif kind == "callHierarchyOutgoing":
        return QueryResponseCallHierarchyOutgoing(
            call_hierarchy_outgoing=destack._generated.query.navigation.call_hierarchy.from_json_call_hierarchy_outgoing_response(
                json_field(object_, "call_hierarchy_outgoing")
            )
        )
    elif kind == "typeHierarchyItem":
        return QueryResponseTypeHierarchyItem(
            type_hierarchy_item=destack._generated.query.navigation.type_hierarchy.from_json_type_hierarchy_item_response(
                json_field(object_, "type_hierarchy_item")
            )
        )
    elif kind == "typeHierarchySupertypes":
        return QueryResponseTypeHierarchySupertypes(
            type_hierarchy_supertypes=destack._generated.query.navigation.type_hierarchy.from_json_type_hierarchy_supertypes_response(
                json_field(object_, "type_hierarchy_supertypes")
            )
        )
    elif kind == "typeHierarchySubtypes":
        return QueryResponseTypeHierarchySubtypes(
            type_hierarchy_subtypes=destack._generated.query.navigation.type_hierarchy.from_json_type_hierarchy_subtypes_response(
                json_field(object_, "type_hierarchy_subtypes")
            )
        )
    elif kind == "annotations":
        return QueryResponseAnnotations(
            annotations=destack._generated.query.navigation.annotation.from_json_annotations_response(
                json_field(object_, "annotations")
            )
        )
    elif kind == "renameTarget":
        return QueryResponseRenameTarget(
            rename_target=destack._generated.query.refactor.rename.from_json_rename_target_response(
                json_field(object_, "rename_target")
            )
        )
    elif kind == "rename":
        return QueryResponseRename(
            rename=destack._generated.query.refactor.rename.from_json_rename_response(
                json_field(object_, "rename")
            )
        )
    elif kind == "renameFiles":
        return QueryResponseRenameFiles(
            rename_files=destack._generated.query.refactor.file_rename.from_json_rename_files_response(
                json_field(object_, "rename_files")
            )
        )
    elif kind == "extractFunction":
        return QueryResponseExtractFunction(
            extract_function=destack._generated.query.refactor.extract_function.from_json_extract_function_response(
                json_field(object_, "extract_function")
            )
        )
    elif kind == "extractVariable":
        return QueryResponseExtractVariable(
            extract_variable=destack._generated.query.refactor.extract_variable.from_json_extract_variable_response(
                json_field(object_, "extract_variable")
            )
        )
    elif kind == "inline":
        return QueryResponseInline(
            inline=destack._generated.query.refactor.inline.from_json_inline_response(
                json_field(object_, "inline")
            )
        )
    elif kind == "changeSignature":
        return QueryResponseChangeSignature(
            change_signature=destack._generated.query.refactor.change_signature.from_json_change_signature_response(
                json_field(object_, "change_signature")
            )
        )
    elif kind == "codeActions":
        return QueryResponseCodeActions(
            code_actions=destack._generated.query.refactor.code_action.from_json_code_actions_response(
                json_field(object_, "code_actions")
            )
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


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
