# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.query.core.target

if TYPE_CHECKING:
    from destack._generated.protocol.query.core.target import (
        QueryPosition,
        QueryTarget,
    )


@dataclass(frozen=True, slots=True)
class TypeHierarchyItemRequest:
    """Request the type hierarchy item at a cursor position."""

    """The queried position."""
    position: QueryPosition


def encode_type_hierarchy_item_request(
    writer: Writer, value: TypeHierarchyItemRequest
) -> None:
    destack._generated.protocol.query.core.target.encode_query_position(
        writer, value.position
    )


def decode_type_hierarchy_item_request(reader: Reader) -> TypeHierarchyItemRequest:
    field_0 = destack._generated.protocol.query.core.target.decode_query_position(
        reader
    )

    return TypeHierarchyItemRequest(
        position=field_0,
    )


@dataclass(frozen=True, slots=True)
class TypeHierarchySupertypesRequest:
    """Request type hierarchy supertypes."""

    """The type hierarchy item to expand."""
    item: TypeHierarchyItem


def encode_type_hierarchy_supertypes_request(
    writer: Writer, value: TypeHierarchySupertypesRequest
) -> None:
    encode_type_hierarchy_item(writer, value.item)


def decode_type_hierarchy_supertypes_request(
    reader: Reader,
) -> TypeHierarchySupertypesRequest:
    field_0 = decode_type_hierarchy_item(reader)

    return TypeHierarchySupertypesRequest(
        item=field_0,
    )


@dataclass(frozen=True, slots=True)
class TypeHierarchyItem:
    """An item in the type hierarchy."""

    """The name of the type."""
    name: str
    """The kind of type."""
    kind: TypeHierarchyKind
    """Detail (e.g., generic parameters)."""
    detail: str | None
    """The target source and resolved identity."""
    target: QueryTarget


def encode_type_hierarchy_item(writer: Writer, value: TypeHierarchyItem) -> None:
    writer.write_string(value.name)
    encode_type_hierarchy_kind(writer, value.kind)
    if value.detail is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.detail)
    destack._generated.protocol.query.core.target.encode_query_target(
        writer, value.target
    )


def decode_type_hierarchy_item(reader: Reader) -> TypeHierarchyItem:
    field_0 = reader.read_string()
    field_1 = decode_type_hierarchy_kind(reader)
    field_2 = reader.read_option(lambda: reader.read_string())
    field_3 = destack._generated.protocol.query.core.target.decode_query_target(reader)

    return TypeHierarchyItem(
        name=field_0,
        kind=field_1,
        detail=field_2,
        target=field_3,
    )


"""Kind of type hierarchy item."""
TypeHierarchyKind: TypeAlias = (
    Literal["class"]
    | Literal["interface"]
    | Literal["struct"]
    | Literal["enum"]
    | Literal["typeAlias"]
)


def encode_type_hierarchy_kind(writer: Writer, value: TypeHierarchyKind) -> None:
    if value == "class":
        writer.write_unsigned(0)
    elif value == "interface":
        writer.write_unsigned(1)
    elif value == "struct":
        writer.write_unsigned(2)
    elif value == "enum":
        writer.write_unsigned(3)
    elif value == "typeAlias":
        writer.write_unsigned(4)
    else:
        raise SerdeError("unknown enum variant")


def decode_type_hierarchy_kind(reader: Reader) -> TypeHierarchyKind:
    variant = reader.read_number()

    if variant == 0:
        return "class"
    elif variant == 1:
        return "interface"
    elif variant == 2:
        return "struct"
    elif variant == 3:
        return "enum"
    elif variant == 4:
        return "typeAlias"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


@dataclass(frozen=True, slots=True)
class TypeHierarchySubtypesRequest:
    """Request type hierarchy subtypes."""

    """The type hierarchy item to expand."""
    item: TypeHierarchyItem


def encode_type_hierarchy_subtypes_request(
    writer: Writer, value: TypeHierarchySubtypesRequest
) -> None:
    encode_type_hierarchy_item(writer, value.item)


def decode_type_hierarchy_subtypes_request(
    reader: Reader,
) -> TypeHierarchySubtypesRequest:
    field_0 = decode_type_hierarchy_item(reader)

    return TypeHierarchySubtypesRequest(
        item=field_0,
    )


@dataclass(frozen=True, slots=True)
class TypeHierarchyItemResponse:
    """Response payload for type hierarchy item queries."""

    """Type hierarchy item, if available."""
    item: TypeHierarchyItem | None


def encode_type_hierarchy_item_response(
    writer: Writer, value: TypeHierarchyItemResponse
) -> None:
    if value.item is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_type_hierarchy_item(writer, value.item)


def decode_type_hierarchy_item_response(reader: Reader) -> TypeHierarchyItemResponse:
    field_0 = reader.read_option(lambda: decode_type_hierarchy_item(reader))

    return TypeHierarchyItemResponse(
        item=field_0,
    )


@dataclass(frozen=True, slots=True)
class TypeHierarchySupertypesResponse:
    """Response payload for type hierarchy supertypes queries."""

    """Type hierarchy items."""
    items: Sequence[TypeHierarchyItem]


def encode_type_hierarchy_supertypes_response(
    writer: Writer, value: TypeHierarchySupertypesResponse
) -> None:
    writer.write_unsigned(len(value.items))
    for item_0 in value.items:
        encode_type_hierarchy_item(writer, item_0)


def decode_type_hierarchy_supertypes_response(
    reader: Reader,
) -> TypeHierarchySupertypesResponse:
    field_0 = [decode_type_hierarchy_item(reader) for _ in range(reader.read_number())]

    return TypeHierarchySupertypesResponse(
        items=field_0,
    )


@dataclass(frozen=True, slots=True)
class TypeHierarchySubtypesResponse:
    """Response payload for type hierarchy subtypes queries."""

    """Type hierarchy items."""
    items: Sequence[TypeHierarchyItem]


def encode_type_hierarchy_subtypes_response(
    writer: Writer, value: TypeHierarchySubtypesResponse
) -> None:
    writer.write_unsigned(len(value.items))
    for item_0 in value.items:
        encode_type_hierarchy_item(writer, item_0)


def decode_type_hierarchy_subtypes_response(
    reader: Reader,
) -> TypeHierarchySubtypesResponse:
    field_0 = [decode_type_hierarchy_item(reader) for _ in range(reader.read_number())]

    return TypeHierarchySubtypesResponse(
        items=field_0,
    )


__all__ = [
    "TypeHierarchyItemRequest",
    "encode_type_hierarchy_item_request",
    "decode_type_hierarchy_item_request",
    "TypeHierarchySupertypesRequest",
    "encode_type_hierarchy_supertypes_request",
    "decode_type_hierarchy_supertypes_request",
    "TypeHierarchyItem",
    "encode_type_hierarchy_item",
    "decode_type_hierarchy_item",
    "TypeHierarchyKind",
    "encode_type_hierarchy_kind",
    "decode_type_hierarchy_kind",
    "TypeHierarchySubtypesRequest",
    "encode_type_hierarchy_subtypes_request",
    "decode_type_hierarchy_subtypes_request",
    "TypeHierarchyItemResponse",
    "encode_type_hierarchy_item_response",
    "decode_type_hierarchy_item_response",
    "TypeHierarchySupertypesResponse",
    "encode_type_hierarchy_supertypes_response",
    "decode_type_hierarchy_supertypes_response",
    "TypeHierarchySubtypesResponse",
    "encode_type_hierarchy_subtypes_response",
    "decode_type_hierarchy_subtypes_response",
]
