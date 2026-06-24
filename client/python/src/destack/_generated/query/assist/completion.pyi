# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CompletionRequest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CompletionRequest: ...

def encode_completion_request(
    writer: BinaryWriter, value: CompletionRequest
) -> None: ...
def decode_completion_request(reader: BinaryReader) -> CompletionRequest: ...
def to_json_completion_request(value: CompletionRequest) -> Json: ...
def from_json_completion_request(value: Json) -> CompletionRequest: ...

@dataclass(frozen=True, slots=True)
class CompletionTriggerInvoked:
    """Invoked manually or automatically."""

    kind: typing.Literal["invoked"] = "invoked"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CompletionTriggerCharacter:
    """Triggered by one character, for example `.`."""

    character: str
    kind: typing.Literal["character"] = "character"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CompletionTriggerIncomplete:
    """Retriggered for incomplete results."""

    kind: typing.Literal["incomplete"] = "incomplete"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Trigger character that caused the completion."""
CompletionTrigger: typing.TypeAlias = (
    CompletionTriggerInvoked | CompletionTriggerCharacter | CompletionTriggerIncomplete
)

def encode_completion_trigger(
    writer: BinaryWriter, value: CompletionTrigger
) -> None: ...
def decode_completion_trigger(reader: BinaryReader) -> CompletionTrigger: ...
def to_json_completion_trigger(value: CompletionTrigger) -> Json: ...
def from_json_completion_trigger(value: Json) -> CompletionTrigger: ...

@dataclass(frozen=True, slots=True)
class CompletionResponse:
    """Response payload for completion queries."""

    # completion items
    items: Sequence[Completion]
    # whether the results are incomplete
    is_incomplete: bool

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CompletionResponse: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CompletionResponse: ...

def encode_completion_response(
    writer: BinaryWriter, value: CompletionResponse
) -> None: ...
def decode_completion_response(reader: BinaryReader) -> CompletionResponse: ...
def to_json_completion_response(value: CompletionResponse) -> Json: ...
def from_json_completion_response(value: Json) -> CompletionResponse: ...

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Completion: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Completion: ...

def encode_completion(writer: BinaryWriter, value: Completion) -> None: ...
def decode_completion(reader: BinaryReader) -> Completion: ...
def to_json_completion(value: Completion) -> Json: ...
def from_json_completion(value: Json) -> Completion: ...

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

def encode_completion_kind(writer: BinaryWriter, value: CompletionKind) -> None: ...
def decode_completion_kind(reader: BinaryReader) -> CompletionKind: ...
def to_json_completion_kind(value: CompletionKind) -> Json: ...
def from_json_completion_kind(value: Json) -> CompletionKind: ...

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
