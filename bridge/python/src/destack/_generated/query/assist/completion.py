# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_array,
    json_bool,
    json_field,
    json_int,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.query.core.target
import destack._generated.source.edit.edit


@dataclass(frozen=True, slots=True)
class CompletionRequest:
    """Request completion items at a cursor position."""

    # the queried position
    position: destack._generated.query.core.target.QueryPosition
    # the trigger that initiated completion
    trigger: CompletionTrigger
    # whether to include auto import completions
    include_imports: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_completion_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CompletionRequest:
        """Decode one CompletionRequest."""
        return decode_completion_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_completion_request(self)

    @classmethod
    def from_json(cls, value: Json) -> CompletionRequest:
        """Return one CompletionRequest from one JSON value."""
        return from_json_completion_request(value)


def encode_completion_request(writer: BinaryWriter, value: CompletionRequest) -> None:
    """Encode one CompletionRequest."""
    destack._generated.query.core.target.encode_query_position(writer, value.position)
    encode_completion_trigger(writer, value.trigger)
    writer.write_bool(value.include_imports)


def decode_completion_request(reader: BinaryReader) -> CompletionRequest:
    """Decode one CompletionRequest."""
    position = destack._generated.query.core.target.decode_query_position(reader)
    trigger = decode_completion_trigger(reader)
    include_imports = reader.read_bool()

    return CompletionRequest(
        position=position,
        trigger=trigger,
        include_imports=include_imports,
    )


def to_json_completion_request(value: CompletionRequest) -> Json:
    """Return one JSON value for one CompletionRequest."""
    return {
        "position": destack._generated.query.core.target.to_json_query_position(
            value.position
        ),
        "trigger": to_json_completion_trigger(value.trigger),
        "includeImports": value.include_imports,
    }


def from_json_completion_request(value: Json) -> CompletionRequest:
    """Return one CompletionRequest from one JSON value."""
    object_ = json_object(value)

    return CompletionRequest(
        position=destack._generated.query.core.target.from_json_query_position(
            json_field(object_, "position")
        ),
        trigger=from_json_completion_trigger(json_field(object_, "trigger")),
        include_imports=json_bool(json_field(object_, "includeImports")),
    )


