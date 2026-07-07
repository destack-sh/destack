# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.symbol.symbol
import destack._generated.query.protocol.target

@dataclass(frozen=True, slots=True)
class TypeItemRequest:
    """Request the type item at a cursor position."""

    # the queried position
    position: destack._generated.query.protocol.target.Position

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeItemRequest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TypeItemRequest: ...

def encode_type_item_request(writer: BinaryWriter, value: TypeItemRequest) -> None: ...
def decode_type_item_request(reader: BinaryReader) -> TypeItemRequest: ...
def to_json_type_item_request(value: TypeItemRequest) -> Json: ...
def from_json_type_item_request(value: Json) -> TypeItemRequest: ...

@dataclass(frozen=True, slots=True)
class SupertypesRequest:
    """Request supertypes."""

    # the type item to expand
    item: TypeItem

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SupertypesRequest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SupertypesRequest: ...

def encode_supertypes_request(
    writer: BinaryWriter, value: SupertypesRequest
) -> None: ...
def decode_supertypes_request(reader: BinaryReader) -> SupertypesRequest: ...
def to_json_supertypes_request(value: SupertypesRequest) -> Json: ...
def from_json_supertypes_request(value: Json) -> SupertypesRequest: ...

@dataclass(frozen=True, slots=True)
class TypeItem:
    """An item in the type hierarchy."""

    # the name of the type
    name: str
    # the kind of type
    kind: destack._generated.dir.symbol.symbol.SymbolKind
    # detail (e.g., generic parameters)
    detail: str | None
    # the target source and resolved identity
    target: destack._generated.query.protocol.target.Target

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeItem: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TypeItem: ...

def encode_type_item(writer: BinaryWriter, value: TypeItem) -> None: ...
def decode_type_item(reader: BinaryReader) -> TypeItem: ...
def to_json_type_item(value: TypeItem) -> Json: ...
def from_json_type_item(value: Json) -> TypeItem: ...

@dataclass(frozen=True, slots=True)
class SubtypesRequest:
    """Request subtypes."""

    # the type item to expand
    item: TypeItem

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SubtypesRequest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SubtypesRequest: ...

def encode_subtypes_request(writer: BinaryWriter, value: SubtypesRequest) -> None: ...
def decode_subtypes_request(reader: BinaryReader) -> SubtypesRequest: ...
def to_json_subtypes_request(value: SubtypesRequest) -> Json: ...
def from_json_subtypes_request(value: Json) -> SubtypesRequest: ...

@dataclass(frozen=True, slots=True)
class TypeItemResponse:
    """Response payload for type item queries."""

    # type hierarchy item, if available
    item: TypeItem | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeItemResponse: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TypeItemResponse: ...

def encode_type_item_response(
    writer: BinaryWriter, value: TypeItemResponse
) -> None: ...
def decode_type_item_response(reader: BinaryReader) -> TypeItemResponse: ...
def to_json_type_item_response(value: TypeItemResponse) -> Json: ...
def from_json_type_item_response(value: Json) -> TypeItemResponse: ...

@dataclass(frozen=True, slots=True)
class SupertypesResponse:
    """Response payload for supertypes queries."""

    # type hierarchy items
    items: Sequence[TypeItem]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SupertypesResponse: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SupertypesResponse: ...

def encode_supertypes_response(
    writer: BinaryWriter, value: SupertypesResponse
) -> None: ...
def decode_supertypes_response(reader: BinaryReader) -> SupertypesResponse: ...
def to_json_supertypes_response(value: SupertypesResponse) -> Json: ...
def from_json_supertypes_response(value: Json) -> SupertypesResponse: ...

@dataclass(frozen=True, slots=True)
class SubtypesResponse:
    """Response payload for subtypes queries."""

    # type hierarchy items
    items: Sequence[TypeItem]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SubtypesResponse: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SubtypesResponse: ...

def encode_subtypes_response(writer: BinaryWriter, value: SubtypesResponse) -> None: ...
def decode_subtypes_response(reader: BinaryReader) -> SubtypesResponse: ...
def to_json_subtypes_response(value: SubtypesResponse) -> Json: ...
def from_json_subtypes_response(value: Json) -> SubtypesResponse: ...

__all__ = [
    "TypeItemRequest",
    "encode_type_item_request",
    "decode_type_item_request",
    "to_json_type_item_request",
    "from_json_type_item_request",
    "SupertypesRequest",
    "encode_supertypes_request",
    "decode_supertypes_request",
    "to_json_supertypes_request",
    "from_json_supertypes_request",
    "TypeItem",
    "encode_type_item",
    "decode_type_item",
    "to_json_type_item",
    "from_json_type_item",
    "SubtypesRequest",
    "encode_subtypes_request",
    "decode_subtypes_request",
    "to_json_subtypes_request",
    "from_json_subtypes_request",
    "TypeItemResponse",
    "encode_type_item_response",
    "decode_type_item_response",
    "to_json_type_item_response",
    "from_json_type_item_response",
    "SupertypesResponse",
    "encode_supertypes_response",
    "decode_supertypes_response",
    "to_json_supertypes_response",
    "from_json_supertypes_response",
    "SubtypesResponse",
    "encode_subtypes_response",
    "decode_subtypes_response",
    "to_json_subtypes_response",
    "from_json_subtypes_response",
]
