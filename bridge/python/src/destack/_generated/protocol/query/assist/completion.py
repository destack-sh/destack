# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.query.core.target
import destack._generated.protocol.source.edit.edit

if TYPE_CHECKING:
    from destack._generated.protocol.query.core.target import (
        QueryPosition,
    )

    from destack._generated.protocol.source.edit.edit import (
        Patch,
    )


@dataclass(frozen=True, slots=True)
class CompletionRequest:
    """Request completion items at a cursor position."""

    """The queried position."""
    position: QueryPosition
    """The trigger that initiated completion."""
    trigger: CompletionTrigger
    """Whether to include auto import completions."""
    include_imports: bool


def encode_completion_request(writer: Writer, value: CompletionRequest) -> None:
    destack._generated.protocol.query.core.target.encode_query_position(
        writer, value.position
    )
    encode_completion_trigger(writer, value.trigger)
    writer.write_bool(value.include_imports)


def decode_completion_request(reader: Reader) -> CompletionRequest:
    field_0 = destack._generated.protocol.query.core.target.decode_query_position(
        reader
    )
    field_1 = decode_completion_trigger(reader)
    field_2 = reader.read_bool()

    return CompletionRequest(
        position=field_0,
        trigger=field_1,
        include_imports=field_2,
    )


@dataclass(frozen=True, slots=True)
class CompletionTriggerInvoked:
    """Invoked manually or automatically."""

    kind: Literal["invoked"] = "invoked"


@dataclass(frozen=True, slots=True)
class CompletionTriggerCharacter:
    """Triggered by one character, for example `.`."""

    character: str
    kind: Literal["character"] = "character"


@dataclass(frozen=True, slots=True)
class CompletionTriggerIncomplete:
    """Retriggered for incomplete results."""

    kind: Literal["incomplete"] = "incomplete"


"""Trigger character that caused the completion."""
CompletionTrigger: TypeAlias = (
    CompletionTriggerInvoked | CompletionTriggerCharacter | CompletionTriggerIncomplete
)


def encode_completion_trigger(writer: Writer, value: CompletionTrigger) -> None:
    if value.kind == "invoked":
        writer.write_unsigned(0)
    elif value.kind == "character":
        writer.write_unsigned(1)
        writer.write_char(value.character)
    elif value.kind == "incomplete":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_completion_trigger(reader: Reader) -> CompletionTrigger:
    variant = reader.read_number()

    if variant == 0:
        return CompletionTriggerInvoked()
    elif variant == 1:
        return CompletionTriggerCharacter(character=reader.read_char())
    elif variant == 2:
        return CompletionTriggerIncomplete()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


@dataclass(frozen=True, slots=True)
class CompletionResponse:
    """Response payload for completion queries."""

    """Completion items."""
    items: Sequence[Completion]
    """Whether the results are incomplete."""
    is_incomplete: bool


def encode_completion_response(writer: Writer, value: CompletionResponse) -> None:
    writer.write_unsigned(len(value.items))
    for item_0 in value.items:
        encode_completion(writer, item_0)
    writer.write_bool(value.is_incomplete)


def decode_completion_response(reader: Reader) -> CompletionResponse:
    field_0 = [decode_completion(reader) for _ in range(reader.read_number())]
    field_1 = reader.read_bool()

    return CompletionResponse(
        items=field_0,
        is_incomplete=field_1,
    )


@dataclass(frozen=True, slots=True)
class Completion:
    """A completion item."""

    """The label shown in the completion list."""
    label: str
    """The kind of completion."""
    kind: CompletionKind
    """Detail shown alongside the label."""
    detail: str | None
    """Documentation for the item."""
    documentation: str | None
    """Text to insert when selected if different from the label."""
    insert_text: str | None
    """Whether the insert text is one snippet."""
    is_snippet: bool
    """Sort priority: lower = higher priority."""
    sort_order: int
    """Sort text for LSP if different from the label."""
    sort_text: str | None
    """Whether to preselect this item."""
    preselect: bool
    """Whether the item is deprecated."""
    deprecated: bool
    """Additional text edits to apply, for example auto imports."""
    additional_text_edits: Sequence[Patch]
    """Whether this completion inserts one auto import."""
    is_auto_import: bool
    """Matched character positions in the label."""
    match_positions: Sequence[int]


def encode_completion(writer: Writer, value: Completion) -> None:
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
    for item_0 in value.additional_text_edits:
        destack._generated.protocol.source.edit.edit.encode_patch(writer, item_0)
    writer.write_bool(value.is_auto_import)
    writer.write_unsigned(len(value.match_positions))
    for item_0 in value.match_positions:
        writer.write_unsigned(item_0)


def decode_completion(reader: Reader) -> Completion:
    field_0 = reader.read_string()
    field_1 = decode_completion_kind(reader)
    field_2 = reader.read_option(lambda: reader.read_string())
    field_3 = reader.read_option(lambda: reader.read_string())
    field_4 = reader.read_option(lambda: reader.read_string())
    field_5 = reader.read_bool()
    field_6 = reader.read_number()
    field_7 = reader.read_option(lambda: reader.read_string())
    field_8 = reader.read_bool()
    field_9 = reader.read_bool()
    field_10 = [
        destack._generated.protocol.source.edit.edit.decode_patch(reader)
        for _ in range(reader.read_number())
    ]
    field_11 = reader.read_bool()
    field_12 = [reader.read_number() for _ in range(reader.read_number())]

    return Completion(
        label=field_0,
        kind=field_1,
        detail=field_2,
        documentation=field_3,
        insert_text=field_4,
        is_snippet=field_5,
        sort_order=field_6,
        sort_text=field_7,
        preselect=field_8,
        deprecated=field_9,
        additional_text_edits=field_10,
        is_auto_import=field_11,
        match_positions=field_12,
    )


"""Kind of completion item."""
CompletionKind: TypeAlias = (
    Literal["text"]
    | Literal["method"]
    | Literal["function"]
    | Literal["constructor"]
    | Literal["field"]
    | Literal["variable"]
    | Literal["class"]
    | Literal["interface"]
    | Literal["module"]
    | Literal["property"]
    | Literal["unit"]
    | Literal["value"]
    | Literal["enum"]
    | Literal["keyword"]
    | Literal["snippet"]
    | Literal["color"]
    | Literal["file"]
    | Literal["reference"]
    | Literal["folder"]
    | Literal["enumMember"]
    | Literal["constant"]
    | Literal["struct"]
    | Literal["event"]
    | Literal["operator"]
    | Literal["typeParameter"]
)


def encode_completion_kind(writer: Writer, value: CompletionKind) -> None:
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


def decode_completion_kind(reader: Reader) -> CompletionKind:
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


__all__ = [
    "CompletionRequest",
    "encode_completion_request",
    "decode_completion_request",
    "CompletionTrigger",
    "encode_completion_trigger",
    "decode_completion_trigger",
    "CompletionTriggerInvoked",
    "CompletionTriggerCharacter",
    "CompletionTriggerIncomplete",
    "CompletionResponse",
    "encode_completion_response",
    "decode_completion_response",
    "Completion",
    "encode_completion",
    "decode_completion",
    "CompletionKind",
    "encode_completion_kind",
    "decode_completion_kind",
]
