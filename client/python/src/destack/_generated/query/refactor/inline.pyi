# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.query.protocol.target
import destack._generated.source.edit.edit

@dataclass(frozen=True, slots=True)
class InlineRequest:
    """Request payload for inline refactor queries."""

    # the queried position
    position: destack._generated.query.protocol.target.Position

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> InlineRequest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> InlineRequest: ...

def encode_inline_request(writer: BinaryWriter, value: InlineRequest) -> None: ...
def decode_inline_request(reader: BinaryReader) -> InlineRequest: ...
def to_json_inline_request(value: InlineRequest) -> Json: ...
def from_json_inline_request(value: Json) -> InlineRequest: ...

@dataclass(frozen=True, slots=True)
class InlineResponse:
    """Response payload for inline refactor queries."""

    # inline edit, if available
    edit: destack._generated.source.edit.edit.PatchSet | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> InlineResponse: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> InlineResponse: ...

def encode_inline_response(writer: BinaryWriter, value: InlineResponse) -> None: ...
def decode_inline_response(reader: BinaryReader) -> InlineResponse: ...
def to_json_inline_response(value: InlineResponse) -> Json: ...
def from_json_inline_response(value: Json) -> InlineResponse: ...

__all__ = [
    "InlineRequest",
    "encode_inline_request",
    "decode_inline_request",
    "to_json_inline_request",
    "from_json_inline_request",
    "InlineResponse",
    "encode_inline_response",
    "decode_inline_response",
    "to_json_inline_response",
    "from_json_inline_response",
]
