# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.query.core.target
import destack._generated.source.edit.edit

@dataclass(frozen=True, slots=True)
class ExtractFunctionRequest:
    """Request payload for extract function queries."""

    # the selected source range
    range: destack._generated.query.core.target.QueryRange
    # the name for the extracted function
    new_name: str

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ExtractFunctionRequest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ExtractFunctionRequest: ...

def encode_extract_function_request(
    writer: BinaryWriter, value: ExtractFunctionRequest
) -> None: ...
def decode_extract_function_request(reader: BinaryReader) -> ExtractFunctionRequest: ...
def to_json_extract_function_request(value: ExtractFunctionRequest) -> Json: ...
def from_json_extract_function_request(value: Json) -> ExtractFunctionRequest: ...

@dataclass(frozen=True, slots=True)
class ExtractFunctionResponse:
    """Response payload for extract function queries."""

    # extract function edit, if available
    edit: destack._generated.source.edit.edit.PatchSet | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ExtractFunctionResponse: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ExtractFunctionResponse: ...

def encode_extract_function_response(
    writer: BinaryWriter, value: ExtractFunctionResponse
) -> None: ...
def decode_extract_function_response(
    reader: BinaryReader,
) -> ExtractFunctionResponse: ...
def to_json_extract_function_response(value: ExtractFunctionResponse) -> Json: ...
def from_json_extract_function_response(value: Json) -> ExtractFunctionResponse: ...

__all__ = [
    "ExtractFunctionRequest",
    "encode_extract_function_request",
    "decode_extract_function_request",
    "to_json_extract_function_request",
    "from_json_extract_function_request",
    "ExtractFunctionResponse",
    "encode_extract_function_response",
    "decode_extract_function_response",
    "to_json_extract_function_response",
    "from_json_extract_function_response",
]
