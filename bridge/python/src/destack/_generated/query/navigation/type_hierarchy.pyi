# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.query.core.target

@dataclass(frozen=True, slots=True)
class TypeHierarchyItemRequest:
    """Request the type hierarchy item at a cursor position."""

    # the queried position
    position: destack._generated.query.core.target.QueryPosition

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeHierarchyItemRequest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TypeHierarchyItemRequest: ...

def encode_type_hierarchy_item_request(
    writer: BinaryWriter, value: TypeHierarchyItemRequest
) -> None: ...
def decode_type_hierarchy_item_request(
    reader: BinaryReader,
) -> TypeHierarchyItemRequest: ...
def to_json_type_hierarchy_item_request(value: TypeHierarchyItemRequest) -> Json: ...
def from_json_type_hierarchy_item_request(value: Json) -> TypeHierarchyItemRequest: ...

@dataclass(frozen=True, slots=True)
class TypeHierarchySupertypesRequest:
    """Request type hierarchy supertypes."""

    # the type hierarchy item to expand
    item: TypeHierarchyItem

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeHierarchySupertypesRequest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TypeHierarchySupertypesRequest: ...

def encode_type_hierarchy_supertypes_request(
    writer: BinaryWriter, value: TypeHierarchySupertypesRequest
) -> None: ...
def decode_type_hierarchy_supertypes_request(
    reader: BinaryReader,
) -> TypeHierarchySupertypesRequest: ...
def to_json_type_hierarchy_supertypes_request(
    value: TypeHierarchySupertypesRequest,
) -> Json: ...
def from_json_type_hierarchy_supertypes_request(
    value: Json,
) -> TypeHierarchySupertypesRequest: ...

@dataclass(frozen=True, slots=True)
class TypeHierarchyItem:
    """An item in the type hierarchy."""

    # the name of the type
    name: str
    # the kind of type
    kind: TypeHierarchyKind
    # detail (e.g., generic parameters)
    detail: str | None
    # the target source and resolved identity
    target: destack._generated.query.core.target.QueryTarget

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeHierarchyItem: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TypeHierarchyItem: ...

def encode_type_hierarchy_item(
    writer: BinaryWriter, value: TypeHierarchyItem
) -> None: ...
def decode_type_hierarchy_item(reader: BinaryReader) -> TypeHierarchyItem: ...
def to_json_type_hierarchy_item(value: TypeHierarchyItem) -> Json: ...
def from_json_type_hierarchy_item(value: Json) -> TypeHierarchyItem: ...

"""Kind of type hierarchy item."""
TypeHierarchyKind: typing.TypeAlias = (
    typing.Literal["class"]
    | typing.Literal["interface"]
    | typing.Literal["struct"]
    | typing.Literal["enum"]
    | typing.Literal["typeAlias"]
)

def encode_type_hierarchy_kind(
    writer: BinaryWriter, value: TypeHierarchyKind
) -> None: ...
def decode_type_hierarchy_kind(reader: BinaryReader) -> TypeHierarchyKind: ...
def to_json_type_hierarchy_kind(value: TypeHierarchyKind) -> Json: ...
def from_json_type_hierarchy_kind(value: Json) -> TypeHierarchyKind: ...

@dataclass(frozen=True, slots=True)
class TypeHierarchySubtypesRequest:
    """Request type hierarchy subtypes."""

    # the type hierarchy item to expand
    item: TypeHierarchyItem

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeHierarchySubtypesRequest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TypeHierarchySubtypesRequest: ...

def encode_type_hierarchy_subtypes_request(
    writer: BinaryWriter, value: TypeHierarchySubtypesRequest
) -> None: ...
def decode_type_hierarchy_subtypes_request(
    reader: BinaryReader,
) -> TypeHierarchySubtypesRequest: ...
def to_json_type_hierarchy_subtypes_request(
    value: TypeHierarchySubtypesRequest,
) -> Json: ...
def from_json_type_hierarchy_subtypes_request(
    value: Json,
) -> TypeHierarchySubtypesRequest: ...

@dataclass(frozen=True, slots=True)
class TypeHierarchyItemResponse:
    """Response payload for type hierarchy item queries."""

    # type hierarchy item, if available
    item: TypeHierarchyItem | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeHierarchyItemResponse: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TypeHierarchyItemResponse: ...

