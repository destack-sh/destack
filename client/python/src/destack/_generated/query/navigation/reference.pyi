# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.query.protocol.target

@dataclass(frozen=True, slots=True)
class FindReferencesRequest:
    """Request find references at a cursor position."""

    # the queried position
    position: destack._generated.query.protocol.target.Position
    # whether to include the declaration in results
    include_declaration: bool

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FindReferencesRequest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FindReferencesRequest: ...

def encode_find_references_request(
    writer: BinaryWriter, value: FindReferencesRequest
) -> None: ...
def decode_find_references_request(reader: BinaryReader) -> FindReferencesRequest: ...
def to_json_find_references_request(value: FindReferencesRequest) -> Json: ...
def from_json_find_references_request(value: Json) -> FindReferencesRequest: ...

@dataclass(frozen=True, slots=True)
class FindReferencesResponse:
    """Response payload for find references queries."""

    # reference occurrences
    references: Sequence[Reference]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FindReferencesResponse: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FindReferencesResponse: ...

def encode_find_references_response(
    writer: BinaryWriter, value: FindReferencesResponse
) -> None: ...
def decode_find_references_response(reader: BinaryReader) -> FindReferencesResponse: ...
def to_json_find_references_response(value: FindReferencesResponse) -> Json: ...
def from_json_find_references_response(value: Json) -> FindReferencesResponse: ...

@dataclass(frozen=True, slots=True)
class Reference:
    """One symbol reference occurrence."""

    # the referenced source target
    target: destack._generated.query.protocol.target.Target
    # the reference role
    role: ReferenceRole

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Reference: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Reference: ...

def encode_reference(writer: BinaryWriter, value: Reference) -> None: ...
def decode_reference(reader: BinaryReader) -> Reference: ...
def to_json_reference(value: Reference) -> Json: ...
def from_json_reference(value: Json) -> Reference: ...

"""Role of one reference occurrence."""
ReferenceRole: typing.TypeAlias = (
    typing.Literal["declaration"]
    | typing.Literal["read"]
    | typing.Literal["write"]
    | typing.Literal["type"]
    | typing.Literal["import"]
    | typing.Literal["export"]
)

def encode_reference_role(writer: BinaryWriter, value: ReferenceRole) -> None: ...
def decode_reference_role(reader: BinaryReader) -> ReferenceRole: ...
def to_json_reference_role(value: ReferenceRole) -> Json: ...
def from_json_reference_role(value: Json) -> ReferenceRole: ...

__all__ = [
    "FindReferencesRequest",
    "encode_find_references_request",
    "decode_find_references_request",
    "to_json_find_references_request",
    "from_json_find_references_request",
    "FindReferencesResponse",
    "encode_find_references_response",
    "decode_find_references_response",
    "to_json_find_references_response",
    "from_json_find_references_response",
    "Reference",
    "encode_reference",
    "decode_reference",
    "to_json_reference",
    "from_json_reference",
    "ReferenceRole",
    "encode_reference_role",
    "decode_reference_role",
    "to_json_reference_role",
    "from_json_reference_role",
]
