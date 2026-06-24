# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.query.core.target

@dataclass(frozen=True, slots=True)
class InlayHintsRequest:
    """Request inlay hints for a range in a document."""

    # the queried range
    range: destack._generated.query.core.target.QueryRange

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> InlayHintsRequest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> InlayHintsRequest: ...

def encode_inlay_hints_request(
    writer: BinaryWriter, value: InlayHintsRequest
) -> None: ...
def decode_inlay_hints_request(reader: BinaryReader) -> InlayHintsRequest: ...
def to_json_inlay_hints_request(value: InlayHintsRequest) -> Json: ...
def from_json_inlay_hints_request(value: Json) -> InlayHintsRequest: ...

@dataclass(frozen=True, slots=True)
class InlayHintsResponse:
    """Response payload for inlay hints queries."""

    # inlay hints
    hints: Sequence[InlayHint]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> InlayHintsResponse: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> InlayHintsResponse: ...

def encode_inlay_hints_response(
    writer: BinaryWriter, value: InlayHintsResponse
) -> None: ...
def decode_inlay_hints_response(reader: BinaryReader) -> InlayHintsResponse: ...
def to_json_inlay_hints_response(value: InlayHintsResponse) -> Json: ...
def from_json_inlay_hints_response(value: Json) -> InlayHintsResponse: ...

@dataclass(frozen=True, slots=True)
class InlayHint:
    """An inlay hint (virtual text shown inline)."""

    # position where the hint should be displayed
    position: int
    # the hint text
    label: str
    # the kind of hint
    kind: InlayHintKind
    # whether there should be padding before the hint
    padding_left: bool
    # whether there should be padding after the hint
    padding_right: bool

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> InlayHint: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> InlayHint: ...

def encode_inlay_hint(writer: BinaryWriter, value: InlayHint) -> None: ...
def decode_inlay_hint(reader: BinaryReader) -> InlayHint: ...
def to_json_inlay_hint(value: InlayHint) -> Json: ...
def from_json_inlay_hint(value: Json) -> InlayHint: ...

"""Kind of inlay hint."""
InlayHintKind: typing.TypeAlias = typing.Literal["type"] | typing.Literal["parameter"]

def encode_inlay_hint_kind(writer: BinaryWriter, value: InlayHintKind) -> None: ...
def decode_inlay_hint_kind(reader: BinaryReader) -> InlayHintKind: ...
def to_json_inlay_hint_kind(value: InlayHintKind) -> Json: ...
def from_json_inlay_hint_kind(value: Json) -> InlayHintKind: ...

__all__ = [
    "InlayHintsRequest",
    "encode_inlay_hints_request",
    "decode_inlay_hints_request",
    "to_json_inlay_hints_request",
    "from_json_inlay_hints_request",
    "InlayHintsResponse",
    "encode_inlay_hints_response",
    "decode_inlay_hints_response",
    "to_json_inlay_hints_response",
    "from_json_inlay_hints_response",
    "InlayHint",
    "encode_inlay_hint",
    "decode_inlay_hint",
    "to_json_inlay_hint",
    "from_json_inlay_hint",
    "InlayHintKind",
    "encode_inlay_hint_kind",
    "decode_inlay_hint_kind",
    "to_json_inlay_hint_kind",
    "from_json_inlay_hint_kind",
]
