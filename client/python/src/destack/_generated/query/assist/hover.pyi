# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.query.protocol.target
import destack._generated.source.file.model.span

@dataclass(frozen=True, slots=True)
class HoverRequest:
    """Request hover information at a cursor position."""

    # the queried position
    position: destack._generated.query.protocol.target.Position

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> HoverRequest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> HoverRequest: ...

def encode_hover_request(writer: BinaryWriter, value: HoverRequest) -> None: ...
def decode_hover_request(reader: BinaryReader) -> HoverRequest: ...
def to_json_hover_request(value: HoverRequest) -> Json: ...
def from_json_hover_request(value: Json) -> HoverRequest: ...

@dataclass(frozen=True, slots=True)
class HoverResponse:
    """Response payload for hover queries."""

    # hover information, if available
    hover: Hover | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> HoverResponse: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> HoverResponse: ...

def encode_hover_response(writer: BinaryWriter, value: HoverResponse) -> None: ...
def decode_hover_response(reader: BinaryReader) -> HoverResponse: ...
def to_json_hover_response(value: HoverResponse) -> Json: ...
def from_json_hover_response(value: Json) -> HoverResponse: ...

@dataclass(frozen=True, slots=True)
class Hover:
    """Hover payload for a source position."""

    # the type/signature in code format
    signature: str
    # documentation (markdown)
    documentation: str | None
    # resolved type information when available
    type_text: str | None
    # source location text when available
    location: str | None
    # the range of the hovered element
    range: destack._generated.source.file.model.span.Span | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Hover: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Hover: ...

def encode_hover(writer: BinaryWriter, value: Hover) -> None: ...
def decode_hover(reader: BinaryReader) -> Hover: ...
def to_json_hover(value: Hover) -> Json: ...
def from_json_hover(value: Json) -> Hover: ...

__all__ = [
    "HoverRequest",
    "encode_hover_request",
    "decode_hover_request",
    "to_json_hover_request",
    "from_json_hover_request",
    "HoverResponse",
    "encode_hover_response",
    "decode_hover_response",
    "to_json_hover_response",
    "from_json_hover_response",
    "Hover",
    "encode_hover",
    "decode_hover",
    "to_json_hover",
    "from_json_hover",
]
