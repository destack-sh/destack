# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.query.protocol.target
import destack._generated.source.file.model.span

@dataclass(frozen=True, slots=True)
class SelectionRangesRequest:
    """Request selection ranges for positions in a document."""

    # the queried module
    module: destack._generated.query.protocol.target.Module
    # the byte offsets in the document
    offsets: Sequence[int]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SelectionRangesRequest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SelectionRangesRequest: ...

def encode_selection_ranges_request(
    writer: BinaryWriter, value: SelectionRangesRequest
) -> None: ...
def decode_selection_ranges_request(reader: BinaryReader) -> SelectionRangesRequest: ...
def to_json_selection_ranges_request(value: SelectionRangesRequest) -> Json: ...
def from_json_selection_ranges_request(value: Json) -> SelectionRangesRequest: ...

@dataclass(frozen=True, slots=True)
class SelectionRangesResponse:
    """Response payload for selection ranges queries."""

    # selection ranges
    ranges: Sequence[SelectionRange]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SelectionRangesResponse: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SelectionRangesResponse: ...

def encode_selection_ranges_response(
    writer: BinaryWriter, value: SelectionRangesResponse
) -> None: ...
def decode_selection_ranges_response(
    reader: BinaryReader,
) -> SelectionRangesResponse: ...
def to_json_selection_ranges_response(value: SelectionRangesResponse) -> Json: ...
def from_json_selection_ranges_response(value: Json) -> SelectionRangesResponse: ...

@dataclass(frozen=True, slots=True)
class SelectionRange:
    """A selection range with parent."""

    # the range of this selection
    range: destack._generated.source.file.model.span.Span
    # the parent selection range (for expand selection)
    parent: SelectionRange | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SelectionRange: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SelectionRange: ...

def encode_selection_range(writer: BinaryWriter, value: SelectionRange) -> None: ...
def decode_selection_range(reader: BinaryReader) -> SelectionRange: ...
def to_json_selection_range(value: SelectionRange) -> Json: ...
def from_json_selection_range(value: Json) -> SelectionRange: ...

__all__ = [
    "SelectionRangesRequest",
    "encode_selection_ranges_request",
    "decode_selection_ranges_request",
    "to_json_selection_ranges_request",
    "from_json_selection_ranges_request",
    "SelectionRangesResponse",
    "encode_selection_ranges_response",
    "decode_selection_ranges_response",
    "to_json_selection_ranges_response",
    "from_json_selection_ranges_response",
    "SelectionRange",
    "encode_selection_range",
    "decode_selection_range",
    "to_json_selection_range",
    "from_json_selection_range",
]
