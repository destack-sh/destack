# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.query.protocol.target
import destack._generated.source.file.model.span

@dataclass(frozen=True, slots=True)
class HighlightRequest:
    """Request highlights at a cursor position."""

    # the queried position
    position: destack._generated.query.protocol.target.Position

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> HighlightRequest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> HighlightRequest: ...

def encode_highlight_request(writer: BinaryWriter, value: HighlightRequest) -> None: ...
def decode_highlight_request(reader: BinaryReader) -> HighlightRequest: ...
def to_json_highlight_request(value: HighlightRequest) -> Json: ...
def from_json_highlight_request(value: Json) -> HighlightRequest: ...

@dataclass(frozen=True, slots=True)
class HighlightResponse:
    """Response payload for highlight queries."""

    # highlights
    highlights: Sequence[Highlight]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> HighlightResponse: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> HighlightResponse: ...

def encode_highlight_response(
    writer: BinaryWriter, value: HighlightResponse
) -> None: ...
def decode_highlight_response(reader: BinaryReader) -> HighlightResponse: ...
def to_json_highlight_response(value: HighlightResponse) -> Json: ...
def from_json_highlight_response(value: Json) -> HighlightResponse: ...

@dataclass(frozen=True, slots=True)
class Highlight:
    """A highlighted range in a module."""

    # the highlighted range
    range: destack._generated.source.file.model.span.Span
    # the kind of highlight
    kind: HighlightKind

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Highlight: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Highlight: ...

def encode_highlight(writer: BinaryWriter, value: Highlight) -> None: ...
def decode_highlight(reader: BinaryReader) -> Highlight: ...
def to_json_highlight(value: Highlight) -> Json: ...
def from_json_highlight(value: Json) -> Highlight: ...

"""Kind of highlight."""
HighlightKind: typing.TypeAlias = (
    typing.Literal["text"] | typing.Literal["read"] | typing.Literal["write"]
)

def encode_highlight_kind(writer: BinaryWriter, value: HighlightKind) -> None: ...
def decode_highlight_kind(reader: BinaryReader) -> HighlightKind: ...
def to_json_highlight_kind(value: HighlightKind) -> Json: ...
def from_json_highlight_kind(value: Json) -> HighlightKind: ...

__all__ = [
    "HighlightRequest",
    "encode_highlight_request",
    "decode_highlight_request",
    "to_json_highlight_request",
    "from_json_highlight_request",
    "HighlightResponse",
    "encode_highlight_response",
    "decode_highlight_response",
    "to_json_highlight_response",
    "from_json_highlight_response",
    "Highlight",
    "encode_highlight",
    "decode_highlight",
    "to_json_highlight",
    "from_json_highlight",
    "HighlightKind",
    "encode_highlight_kind",
    "decode_highlight_kind",
    "to_json_highlight_kind",
    "from_json_highlight_kind",
]