@dataclass(frozen=True, slots=True)
class CompletionTriggerInvoked:
    """Invoked manually or automatically."""

    kind: typing.Literal["invoked"] = "invoked"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_completion_trigger(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_completion_trigger(self)


@dataclass(frozen=True, slots=True)
class CompletionTriggerCharacter:
    """Triggered by one character, for example `.`."""

    character: str
    kind: typing.Literal["character"] = "character"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_completion_trigger(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_completion_trigger(self)


@dataclass(frozen=True, slots=True)
class CompletionTriggerIncomplete:
    """Retriggered for incomplete results."""

    kind: typing.Literal["incomplete"] = "incomplete"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_completion_trigger(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_completion_trigger(self)


"""Trigger character that caused the completion."""
CompletionTrigger: typing.TypeAlias = (
    CompletionTriggerInvoked | CompletionTriggerCharacter | CompletionTriggerIncomplete
)


def encode_completion_trigger(writer: BinaryWriter, value: CompletionTrigger) -> None:
    """Encode one CompletionTrigger."""
    if value.kind == "invoked":
        writer.write_unsigned(0)
    elif value.kind == "character":
        writer.write_unsigned(1)
        writer.write_char(value.character)
    elif value.kind == "incomplete":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_completion_trigger(reader: BinaryReader) -> CompletionTrigger:
    """Decode one CompletionTrigger."""
    variant = reader.read_number()

    if variant == 0:
        return CompletionTriggerInvoked()
    elif variant == 1:
        character = reader.read_char()

        return CompletionTriggerCharacter(character=character)
    elif variant == 2:
        return CompletionTriggerIncomplete()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_completion_trigger(value: CompletionTrigger) -> Json:
    """Return one JSON value for one CompletionTrigger."""
    if value.kind == "invoked":
        return {
            "kind": "invoked",
        }
    elif value.kind == "character":
        return {
            "kind": "character",
            "character": value.character,
        }
    elif value.kind == "incomplete":
        return {
            "kind": "incomplete",
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_completion_trigger(value: Json) -> CompletionTrigger:
    """Return one CompletionTrigger from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "invoked":
        return CompletionTriggerInvoked()
    elif kind == "character":
        return CompletionTriggerCharacter(
            character=json_string(json_field(object_, "character"))
        )
    elif kind == "incomplete":
        return CompletionTriggerIncomplete()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class CompletionResponse:
    """Response payload for completion queries."""

    # completion items
    items: Sequence[Completion]
    # whether the results are incomplete
    is_incomplete: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_completion_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CompletionResponse:
        """Decode one CompletionResponse."""
        return decode_completion_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_completion_response(self)

    @classmethod
    def from_json(cls, value: Json) -> CompletionResponse:
        """Return one CompletionResponse from one JSON value."""
        return from_json_completion_response(value)


def encode_completion_response(writer: BinaryWriter, value: CompletionResponse) -> None:
    """Encode one CompletionResponse."""
    writer.write_unsigned(len(value.items))
    for item_value_items_0 in value.items:
        encode_completion(writer, item_value_items_0)
    writer.write_bool(value.is_incomplete)


def decode_completion_response(reader: BinaryReader) -> CompletionResponse:
    """Decode one CompletionResponse."""
    items = [decode_completion(reader) for _ in range(reader.read_number())]
    is_incomplete = reader.read_bool()

    return CompletionResponse(
        items=items,
        is_incomplete=is_incomplete,
    )


def to_json_completion_response(value: CompletionResponse) -> Json:
    """Return one JSON value for one CompletionResponse."""
    return {
        "items": [to_json_completion(item_0) for item_0 in value.items],
        "isIncomplete": value.is_incomplete,
    }


def from_json_completion_response(value: Json) -> CompletionResponse:
    """Return one CompletionResponse from one JSON value."""
    object_ = json_object(value)

    return CompletionResponse(
        items=[
            from_json_completion(item_0)
            for item_0 in json_array(json_field(object_, "items"))
        ],
        is_incomplete=json_bool(json_field(object_, "isIncomplete")),
    )


@dataclass(frozen=True, slots=True)
class Completion:
    """A completion item."""

    # the label shown in the completion list
    label: str
    # the kind of completion
    kind: CompletionKind
    # detail shown alongside the label
    detail: str | None
    # documentation for the item
    documentation: str | None
    # text to insert when selected if different from the label
    insert_text: str | None
    # whether the insert text is one snippet
    is_snippet: bool
    # sort priority: lower = higher priority
    sort_order: int
    # sort text for LSP if different from the label
    sort_text: str | None
    # whether to preselect this item
    preselect: bool
    # whether the item is deprecated
    deprecated: bool
    # additional text edits to apply, for example auto imports
    additional_text_edits: Sequence[destack._generated.source.edit.edit.Patch]
    # whether this completion inserts one auto import
    is_auto_import: bool
    # matched character positions in the label
    match_positions: Sequence[int]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_completion(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Completion:
        """Decode one Completion."""
        return decode_completion(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_completion(self)

    @classmethod
    def from_json(cls, value: Json) -> Completion:
        """Return one Completion from one JSON value."""
        return from_json_completion(value)


def encode_completion(writer: BinaryWriter, value: Completion) -> None:
    """Encode one Completion."""
    writer.write_string(value.label)
    encode_completion_kind(writer, value.kind)
    if value.detail is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.detail)
    if value.documentation is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.documentation)
    if value.insert_text is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.insert_text)
    writer.write_bool(value.is_snippet)
    writer.write_unsigned(value.sort_order)
    if value.sort_text is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.sort_text)
    writer.write_bool(value.preselect)
    writer.write_bool(value.deprecated)
    writer.write_unsigned(len(value.additional_text_edits))
    for item_value_additional_text_edits_0 in value.additional_text_edits:
        destack._generated.source.edit.edit.encode_patch(
            writer, item_value_additional_text_edits_0
        )
    writer.write_bool(value.is_auto_import)
    writer.write_unsigned(len(value.match_positions))
    for item_value_match_positions_0 in value.match_positions:
        writer.write_unsigned(item_value_match_positions_0)


def decode_completion(reader: BinaryReader) -> Completion:
    """Decode one Completion."""
    label = reader.read_string()
    kind = decode_completion_kind(reader)
    detail = reader.read_option(lambda: reader.read_string())
    documentation = reader.read_option(lambda: reader.read_string())
    insert_text = reader.read_option(lambda: reader.read_string())
    is_snippet = reader.read_bool()
    sort_order = reader.read_number()
    sort_text = reader.read_option(lambda: reader.read_string())
    preselect = reader.read_bool()
    deprecated = reader.read_bool()
    additional_text_edits = [
        destack._generated.source.edit.edit.decode_patch(reader)
        for _ in range(reader.read_number())
    ]
    is_auto_import = reader.read_bool()
    match_positions = [reader.read_number() for _ in range(reader.read_number())]

    return Completion(
        label=label,
        kind=kind,
        detail=detail,
        documentation=documentation,
        insert_text=insert_text,
        is_snippet=is_snippet,
        sort_order=sort_order,
        sort_text=sort_text,
        preselect=preselect,
        deprecated=deprecated,
        additional_text_edits=additional_text_edits,
        is_auto_import=is_auto_import,
        match_positions=match_positions,
    )


def to_json_completion(value: Completion) -> Json:
    """Return one JSON value for one Completion."""
    return {
        "label": value.label,
        "kind": to_json_completion_kind(value.kind),
        **({} if value.detail is None else {"detail": value.detail}),
        **(
            {}
            if value.documentation is None
            else {"documentation": value.documentation}
        ),
        **({} if value.insert_text is None else {"insertText": value.insert_text}),
        "isSnippet": value.is_snippet,
        "sortOrder": value.sort_order,
        **({} if value.sort_text is None else {"sortText": value.sort_text}),
        "preselect": value.preselect,
        "deprecated": value.deprecated,
        "additionalTextEdits": [
            destack._generated.source.edit.edit.to_json_patch(item_0)
            for item_0 in value.additional_text_edits
        ],
        "isAutoImport": value.is_auto_import,
        "matchPositions": [item_0 for item_0 in value.match_positions],
    }


def from_json_completion(value: Json) -> Completion:
    """Return one Completion from one JSON value."""
    object_ = json_object(value)

    return Completion(
        label=json_string(json_field(object_, "label")),
        kind=from_json_completion_kind(json_field(object_, "kind")),
        detail=json_optional(object_, "detail", lambda value: json_string(value)),
        documentation=json_optional(
            object_, "documentation", lambda value: json_string(value)
        ),
        insert_text=json_optional(
            object_, "insertText", lambda value: json_string(value)
        ),
        is_snippet=json_bool(json_field(object_, "isSnippet")),
        sort_order=json_int(json_field(object_, "sortOrder")),
        sort_text=json_optional(object_, "sortText", lambda value: json_string(value)),
        preselect=json_bool(json_field(object_, "preselect")),
        deprecated=json_bool(json_field(object_, "deprecated")),
        additional_text_edits=[
            destack._generated.source.edit.edit.from_json_patch(item_0)
            for item_0 in json_array(json_field(object_, "additionalTextEdits"))
        ],
        is_auto_import=json_bool(json_field(object_, "isAutoImport")),
        match_positions=[
            json_int(item_0)
            for item_0 in json_array(json_field(object_, "matchPositions"))
        ],
    )


"""Kind of completion item."""
CompletionKind: typing.TypeAlias = (
    typing.Literal["text"]
    | typing.Literal["method"]
    | typing.Literal["function"]
    | typing.Literal["constructor"]
    | typing.Literal["field"]
    | typing.Literal["variable"]
    | typing.Literal["class"]
    | typing.Literal["interface"]
    | typing.Literal["module"]
    | typing.Literal["property"]
    | typing.Literal["unit"]
    | typing.Literal["value"]
    | typing.Literal["enum"]
    | typing.Literal["keyword"]
    | typing.Literal["snippet"]
    | typing.Literal["color"]
    | typing.Literal["file"]
    | typing.Literal["reference"]
    | typing.Literal["folder"]
    | typing.Literal["enumMember"]
    | typing.Literal["constant"]
    | typing.Literal["struct"]
    | typing.Literal["event"]
    | typing.Literal["operator"]
    | typing.Literal["typeParameter"]
)


def encode_completion_kind(writer: BinaryWriter, value: CompletionKind) -> None:
    """Encode one CompletionKind."""
    if value == "text":
        writer.write_unsigned(0)
    elif value == "method":
        writer.write_unsigned(1)
    elif value == "function":
        writer.write_unsigned(2)
    elif value == "constructor":
        writer.write_unsigned(3)
    elif value == "field":
        writer.write_unsigned(4)
    elif value == "variable":
        writer.write_unsigned(5)
    elif value == "class":
        writer.write_unsigned(6)
    elif value == "interface":
        writer.write_unsigned(7)
    elif value == "module":
        writer.write_unsigned(8)
    elif value == "property":
        writer.write_unsigned(9)
    elif value == "unit":
        writer.write_unsigned(10)
    elif value == "value":
        writer.write_unsigned(11)
    elif value == "enum":
        writer.write_unsigned(12)
    elif value == "keyword":
        writer.write_unsigned(13)
    elif value == "snippet":
        writer.write_unsigned(14)
    elif value == "color":
        writer.write_unsigned(15)
    elif value == "file":
        writer.write_unsigned(16)
    elif value == "reference":
        writer.write_unsigned(17)
    elif value == "folder":
        writer.write_unsigned(18)
    elif value == "enumMember":
        writer.write_unsigned(19)
    elif value == "constant":
        writer.write_unsigned(20)
    elif value == "struct":
        writer.write_unsigned(21)
    elif value == "event":
        writer.write_unsigned(22)
    elif value == "operator":
        writer.write_unsigned(23)
    elif value == "typeParameter":
        writer.write_unsigned(24)
    else:
        raise SerdeError("unknown enum variant")


def decode_completion_kind(reader: BinaryReader) -> CompletionKind:
    """Decode one CompletionKind."""
    variant = reader.read_number()

    if variant == 0:
        return "text"
    elif variant == 1:
        return "method"
    elif variant == 2:
        return "function"
    elif variant == 3:
        return "constructor"
    elif variant == 4:
        return "field"
    elif variant == 5:
        return "variable"
    elif variant == 6:
        return "class"
    elif variant == 7:
        return "interface"
    elif variant == 8:
        return "module"
    elif variant == 9:
        return "property"
    elif variant == 10:
        return "unit"
    elif variant == 11:
        return "value"
    elif variant == 12:
        return "enum"
    elif variant == 13:
        return "keyword"
    elif variant == 14:
        return "snippet"
    elif variant == 15:
        return "color"
    elif variant == 16:
        return "file"
    elif variant == 17:
        return "reference"
    elif variant == 18:
        return "folder"
    elif variant == 19:
        return "enumMember"
    elif variant == 20:
        return "constant"
    elif variant == 21:
        return "struct"
    elif variant == 22:
        return "event"
    elif variant == 23:
        return "operator"
    elif variant == 24:
        return "typeParameter"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_completion_kind(value: CompletionKind) -> Json:
    """Return one JSON value for one CompletionKind."""
    return value


def from_json_completion_kind(value: Json) -> CompletionKind:
    """Return one CompletionKind from one JSON value."""
    variant = json_string(value)

    if variant == "text":
        return "text"
    elif variant == "method":
        return "method"
    elif variant == "function":
        return "function"
    elif variant == "constructor":
        return "constructor"
    elif variant == "field":
        return "field"
    elif variant == "variable":
        return "variable"
    elif variant == "class":
        return "class"
    elif variant == "interface":
        return "interface"
    elif variant == "module":
        return "module"
    elif variant == "property":
        return "property"
    elif variant == "unit":
        return "unit"
    elif variant == "value":
        return "value"
    elif variant == "enum":
        return "enum"
    elif variant == "keyword":
        return "keyword"
    elif variant == "snippet":
        return "snippet"
    elif variant == "color":
        return "color"
    elif variant == "file":
        return "file"
    elif variant == "reference":
        return "reference"
    elif variant == "folder":
        return "folder"
    elif variant == "enumMember":
        return "enumMember"
    elif variant == "constant":
        return "constant"
    elif variant == "struct":
        return "struct"
    elif variant == "event":
        return "event"
    elif variant == "operator":
        return "operator"
    elif variant == "typeParameter":
        return "typeParameter"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "CompletionRequest",
    "encode_completion_request",
    "decode_completion_request",
    "to_json_completion_request",
    "from_json_completion_request",
    "CompletionTrigger",
    "encode_completion_trigger",
    "decode_completion_trigger",
    "to_json_completion_trigger",
    "from_json_completion_trigger",
    "CompletionTriggerInvoked",
    "CompletionTriggerCharacter",
    "CompletionTriggerIncomplete",
    "CompletionResponse",
    "encode_completion_response",
    "decode_completion_response",
    "to_json_completion_response",
    "from_json_completion_response",
    "Completion",
    "encode_completion",
    "decode_completion",
    "to_json_completion",
    "from_json_completion",
    "CompletionKind",
    "encode_completion_kind",
    "decode_completion_kind",
    "to_json_completion_kind",
    "from_json_completion_kind",
]
