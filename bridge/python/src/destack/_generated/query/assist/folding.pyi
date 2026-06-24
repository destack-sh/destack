# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.query.core.target

@dataclass(frozen=True, slots=True)
class FoldingRangesRequest:
    """Request folding ranges for a document."""

    # the queried module
    module: destack._generated.query.core.target.QueryModule

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FoldingRangesRequest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FoldingRangesRequest: ...

def encode_folding_ranges_request(
    writer: BinaryWriter, value: FoldingRangesRequest
) -> None: ...
def decode_folding_ranges_request(reader: BinaryReader) -> FoldingRangesRequest: ...
def to_json_folding_ranges_request(value: FoldingRangesRequest) -> Json: ...
def from_json_folding_ranges_request(value: Json) -> FoldingRangesRequest: ...

@dataclass(frozen=True, slots=True)
class FoldingRangesResponse:
    """Response payload for folding ranges queries."""

    # folding ranges
    ranges: Sequence[FoldingRange]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FoldingRangesResponse: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FoldingRangesResponse: ...

def encode_folding_ranges_response(
    writer: BinaryWriter, value: FoldingRangesResponse
) -> None: ...
def decode_folding_ranges_response(reader: BinaryReader) -> FoldingRangesResponse: ...
def to_json_folding_ranges_response(value: FoldingRangesResponse) -> Json: ...
def from_json_folding_ranges_response(value: Json) -> FoldingRangesResponse: ...

@dataclass(frozen=True, slots=True)
class FoldingRange:
    """A foldable range in source code."""

    # start line (0-indexed)
    start_line: int
    # end line (0-indexed)
    end_line: int
    # optional start character
    start_character: int | None
    # optional end character
    end_character: int | None
    # the kind of folding range
    kind: FoldingRangeKind | None
    # text to show when collapsed
    collapsed_text: str | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FoldingRange: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FoldingRange: ...

def encode_folding_range(writer: BinaryWriter, value: FoldingRange) -> None: ...
def decode_folding_range(reader: BinaryReader) -> FoldingRange: ...
def to_json_folding_range(value: FoldingRange) -> Json: ...
def from_json_folding_range(value: Json) -> FoldingRange: ...

"""Kind of folding range."""
FoldingRangeKind: typing.TypeAlias = (
    typing.Literal["comment"] | typing.Literal["imports"] | typing.Literal["region"]
)

def encode_folding_range_kind(
    writer: BinaryWriter, value: FoldingRangeKind
) -> None: ...
def decode_folding_range_kind(reader: BinaryReader) -> FoldingRangeKind: ...
def to_json_folding_range_kind(value: FoldingRangeKind) -> Json: ...
def from_json_folding_range_kind(value: Json) -> FoldingRangeKind: ...

__all__ = [
    "FoldingRangesRequest",
    "encode_folding_ranges_request",
    "decode_folding_ranges_request",
    "to_json_folding_ranges_request",
    "from_json_folding_ranges_request",
    "FoldingRangesResponse",
    "encode_folding_ranges_response",
    "decode_folding_ranges_response",
    "to_json_folding_ranges_response",
    "from_json_folding_ranges_response",
    "FoldingRange",
    "encode_folding_range",
    "decode_folding_range",
    "to_json_folding_range",
    "from_json_folding_range",
    "FoldingRangeKind",
    "encode_folding_range_kind",
    "decode_folding_range_kind",
    "to_json_folding_range_kind",
    "from_json_folding_range_kind",
]
