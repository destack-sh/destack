# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.query.core.target
import destack._generated.source.edit.edit

@dataclass(frozen=True, slots=True)
class ChangeSignatureRequest:
    """Request payload for change signature queries."""

    # the queried position
    position: destack._generated.query.core.target.QueryPosition
    # the new parameter list, comma-separated
    new_parameters: str
    # the new argument list, comma-separated
    new_arguments: str

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ChangeSignatureRequest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ChangeSignatureRequest: ...

def encode_change_signature_request(
    writer: BinaryWriter, value: ChangeSignatureRequest
) -> None: ...
def decode_change_signature_request(reader: BinaryReader) -> ChangeSignatureRequest: ...
def to_json_change_signature_request(value: ChangeSignatureRequest) -> Json: ...
def from_json_change_signature_request(value: Json) -> ChangeSignatureRequest: ...

@dataclass(frozen=True, slots=True)
class ChangeSignatureResponse:
    """Response payload for change signature queries."""

    # change signature edit, if available
    edit: destack._generated.source.edit.edit.PatchSet | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ChangeSignatureResponse: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ChangeSignatureResponse: ...

def encode_change_signature_response(
    writer: BinaryWriter, value: ChangeSignatureResponse
) -> None: ...
def decode_change_signature_response(
    reader: BinaryReader,
) -> ChangeSignatureResponse: ...
def to_json_change_signature_response(value: ChangeSignatureResponse) -> Json: ...
def from_json_change_signature_response(value: Json) -> ChangeSignatureResponse: ...

__all__ = [
    "ChangeSignatureRequest",
    "encode_change_signature_request",
    "decode_change_signature_request",
    "to_json_change_signature_request",
    "from_json_change_signature_request",
    "ChangeSignatureResponse",
    "encode_change_signature_response",
    "decode_change_signature_response",
    "to_json_change_signature_response",
    "from_json_change_signature_response",
]
