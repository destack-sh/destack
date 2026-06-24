# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.query.core.target
import destack._generated.query.navigation.definition

@dataclass(frozen=True, slots=True)
class GotoImplementationRequest:
    """Request goto implementation at a cursor position."""

    # the queried position
    position: destack._generated.query.core.target.QueryPosition

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> GotoImplementationRequest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> GotoImplementationRequest: ...

def encode_goto_implementation_request(
    writer: BinaryWriter, value: GotoImplementationRequest
) -> None: ...
def decode_goto_implementation_request(
    reader: BinaryReader,
) -> GotoImplementationRequest: ...
def to_json_goto_implementation_request(value: GotoImplementationRequest) -> Json: ...
def from_json_goto_implementation_request(value: Json) -> GotoImplementationRequest: ...

@dataclass(frozen=True, slots=True)
class GotoImplementationResponse:
    """Response payload for goto implementation queries."""

    # implementation targets
    targets: Sequence[destack._generated.query.navigation.definition.NavigationTarget]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> GotoImplementationResponse: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> GotoImplementationResponse: ...

def encode_goto_implementation_response(
    writer: BinaryWriter, value: GotoImplementationResponse
) -> None: ...
def decode_goto_implementation_response(
    reader: BinaryReader,
) -> GotoImplementationResponse: ...
def to_json_goto_implementation_response(value: GotoImplementationResponse) -> Json: ...
def from_json_goto_implementation_response(
    value: Json,
) -> GotoImplementationResponse: ...

__all__ = [
    "GotoImplementationRequest",
    "encode_goto_implementation_request",
    "decode_goto_implementation_request",
    "to_json_goto_implementation_request",
    "from_json_goto_implementation_request",
    "GotoImplementationResponse",
    "encode_goto_implementation_response",
    "decode_goto_implementation_response",
    "to_json_goto_implementation_response",
    "from_json_goto_implementation_response",
]