def encode_type_hierarchy_item_response(
    writer: BinaryWriter, value: TypeHierarchyItemResponse
) -> None: ...
def decode_type_hierarchy_item_response(
    reader: BinaryReader,
) -> TypeHierarchyItemResponse: ...
def to_json_type_hierarchy_item_response(value: TypeHierarchyItemResponse) -> Json: ...
def from_json_type_hierarchy_item_response(
    value: Json,
) -> TypeHierarchyItemResponse: ...

@dataclass(frozen=True, slots=True)
class TypeHierarchySupertypesResponse:
    """Response payload for type hierarchy supertypes queries."""

    # type hierarchy items
    items: Sequence[TypeHierarchyItem]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeHierarchySupertypesResponse: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TypeHierarchySupertypesResponse: ...

def encode_type_hierarchy_supertypes_response(
    writer: BinaryWriter, value: TypeHierarchySupertypesResponse
) -> None: ...
def decode_type_hierarchy_supertypes_response(
    reader: BinaryReader,
) -> TypeHierarchySupertypesResponse: ...
def to_json_type_hierarchy_supertypes_response(
    value: TypeHierarchySupertypesResponse,
) -> Json: ...
def from_json_type_hierarchy_supertypes_response(
    value: Json,
) -> TypeHierarchySupertypesResponse: ...

@dataclass(frozen=True, slots=True)
class TypeHierarchySubtypesResponse:
    """Response payload for type hierarchy subtypes queries."""

    # type hierarchy items
    items: Sequence[TypeHierarchyItem]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeHierarchySubtypesResponse: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TypeHierarchySubtypesResponse: ...

def encode_type_hierarchy_subtypes_response(
    writer: BinaryWriter, value: TypeHierarchySubtypesResponse
) -> None: ...
def decode_type_hierarchy_subtypes_response(
    reader: BinaryReader,
) -> TypeHierarchySubtypesResponse: ...
def to_json_type_hierarchy_subtypes_response(
    value: TypeHierarchySubtypesResponse,
) -> Json: ...
def from_json_type_hierarchy_subtypes_response(
    value: Json,
) -> TypeHierarchySubtypesResponse: ...

__all__ = [
    "TypeHierarchyItemRequest",
    "encode_type_hierarchy_item_request",
    "decode_type_hierarchy_item_request",
    "to_json_type_hierarchy_item_request",
    "from_json_type_hierarchy_item_request",
    "TypeHierarchySupertypesRequest",
    "encode_type_hierarchy_supertypes_request",
    "decode_type_hierarchy_supertypes_request",
    "to_json_type_hierarchy_supertypes_request",
    "from_json_type_hierarchy_supertypes_request",
    "TypeHierarchyItem",
    "encode_type_hierarchy_item",
    "decode_type_hierarchy_item",
    "to_json_type_hierarchy_item",
    "from_json_type_hierarchy_item",
    "TypeHierarchyKind",
    "encode_type_hierarchy_kind",
    "decode_type_hierarchy_kind",
    "to_json_type_hierarchy_kind",
    "from_json_type_hierarchy_kind",
    "TypeHierarchySubtypesRequest",
    "encode_type_hierarchy_subtypes_request",
    "decode_type_hierarchy_subtypes_request",
    "to_json_type_hierarchy_subtypes_request",
    "from_json_type_hierarchy_subtypes_request",
    "TypeHierarchyItemResponse",
    "encode_type_hierarchy_item_response",
    "decode_type_hierarchy_item_response",
    "to_json_type_hierarchy_item_response",
    "from_json_type_hierarchy_item_response",
    "TypeHierarchySupertypesResponse",
    "encode_type_hierarchy_supertypes_response",
    "decode_type_hierarchy_supertypes_response",
    "to_json_type_hierarchy_supertypes_response",
    "from_json_type_hierarchy_supertypes_response",
    "TypeHierarchySubtypesResponse",
    "encode_type_hierarchy_subtypes_response",
    "decode_type_hierarchy_subtypes_response",
    "to_json_type_hierarchy_subtypes_response",
    "from_json_type_hierarchy_subtypes_response",
]
