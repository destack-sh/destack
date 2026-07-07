# generated client target, do not edit

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
class QueryRequestOutline:
    """Symbols request payload."""

    outline: destack._generated.query.navigation.symbol.OutlineRequest
    kind: typing.Literal["outline"] = "outline"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request(self)


@dataclass(frozen=True, slots=True)
class QueryRequestSymbolSearch:
    """Symbol search request payload."""

    symbol_search: destack._generated.query.navigation.workspace.SymbolSearchRequest
    kind: typing.Literal["symbolSearch"] = "symbolSearch"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request(self)


@dataclass(frozen=True, slots=True)
class QueryRequestLinks:
    """Links request payload."""

    links: destack._generated.query.navigation.link.LinksRequest
    kind: typing.Literal["links"] = "links"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request(self)


@dataclass(frozen=True, slots=True)
class QueryRequestHighlight:
    """Highlight request payload."""

    highlight: destack._generated.query.navigation.highlight.HighlightRequest
    kind: typing.Literal["highlight"] = "highlight"

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
        destack._generated.query.navigation.selection.SelectionRangesRequest
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

    find_references: destack._generated.query.navigation.reference.FindReferencesRequest
    kind: typing.Literal["findReferences"] = "findReferences"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request(self)


@dataclass(frozen=True, slots=True)
class QueryRequestCallItem:
    """Call item request payload."""

    call_item: destack._generated.query.navigation.call.CallItemRequest
    kind: typing.Literal["callItem"] = "callItem"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request(self)


@dataclass(frozen=True, slots=True)
class QueryRequestIncomingCalls:
    """Incoming calls request payload."""

    incoming_calls: destack._generated.query.navigation.call.IncomingCallsRequest
    kind: typing.Literal["incomingCalls"] = "incomingCalls"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request(self)


@dataclass(frozen=True, slots=True)
class QueryRequestOutgoingCalls:
    """Outgoing calls request payload."""

    outgoing_calls: destack._generated.query.navigation.call.OutgoingCallsRequest
    kind: typing.Literal["outgoingCalls"] = "outgoingCalls"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request(self)


@dataclass(frozen=True, slots=True)
class QueryRequestTypeItem:
    """Type hierarchy item request payload."""

    type_item: destack._generated.query.navigation.type.TypeItemRequest
    kind: typing.Literal["typeItem"] = "typeItem"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request(self)


@dataclass(frozen=True, slots=True)
class QueryRequestSupertypes:
    """Type hierarchy supertypes request payload."""

    supertypes: destack._generated.query.navigation.type.SupertypesRequest
    kind: typing.Literal["supertypes"] = "supertypes"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request(self)


@dataclass(frozen=True, slots=True)
class QueryRequestSubtypes:
    """Type hierarchy subtypes request payload."""

    subtypes: destack._generated.query.navigation.type.SubtypesRequest
    kind: typing.Literal["subtypes"] = "subtypes"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request(self)


@dataclass(frozen=True, slots=True)
class QueryRequestDecorators:
    """Decorator request payload."""

    decorators: destack._generated.query.navigation.decorator.DecoratorsRequest
    kind: typing.Literal["decorators"] = "decorators"

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

    rename_files: destack._generated.query.refactor.file.RenameFilesRequest
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

    extract_function: destack._generated.query.refactor.function.ExtractFunctionRequest
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

    extract_variable: destack._generated.query.refactor.variable.ExtractVariableRequest
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

    change_signature: destack._generated.query.refactor.signature.ChangeSignatureRequest
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

    code_actions: destack._generated.query.refactor.action.CodeActionsRequest
    kind: typing.Literal["codeActions"] = "codeActions"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request(self)


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


