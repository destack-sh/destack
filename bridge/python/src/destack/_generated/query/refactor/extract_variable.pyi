# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.query.core.target
import destack._generated.source.edit.edit

@dataclass(frozen=True, slots=True)
class ExtractVariableRequest:
    """Request payload for extract variable queries."""

    # the selected source range
    range: destack._generated.query.core.target.QueryRange
    # the name for the extracted variable
    new_name: str

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ExtractVariableRequest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ExtractVariableRequest: ...

def encode_extract_variable_request(
    writer: BinaryWriter, value: ExtractVariableRequest
) -> None: ...
def decode_extract_variable_request(reader: BinaryReader) -> ExtractVariableRequest: ...
def to_json_extract_variable_request(value: ExtractVariableRequest) -> Json: ...
def from_json_extract_variable_request(value: Json) -> ExtractVariableRequest: ...

@dataclass(frozen=True, slots=True)
class ExtractVariableResponse:
    """Response payload for extract variable queries."""

    # extract variable edit, if available
    edit: destack._generated.source.edit.edit.PatchSet | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ExtractVariableResponse: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ExtractVariableResponse: ...

def encode_extract_variable_response(
    writer: BinaryWriter, value: ExtractVariableResponse
) -> None: ...
def decode_extract_variable_response(
    reader: BinaryReader,
) -> ExtractVariableResponse: ...
def to_json_extract_variable_response(value: ExtractVariableResponse) -> Json: ...
def from_json_extract_variable_response(value: Json) -> ExtractVariableResponse: ...

__all__ = [
    "ExtractVariableRequest",
    "encode_extract_variable_request",
    "decode_extract_variable_request",
    "to_json_extract_variable_request",
    "from_json_extract_variable_request",
    "ExtractVariableResponse",
    "encode_extract_variable_response",
    "decode_extract_variable_response",
    "to_json_extract_variable_response",
    "from_json_extract_variable_response",
]
