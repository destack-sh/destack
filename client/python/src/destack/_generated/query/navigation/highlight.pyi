# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.query.core.target
import destack._generated.source.file.model.span

@dataclass(frozen=True, slots=True)
class DocumentHighlightRequest:
    """Request highlights at a cursor position."""

    # the queried position
    position: destack._generated.query.core.target.QueryPosition

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DocumentHighlightRequest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DocumentHighlightRequest: ...

def encode_document_highlight_request(
    writer: BinaryWriter, value: DocumentHighlightRequest
) -> None: ...
def decode_document_highlight_request(
    reader: BinaryReader,
) -> DocumentHighlightRequest: ...
def to_json_document_highlight_request(value: DocumentHighlightRequest) -> Json: ...
def from_json_document_highlight_request(value: Json) -> DocumentHighlightRequest: ...

@dataclass(frozen=True, slots=True)
class DocumentHighlightResponse:
    """Response payload for document highlight queries."""

    # document highlights
    highlights: Sequence[DocumentHighlight]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DocumentHighlightResponse: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DocumentHighlightResponse: ...

def encode_document_highlight_response(
    writer: BinaryWriter, value: DocumentHighlightResponse
) -> None: ...
def decode_document_highlight_response(
    reader: BinaryReader,
) -> DocumentHighlightResponse: ...
def to_json_document_highlight_response(value: DocumentHighlightResponse) -> Json: ...
def from_json_document_highlight_response(value: Json) -> DocumentHighlightResponse: ...

@dataclass(frozen=True, slots=True)
class DocumentHighlight:
    """A highlighted range in a document."""

    # the highlighted range
    range: destack._generated.source.file.model.span.Span
    # the kind of highlight
    kind: HighlightKind

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DocumentHighlight: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DocumentHighlight: ...

def encode_document_highlight(
    writer: BinaryWriter, value: DocumentHighlight
) -> None: ...
def decode_document_highlight(reader: BinaryReader) -> DocumentHighlight: ...
def to_json_document_highlight(value: DocumentHighlight) -> Json: ...
def from_json_document_highlight(value: Json) -> DocumentHighlight: ...

"""Kind of document highlight."""
HighlightKind: typing.TypeAlias = (
    typing.Literal["text"] | typing.Literal["read"] | typing.Literal["write"]
)

def encode_highlight_kind(writer: BinaryWriter, value: HighlightKind) -> None: ...
def decode_highlight_kind(reader: BinaryReader) -> HighlightKind: ...
def to_json_highlight_kind(value: HighlightKind) -> Json: ...
def from_json_highlight_kind(value: Json) -> HighlightKind: ...

__all__ = [
    "DocumentHighlightRequest",
    "encode_document_highlight_request",
    "decode_document_highlight_request",
    "to_json_document_highlight_request",
    "from_json_document_highlight_request",
    "DocumentHighlightResponse",
    "encode_document_highlight_response",
    "decode_document_highlight_response",
    "to_json_document_highlight_response",
    "from_json_document_highlight_response",
    "DocumentHighlight",
    "encode_document_highlight",
    "decode_document_highlight",
    "to_json_document_highlight",
    "from_json_document_highlight",
    "HighlightKind",
    "encode_highlight_kind",
    "decode_highlight_kind",
    "to_json_highlight_kind",
    "from_json_highlight_kind",
]