def encode_query_request(writer: BinaryWriter, value: QueryRequest) -> None:
    """Encode one QueryRequest."""
    if value.kind == "completion":
        writer.write_unsigned(0)
        destack._generated.query.completion.item.encode_completion_request(
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
    elif value.kind == "outline":
        writer.write_unsigned(9)
        destack._generated.query.navigation.symbol.encode_outline_request(
            writer, value.outline
        )
    elif value.kind == "symbolSearch":
        writer.write_unsigned(10)
        destack._generated.query.navigation.workspace.encode_symbol_search_request(
            writer, value.symbol_search
        )
    elif value.kind == "links":
        writer.write_unsigned(11)
        destack._generated.query.navigation.link.encode_links_request(
            writer, value.links
        )
    elif value.kind == "highlight":
        writer.write_unsigned(12)
        destack._generated.query.navigation.highlight.encode_highlight_request(
            writer, value.highlight
        )
    elif value.kind == "selectionRanges":
        writer.write_unsigned(13)
        destack._generated.query.navigation.selection.encode_selection_ranges_request(
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
        destack._generated.query.navigation.reference.encode_find_references_request(
            writer, value.find_references
        )
    elif value.kind == "callItem":
        writer.write_unsigned(19)
        destack._generated.query.navigation.call.encode_call_item_request(
            writer, value.call_item
        )
    elif value.kind == "incomingCalls":
        writer.write_unsigned(20)
        destack._generated.query.navigation.call.encode_incoming_calls_request(
            writer, value.incoming_calls
        )
    elif value.kind == "outgoingCalls":
        writer.write_unsigned(21)
        destack._generated.query.navigation.call.encode_outgoing_calls_request(
            writer, value.outgoing_calls
        )
    elif value.kind == "typeItem":
        writer.write_unsigned(22)
        destack._generated.query.navigation.type.encode_type_item_request(
            writer, value.type_item
        )
    elif value.kind == "supertypes":
        writer.write_unsigned(23)
        destack._generated.query.navigation.type.encode_supertypes_request(
            writer, value.supertypes
        )
    elif value.kind == "subtypes":
        writer.write_unsigned(24)
        destack._generated.query.navigation.type.encode_subtypes_request(
            writer, value.subtypes
        )
    elif value.kind == "decorators":
        writer.write_unsigned(25)
        destack._generated.query.navigation.decorator.encode_decorators_request(
            writer, value.decorators
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
        destack._generated.query.refactor.file.encode_rename_files_request(
            writer, value.rename_files
        )
    elif value.kind == "extractFunction":
        writer.write_unsigned(29)
        destack._generated.query.refactor.function.encode_extract_function_request(
            writer, value.extract_function
        )
    elif value.kind == "extractVariable":
        writer.write_unsigned(30)
        destack._generated.query.refactor.variable.encode_extract_variable_request(
            writer, value.extract_variable
        )
    elif value.kind == "inline":
        writer.write_unsigned(31)
        destack._generated.query.refactor.inline.encode_inline_request(
            writer, value.inline
        )
    elif value.kind == "changeSignature":
        writer.write_unsigned(32)
        destack._generated.query.refactor.signature.encode_change_signature_request(
            writer, value.change_signature
        )
    elif value.kind == "codeActions":
        writer.write_unsigned(33)
        destack._generated.query.refactor.action.encode_code_actions_request(
            writer, value.code_actions
        )
    else:
        raise SerdeError("unknown enum variant")


def decode_query_request(reader: BinaryReader) -> QueryRequest:
    """Decode one QueryRequest."""
    variant = reader.read_number()

    if variant == 0:
        completion = destack._generated.query.completion.item.decode_completion_request(
            reader
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
        outline = destack._generated.query.navigation.symbol.decode_outline_request(
            reader
        )

        return QueryRequestOutline(outline=outline)
    elif variant == 10:
        symbol_search = (
            destack._generated.query.navigation.workspace.decode_symbol_search_request(
                reader
            )
        )

        return QueryRequestSymbolSearch(symbol_search=symbol_search)
    elif variant == 11:
        links = destack._generated.query.navigation.link.decode_links_request(reader)

        return QueryRequestLinks(links=links)
    elif variant == 12:
        highlight = (
            destack._generated.query.navigation.highlight.decode_highlight_request(
                reader
            )
        )

        return QueryRequestHighlight(highlight=highlight)
    elif variant == 13:
        selection_ranges = destack._generated.query.navigation.selection.decode_selection_ranges_request(
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
        find_references = destack._generated.query.navigation.reference.decode_find_references_request(
            reader
        )

        return QueryRequestFindReferences(find_references=find_references)
    elif variant == 19:
        call_item = destack._generated.query.navigation.call.decode_call_item_request(
            reader
        )

        return QueryRequestCallItem(call_item=call_item)
    elif variant == 20:
        incoming_calls = (
            destack._generated.query.navigation.call.decode_incoming_calls_request(
                reader
            )
        )

        return QueryRequestIncomingCalls(incoming_calls=incoming_calls)
    elif variant == 21:
        outgoing_calls = (
            destack._generated.query.navigation.call.decode_outgoing_calls_request(
                reader
            )
        )

        return QueryRequestOutgoingCalls(outgoing_calls=outgoing_calls)
    elif variant == 22:
        type_item = destack._generated.query.navigation.type.decode_type_item_request(
            reader
        )

        return QueryRequestTypeItem(type_item=type_item)
    elif variant == 23:
        supertypes = destack._generated.query.navigation.type.decode_supertypes_request(
            reader
        )

        return QueryRequestSupertypes(supertypes=supertypes)
    elif variant == 24:
        subtypes = destack._generated.query.navigation.type.decode_subtypes_request(
            reader
        )

        return QueryRequestSubtypes(subtypes=subtypes)
    elif variant == 25:
        decorators = (
            destack._generated.query.navigation.decorator.decode_decorators_request(
                reader
            )
        )

        return QueryRequestDecorators(decorators=decorators)
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
            destack._generated.query.refactor.file.decode_rename_files_request(reader)
        )

        return QueryRequestRenameFiles(rename_files=rename_files)
    elif variant == 29:
        extract_function = (
            destack._generated.query.refactor.function.decode_extract_function_request(
                reader
            )
        )

        return QueryRequestExtractFunction(extract_function=extract_function)
    elif variant == 30:
        extract_variable = (
            destack._generated.query.refactor.variable.decode_extract_variable_request(
                reader
            )
        )

        return QueryRequestExtractVariable(extract_variable=extract_variable)
    elif variant == 31:
        inline = destack._generated.query.refactor.inline.decode_inline_request(reader)

        return QueryRequestInline(inline=inline)
    elif variant == 32:
        change_signature = (
            destack._generated.query.refactor.signature.decode_change_signature_request(
                reader
            )
        )

        return QueryRequestChangeSignature(change_signature=change_signature)
    elif variant == 33:
        code_actions = (
            destack._generated.query.refactor.action.decode_code_actions_request(reader)
        )

        return QueryRequestCodeActions(code_actions=code_actions)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_query_request(value: QueryRequest) -> Json:
    """Return one JSON value for one QueryRequest."""
    if value.kind == "completion":
        return {
            "kind": "completion",
            "completion": destack._generated.query.completion.item.to_json_completion_request(
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
    elif value.kind == "outline":
        return {
            "kind": "outline",
            "outline": destack._generated.query.navigation.symbol.to_json_outline_request(
                value.outline
            ),
        }
    elif value.kind == "symbolSearch":
        return {
            "kind": "symbolSearch",
            "symbol_search": destack._generated.query.navigation.workspace.to_json_symbol_search_request(
                value.symbol_search
            ),
        }
    elif value.kind == "links":
        return {
            "kind": "links",
            "links": destack._generated.query.navigation.link.to_json_links_request(
                value.links
            ),
        }
    elif value.kind == "highlight":
        return {
            "kind": "highlight",
            "highlight": destack._generated.query.navigation.highlight.to_json_highlight_request(
                value.highlight
            ),
        }
    elif value.kind == "selectionRanges":
        return {
            "kind": "selectionRanges",
            "selection_ranges": destack._generated.query.navigation.selection.to_json_selection_ranges_request(
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
            "find_references": destack._generated.query.navigation.reference.to_json_find_references_request(
                value.find_references
            ),
        }
    elif value.kind == "callItem":
        return {
            "kind": "callItem",
            "call_item": destack._generated.query.navigation.call.to_json_call_item_request(
                value.call_item
            ),
        }
    elif value.kind == "incomingCalls":
        return {
            "kind": "incomingCalls",
            "incoming_calls": destack._generated.query.navigation.call.to_json_incoming_calls_request(
                value.incoming_calls
            ),
        }
    elif value.kind == "outgoingCalls":
        return {
            "kind": "outgoingCalls",
            "outgoing_calls": destack._generated.query.navigation.call.to_json_outgoing_calls_request(
                value.outgoing_calls
            ),
        }
    elif value.kind == "typeItem":
        return {
            "kind": "typeItem",
            "type_item": destack._generated.query.navigation.type.to_json_type_item_request(
                value.type_item
            ),
        }
    elif value.kind == "supertypes":
        return {
            "kind": "supertypes",
            "supertypes": destack._generated.query.navigation.type.to_json_supertypes_request(
                value.supertypes
            ),
        }
    elif value.kind == "subtypes":
        return {
            "kind": "subtypes",
            "subtypes": destack._generated.query.navigation.type.to_json_subtypes_request(
                value.subtypes
            ),
        }
    elif value.kind == "decorators":
        return {
            "kind": "decorators",
            "decorators": destack._generated.query.navigation.decorator.to_json_decorators_request(
                value.decorators
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
            "rename_files": destack._generated.query.refactor.file.to_json_rename_files_request(
                value.rename_files
            ),
        }
    elif value.kind == "extractFunction":
        return {
            "kind": "extractFunction",
            "extract_function": destack._generated.query.refactor.function.to_json_extract_function_request(
                value.extract_function
            ),
        }
    elif value.kind == "extractVariable":
        return {
            "kind": "extractVariable",
            "extract_variable": destack._generated.query.refactor.variable.to_json_extract_variable_request(
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
            "change_signature": destack._generated.query.refactor.signature.to_json_change_signature_request(
                value.change_signature
            ),
        }
    elif value.kind == "codeActions":
        return {
            "kind": "codeActions",
            "code_actions": destack._generated.query.refactor.action.to_json_code_actions_request(
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
            completion=destack._generated.query.completion.item.from_json_completion_request(
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
    elif kind == "outline":
        return QueryRequestOutline(
            outline=destack._generated.query.navigation.symbol.from_json_outline_request(
                json_field(object_, "outline")
            )
        )
    elif kind == "symbolSearch":
        return QueryRequestSymbolSearch(
            symbol_search=destack._generated.query.navigation.workspace.from_json_symbol_search_request(
                json_field(object_, "symbol_search")
            )
        )
    elif kind == "links":
        return QueryRequestLinks(
            links=destack._generated.query.navigation.link.from_json_links_request(
                json_field(object_, "links")
            )
        )
    elif kind == "highlight":
        return QueryRequestHighlight(
            highlight=destack._generated.query.navigation.highlight.from_json_highlight_request(
                json_field(object_, "highlight")
            )
        )
    elif kind == "selectionRanges":
        return QueryRequestSelectionRanges(
            selection_ranges=destack._generated.query.navigation.selection.from_json_selection_ranges_request(
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
            find_references=destack._generated.query.navigation.reference.from_json_find_references_request(
                json_field(object_, "find_references")
            )
        )
    elif kind == "callItem":
        return QueryRequestCallItem(
            call_item=destack._generated.query.navigation.call.from_json_call_item_request(
                json_field(object_, "call_item")
            )
        )
    elif kind == "incomingCalls":
        return QueryRequestIncomingCalls(
            incoming_calls=destack._generated.query.navigation.call.from_json_incoming_calls_request(
                json_field(object_, "incoming_calls")
            )
        )
    elif kind == "outgoingCalls":
        return QueryRequestOutgoingCalls(
            outgoing_calls=destack._generated.query.navigation.call.from_json_outgoing_calls_request(
                json_field(object_, "outgoing_calls")
            )
        )
    elif kind == "typeItem":
        return QueryRequestTypeItem(
            type_item=destack._generated.query.navigation.type.from_json_type_item_request(
                json_field(object_, "type_item")
            )
        )
    elif kind == "supertypes":
        return QueryRequestSupertypes(
            supertypes=destack._generated.query.navigation.type.from_json_supertypes_request(
                json_field(object_, "supertypes")
            )
        )
    elif kind == "subtypes":
        return QueryRequestSubtypes(
            subtypes=destack._generated.query.navigation.type.from_json_subtypes_request(
                json_field(object_, "subtypes")
            )
        )
    elif kind == "decorators":
        return QueryRequestDecorators(
            decorators=destack._generated.query.navigation.decorator.from_json_decorators_request(
                json_field(object_, "decorators")
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
            rename_files=destack._generated.query.refactor.file.from_json_rename_files_request(
                json_field(object_, "rename_files")
            )
        )
    elif kind == "extractFunction":
        return QueryRequestExtractFunction(
            extract_function=destack._generated.query.refactor.function.from_json_extract_function_request(
                json_field(object_, "extract_function")
            )
        )
    elif kind == "extractVariable":
        return QueryRequestExtractVariable(
            extract_variable=destack._generated.query.refactor.variable.from_json_extract_variable_request(
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
            change_signature=destack._generated.query.refactor.signature.from_json_change_signature_request(
                json_field(object_, "change_signature")
            )
        )
    elif kind == "codeActions":
        return QueryRequestCodeActions(
            code_actions=destack._generated.query.refactor.action.from_json_code_actions_request(
                json_field(object_, "code_actions")
            )
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class QueryResponseCompletion:
    """Completion response payload."""

    completion: destack._generated.query.completion.item.CompletionResponse
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
class QueryResponseOutline:
    """Symbols response payload."""

    outline: destack._generated.query.navigation.symbol.OutlineResponse
    kind: typing.Literal["outline"] = "outline"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response(self)


@dataclass(frozen=True, slots=True)
class QueryResponseSymbolSearch:
    """Symbol search response payload."""

    symbol_search: destack._generated.query.navigation.workspace.SymbolSearchResponse
    kind: typing.Literal["symbolSearch"] = "symbolSearch"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response(self)


@dataclass(frozen=True, slots=True)
class QueryResponseLinks:
    """Links response payload."""

    links: destack._generated.query.navigation.link.LinksResponse
    kind: typing.Literal["links"] = "links"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response(self)


@dataclass(frozen=True, slots=True)
class QueryResponseHighlight:
    """Highlight response payload."""

    highlight: destack._generated.query.navigation.highlight.HighlightResponse
    kind: typing.Literal["highlight"] = "highlight"

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
        destack._generated.query.navigation.selection.SelectionRangesResponse
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
        destack._generated.query.navigation.reference.FindReferencesResponse
    )
    kind: typing.Literal["findReferences"] = "findReferences"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response(self)


@dataclass(frozen=True, slots=True)
class QueryResponseCallItem:
    """Call item response payload."""

    call_item: destack._generated.query.navigation.call.CallItemResponse
    kind: typing.Literal["callItem"] = "callItem"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response(self)


@dataclass(frozen=True, slots=True)
class QueryResponseIncomingCalls:
    """Incoming calls response payload."""

    incoming_calls: destack._generated.query.navigation.call.IncomingCallsResponse
    kind: typing.Literal["incomingCalls"] = "incomingCalls"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response(self)


@dataclass(frozen=True, slots=True)
class QueryResponseOutgoingCalls:
    """Outgoing calls response payload."""

    outgoing_calls: destack._generated.query.navigation.call.OutgoingCallsResponse
    kind: typing.Literal["outgoingCalls"] = "outgoingCalls"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response(self)


@dataclass(frozen=True, slots=True)
class QueryResponseTypeItem:
    """Type hierarchy item response payload."""

    type_item: destack._generated.query.navigation.type.TypeItemResponse
    kind: typing.Literal["typeItem"] = "typeItem"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response(self)


@dataclass(frozen=True, slots=True)
class QueryResponseSupertypes:
    """Type hierarchy supertypes response payload."""

    supertypes: destack._generated.query.navigation.type.SupertypesResponse
    kind: typing.Literal["supertypes"] = "supertypes"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response(self)


@dataclass(frozen=True, slots=True)
class QueryResponseSubtypes:
    """Type hierarchy subtypes response payload."""

    subtypes: destack._generated.query.navigation.type.SubtypesResponse
    kind: typing.Literal["subtypes"] = "subtypes"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response(self)


@dataclass(frozen=True, slots=True)
class QueryResponseDecorators:
    """Decorator response payload."""

    decorators: destack._generated.query.navigation.decorator.DecoratorsResponse
    kind: typing.Literal["decorators"] = "decorators"

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

    rename_files: destack._generated.query.refactor.file.RenameFilesResponse
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

    extract_function: destack._generated.query.refactor.function.ExtractFunctionResponse
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

    extract_variable: destack._generated.query.refactor.variable.ExtractVariableResponse
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
        destack._generated.query.refactor.signature.ChangeSignatureResponse
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

    code_actions: destack._generated.query.refactor.action.CodeActionsResponse
    kind: typing.Literal["codeActions"] = "codeActions"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response(self)


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


def encode_query_response(writer: BinaryWriter, value: QueryResponse) -> None:
    """Encode one QueryResponse."""
    if value.kind == "completion":
        writer.write_unsigned(0)
        destack._generated.query.completion.item.encode_completion_response(
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
    elif value.kind == "outline":
        writer.write_unsigned(9)
        destack._generated.query.navigation.symbol.encode_outline_response(
            writer, value.outline
        )
    elif value.kind == "symbolSearch":
        writer.write_unsigned(10)
        destack._generated.query.navigation.workspace.encode_symbol_search_response(
            writer, value.symbol_search
        )
    elif value.kind == "links":
        writer.write_unsigned(11)
        destack._generated.query.navigation.link.encode_links_response(
            writer, value.links
        )
    elif value.kind == "highlight":
        writer.write_unsigned(12)
        destack._generated.query.navigation.highlight.encode_highlight_response(
            writer, value.highlight
        )
    elif value.kind == "selectionRanges":
        writer.write_unsigned(13)
        destack._generated.query.navigation.selection.encode_selection_ranges_response(
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
        destack._generated.query.navigation.reference.encode_find_references_response(
            writer, value.find_references
        )
    elif value.kind == "callItem":
        writer.write_unsigned(19)
        destack._generated.query.navigation.call.encode_call_item_response(
            writer, value.call_item
        )
    elif value.kind == "incomingCalls":
        writer.write_unsigned(20)
        destack._generated.query.navigation.call.encode_incoming_calls_response(
            writer, value.incoming_calls
        )
    elif value.kind == "outgoingCalls":
        writer.write_unsigned(21)
        destack._generated.query.navigation.call.encode_outgoing_calls_response(
            writer, value.outgoing_calls
        )
    elif value.kind == "typeItem":
        writer.write_unsigned(22)
        destack._generated.query.navigation.type.encode_type_item_response(
            writer, value.type_item
        )
    elif value.kind == "supertypes":
        writer.write_unsigned(23)
        destack._generated.query.navigation.type.encode_supertypes_response(
            writer, value.supertypes
        )
    elif value.kind == "subtypes":
        writer.write_unsigned(24)
        destack._generated.query.navigation.type.encode_subtypes_response(
            writer, value.subtypes
        )
    elif value.kind == "decorators":
        writer.write_unsigned(25)
        destack._generated.query.navigation.decorator.encode_decorators_response(
            writer, value.decorators
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
        destack._generated.query.refactor.file.encode_rename_files_response(
            writer, value.rename_files
        )
    elif value.kind == "extractFunction":
        writer.write_unsigned(29)
        destack._generated.query.refactor.function.encode_extract_function_response(
            writer, value.extract_function
        )
    elif value.kind == "extractVariable":
        writer.write_unsigned(30)
        destack._generated.query.refactor.variable.encode_extract_variable_response(
            writer, value.extract_variable
        )
    elif value.kind == "inline":
        writer.write_unsigned(31)
        destack._generated.query.refactor.inline.encode_inline_response(
            writer, value.inline
        )
    elif value.kind == "changeSignature":
        writer.write_unsigned(32)
        destack._generated.query.refactor.signature.encode_change_signature_response(
            writer, value.change_signature
        )
    elif value.kind == "codeActions":
        writer.write_unsigned(33)
        destack._generated.query.refactor.action.encode_code_actions_response(
            writer, value.code_actions
        )
    else:
        raise SerdeError("unknown enum variant")


def decode_query_response(reader: BinaryReader) -> QueryResponse:
    """Decode one QueryResponse."""
    variant = reader.read_number()

    if variant == 0:
        completion = (
            destack._generated.query.completion.item.decode_completion_response(reader)
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
        outline = destack._generated.query.navigation.symbol.decode_outline_response(
            reader
        )

        return QueryResponseOutline(outline=outline)
    elif variant == 10:
        symbol_search = (
            destack._generated.query.navigation.workspace.decode_symbol_search_response(
                reader
            )
        )

        return QueryResponseSymbolSearch(symbol_search=symbol_search)
    elif variant == 11:
        links = destack._generated.query.navigation.link.decode_links_response(reader)

        return QueryResponseLinks(links=links)
    elif variant == 12:
        highlight = (
            destack._generated.query.navigation.highlight.decode_highlight_response(
                reader
            )
        )

        return QueryResponseHighlight(highlight=highlight)
    elif variant == 13:
        selection_ranges = destack._generated.query.navigation.selection.decode_selection_ranges_response(
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
        find_references = destack._generated.query.navigation.reference.decode_find_references_response(
            reader
        )

        return QueryResponseFindReferences(find_references=find_references)
    elif variant == 19:
        call_item = destack._generated.query.navigation.call.decode_call_item_response(
            reader
        )

        return QueryResponseCallItem(call_item=call_item)
    elif variant == 20:
        incoming_calls = (
            destack._generated.query.navigation.call.decode_incoming_calls_response(
                reader
            )
        )

        return QueryResponseIncomingCalls(incoming_calls=incoming_calls)
    elif variant == 21:
        outgoing_calls = (
            destack._generated.query.navigation.call.decode_outgoing_calls_response(
                reader
            )
        )

        return QueryResponseOutgoingCalls(outgoing_calls=outgoing_calls)
    elif variant == 22:
        type_item = destack._generated.query.navigation.type.decode_type_item_response(
            reader
        )

        return QueryResponseTypeItem(type_item=type_item)
    elif variant == 23:
        supertypes = (
            destack._generated.query.navigation.type.decode_supertypes_response(reader)
        )

        return QueryResponseSupertypes(supertypes=supertypes)
    elif variant == 24:
        subtypes = destack._generated.query.navigation.type.decode_subtypes_response(
            reader
        )

        return QueryResponseSubtypes(subtypes=subtypes)
    elif variant == 25:
        decorators = (
            destack._generated.query.navigation.decorator.decode_decorators_response(
                reader
            )
        )

        return QueryResponseDecorators(decorators=decorators)
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
            destack._generated.query.refactor.file.decode_rename_files_response(reader)
        )

        return QueryResponseRenameFiles(rename_files=rename_files)
    elif variant == 29:
        extract_function = (
            destack._generated.query.refactor.function.decode_extract_function_response(
                reader
            )
        )

        return QueryResponseExtractFunction(extract_function=extract_function)
    elif variant == 30:
        extract_variable = (
            destack._generated.query.refactor.variable.decode_extract_variable_response(
                reader
            )
        )

        return QueryResponseExtractVariable(extract_variable=extract_variable)
    elif variant == 31:
        inline = destack._generated.query.refactor.inline.decode_inline_response(reader)

        return QueryResponseInline(inline=inline)
    elif variant == 32:
        change_signature = destack._generated.query.refactor.signature.decode_change_signature_response(
            reader
        )

        return QueryResponseChangeSignature(change_signature=change_signature)
    elif variant == 33:
        code_actions = (
            destack._generated.query.refactor.action.decode_code_actions_response(
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
            "completion": destack._generated.query.completion.item.to_json_completion_response(
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
    elif value.kind == "outline":
        return {
            "kind": "outline",
            "outline": destack._generated.query.navigation.symbol.to_json_outline_response(
                value.outline
            ),
        }
    elif value.kind == "symbolSearch":
        return {
            "kind": "symbolSearch",
            "symbol_search": destack._generated.query.navigation.workspace.to_json_symbol_search_response(
                value.symbol_search
            ),
        }
    elif value.kind == "links":
        return {
            "kind": "links",
            "links": destack._generated.query.navigation.link.to_json_links_response(
                value.links
            ),
        }
    elif value.kind == "highlight":
        return {
            "kind": "highlight",
            "highlight": destack._generated.query.navigation.highlight.to_json_highlight_response(
                value.highlight
            ),
        }
    elif value.kind == "selectionRanges":
        return {
            "kind": "selectionRanges",
            "selection_ranges": destack._generated.query.navigation.selection.to_json_selection_ranges_response(
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
            "find_references": destack._generated.query.navigation.reference.to_json_find_references_response(
                value.find_references
            ),
        }
    elif value.kind == "callItem":
        return {
            "kind": "callItem",
            "call_item": destack._generated.query.navigation.call.to_json_call_item_response(
                value.call_item
            ),
        }
    elif value.kind == "incomingCalls":
        return {
            "kind": "incomingCalls",
            "incoming_calls": destack._generated.query.navigation.call.to_json_incoming_calls_response(
                value.incoming_calls
            ),
        }
    elif value.kind == "outgoingCalls":
        return {
            "kind": "outgoingCalls",
            "outgoing_calls": destack._generated.query.navigation.call.to_json_outgoing_calls_response(
                value.outgoing_calls
            ),
        }
    elif value.kind == "typeItem":
        return {
            "kind": "typeItem",
            "type_item": destack._generated.query.navigation.type.to_json_type_item_response(
                value.type_item
            ),
        }
    elif value.kind == "supertypes":
        return {
            "kind": "supertypes",
            "supertypes": destack._generated.query.navigation.type.to_json_supertypes_response(
                value.supertypes
            ),
        }
    elif value.kind == "subtypes":
        return {
            "kind": "subtypes",
            "subtypes": destack._generated.query.navigation.type.to_json_subtypes_response(
                value.subtypes
            ),
        }
    elif value.kind == "decorators":
        return {
            "kind": "decorators",
            "decorators": destack._generated.query.navigation.decorator.to_json_decorators_response(
                value.decorators
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
            "rename_files": destack._generated.query.refactor.file.to_json_rename_files_response(
                value.rename_files
            ),
        }
    elif value.kind == "extractFunction":
        return {
            "kind": "extractFunction",
            "extract_function": destack._generated.query.refactor.function.to_json_extract_function_response(
                value.extract_function
            ),
        }
    elif value.kind == "extractVariable":
        return {
            "kind": "extractVariable",
            "extract_variable": destack._generated.query.refactor.variable.to_json_extract_variable_response(
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
            "change_signature": destack._generated.query.refactor.signature.to_json_change_signature_response(
                value.change_signature
            ),
        }
    elif value.kind == "codeActions":
        return {
            "kind": "codeActions",
            "code_actions": destack._generated.query.refactor.action.to_json_code_actions_response(
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
            completion=destack._generated.query.completion.item.from_json_completion_response(
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
    elif kind == "outline":
        return QueryResponseOutline(
            outline=destack._generated.query.navigation.symbol.from_json_outline_response(
                json_field(object_, "outline")
            )
        )
    elif kind == "symbolSearch":
        return QueryResponseSymbolSearch(
            symbol_search=destack._generated.query.navigation.workspace.from_json_symbol_search_response(
                json_field(object_, "symbol_search")
            )
        )
    elif kind == "links":
        return QueryResponseLinks(
            links=destack._generated.query.navigation.link.from_json_links_response(
                json_field(object_, "links")
            )
        )
    elif kind == "highlight":
        return QueryResponseHighlight(
            highlight=destack._generated.query.navigation.highlight.from_json_highlight_response(
                json_field(object_, "highlight")
            )
        )
    elif kind == "selectionRanges":
        return QueryResponseSelectionRanges(
            selection_ranges=destack._generated.query.navigation.selection.from_json_selection_ranges_response(
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
            find_references=destack._generated.query.navigation.reference.from_json_find_references_response(
                json_field(object_, "find_references")
            )
        )
    elif kind == "callItem":
        return QueryResponseCallItem(
            call_item=destack._generated.query.navigation.call.from_json_call_item_response(
                json_field(object_, "call_item")
            )
        )
    elif kind == "incomingCalls":
        return QueryResponseIncomingCalls(
            incoming_calls=destack._generated.query.navigation.call.from_json_incoming_calls_response(
                json_field(object_, "incoming_calls")
            )
        )
    elif kind == "outgoingCalls":
        return QueryResponseOutgoingCalls(
            outgoing_calls=destack._generated.query.navigation.call.from_json_outgoing_calls_response(
                json_field(object_, "outgoing_calls")
            )
        )
    elif kind == "typeItem":
        return QueryResponseTypeItem(
            type_item=destack._generated.query.navigation.type.from_json_type_item_response(
                json_field(object_, "type_item")
            )
        )
    elif kind == "supertypes":
        return QueryResponseSupertypes(
            supertypes=destack._generated.query.navigation.type.from_json_supertypes_response(
                json_field(object_, "supertypes")
            )
        )
    elif kind == "subtypes":
        return QueryResponseSubtypes(
            subtypes=destack._generated.query.navigation.type.from_json_subtypes_response(
                json_field(object_, "subtypes")
            )
        )
    elif kind == "decorators":
        return QueryResponseDecorators(
            decorators=destack._generated.query.navigation.decorator.from_json_decorators_response(
                json_field(object_, "decorators")
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
            rename_files=destack._generated.query.refactor.file.from_json_rename_files_response(
                json_field(object_, "rename_files")
            )
        )
    elif kind == "extractFunction":
        return QueryResponseExtractFunction(
            extract_function=destack._generated.query.refactor.function.from_json_extract_function_response(
                json_field(object_, "extract_function")
            )
        )
    elif kind == "extractVariable":
        return QueryResponseExtractVariable(
            extract_variable=destack._generated.query.refactor.variable.from_json_extract_variable_response(
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
            change_signature=destack._generated.query.refactor.signature.from_json_change_signature_response(
                json_field(object_, "change_signature")
            )
        )
    elif kind == "codeActions":
        return QueryResponseCodeActions(
            code_actions=destack._generated.query.refactor.action.from_json_code_actions_response(
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
