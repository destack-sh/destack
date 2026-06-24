# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_array,
    json_field,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.query.core.target


@dataclass(frozen=True, slots=True)
class TypeHierarchyItemRequest:
    """Request the type hierarchy item at a cursor position."""

    # the queried position
    position: destack._generated.query.core.target.QueryPosition

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_hierarchy_item_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeHierarchyItemRequest:
        """Decode one TypeHierarchyItemRequest."""
        return decode_type_hierarchy_item_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_hierarchy_item_request(self)

    @classmethod
    def from_json(cls, value: Json) -> TypeHierarchyItemRequest:
        """Return one TypeHierarchyItemRequest from one JSON value."""
        return from_json_type_hierarchy_item_request(value)


def encode_type_hierarchy_item_request(
    writer: BinaryWriter, value: TypeHierarchyItemRequest
) -> None:
    """Encode one TypeHierarchyItemRequest."""
    destack._generated.query.core.target.encode_query_position(writer, value.position)


def decode_type_hierarchy_item_request(
    reader: BinaryReader,
) -> TypeHierarchyItemRequest:
    """Decode one TypeHierarchyItemRequest."""
    position = destack._generated.query.core.target.decode_query_position(reader)

    return TypeHierarchyItemRequest(
        position=position,
    )


def to_json_type_hierarchy_item_request(value: TypeHierarchyItemRequest) -> Json:
    """Return one JSON value for one TypeHierarchyItemRequest."""
    return {
        "position": destack._generated.query.core.target.to_json_query_position(
            value.position
        ),
    }


def from_json_type_hierarchy_item_request(value: Json) -> TypeHierarchyItemRequest:
    """Return one TypeHierarchyItemRequest from one JSON value."""
    object_ = json_object(value)

    return TypeHierarchyItemRequest(
        position=destack._generated.query.core.target.from_json_query_position(
            json_field(object_, "position")
        ),
    )


@dataclass(frozen=True, slots=True)
class TypeHierarchySupertypesRequest:
    """Request type hierarchy supertypes."""

    # the type hierarchy item to expand
    item: TypeHierarchyItem

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_hierarchy_supertypes_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeHierarchySupertypesRequest:
        """Decode one TypeHierarchySupertypesRequest."""
        return decode_type_hierarchy_supertypes_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_hierarchy_supertypes_request(self)

    @classmethod
    def from_json(cls, value: Json) -> TypeHierarchySupertypesRequest:
        """Return one TypeHierarchySupertypesRequest from one JSON value."""
        return from_json_type_hierarchy_supertypes_request(value)


def encode_type_hierarchy_supertypes_request(
    writer: BinaryWriter, value: TypeHierarchySupertypesRequest
) -> None:
    """Encode one TypeHierarchySupertypesRequest."""
    encode_type_hierarchy_item(writer, value.item)


def decode_type_hierarchy_supertypes_request(
    reader: BinaryReader,
) -> TypeHierarchySupertypesRequest:
    """Decode one TypeHierarchySupertypesRequest."""
    item = decode_type_hierarchy_item(reader)

    return TypeHierarchySupertypesRequest(
        item=item,
    )


def to_json_type_hierarchy_supertypes_request(
    value: TypeHierarchySupertypesRequest,
) -> Json:
    """Return one JSON value for one TypeHierarchySupertypesRequest."""
    return {
        "item": to_json_type_hierarchy_item(value.item),
    }


def from_json_type_hierarchy_supertypes_request(
    value: Json,
) -> TypeHierarchySupertypesRequest:
    """Return one TypeHierarchySupertypesRequest from one JSON value."""
    object_ = json_object(value)

    return TypeHierarchySupertypesRequest(
        item=from_json_type_hierarchy_item(json_field(object_, "item")),
    )


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_hierarchy_item(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeHierarchyItem:
        """Decode one TypeHierarchyItem."""
        return decode_type_hierarchy_item(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_hierarchy_item(self)

    @classmethod
    def from_json(cls, value: Json) -> TypeHierarchyItem:
        """Return one TypeHierarchyItem from one JSON value."""
        return from_json_type_hierarchy_item(value)


def encode_type_hierarchy_item(writer: BinaryWriter, value: TypeHierarchyItem) -> None:
    """Encode one TypeHierarchyItem."""
    writer.write_string(value.name)
    encode_type_hierarchy_kind(writer, value.kind)
    if value.detail is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.detail)
    destack._generated.query.core.target.encode_query_target(writer, value.target)


def decode_type_hierarchy_item(reader: BinaryReader) -> TypeHierarchyItem:
    """Decode one TypeHierarchyItem."""
    name = reader.read_string()
    kind = decode_type_hierarchy_kind(reader)
    detail = reader.read_option(lambda: reader.read_string())
    target = destack._generated.query.core.target.decode_query_target(reader)

    return TypeHierarchyItem(
        name=name,
        kind=kind,
        detail=detail,
        target=target,
    )


def to_json_type_hierarchy_item(value: TypeHierarchyItem) -> Json:
    """Return one JSON value for one TypeHierarchyItem."""
    return {
        "name": value.name,
        "kind": to_json_type_hierarchy_kind(value.kind),
        **({} if value.detail is None else {"detail": value.detail}),
        "target": destack._generated.query.core.target.to_json_query_target(
            value.target
        ),
    }


def from_json_type_hierarchy_item(value: Json) -> TypeHierarchyItem:
    """Return one TypeHierarchyItem from one JSON value."""
    object_ = json_object(value)

    return TypeHierarchyItem(
        name=json_string(json_field(object_, "name")),
        kind=from_json_type_hierarchy_kind(json_field(object_, "kind")),
        detail=json_optional(object_, "detail", lambda value: json_string(value)),
        target=destack._generated.query.core.target.from_json_query_target(
            json_field(object_, "target")
        ),
    )


"""Kind of type hierarchy item."""
TypeHierarchyKind: typing.TypeAlias = (
    typing.Literal["class"]
    | typing.Literal["interface"]
    | typing.Literal["struct"]
    | typing.Literal["enum"]
    | typing.Literal["typeAlias"]
)


def encode_type_hierarchy_kind(writer: BinaryWriter, value: TypeHierarchyKind) -> None:
    """Encode one TypeHierarchyKind."""
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


def decode_type_hierarchy_kind(reader: BinaryReader) -> TypeHierarchyKind:
    """Decode one TypeHierarchyKind."""
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


def to_json_type_hierarchy_kind(value: TypeHierarchyKind) -> Json:
    """Return one JSON value for one TypeHierarchyKind."""
    return value


def from_json_type_hierarchy_kind(value: Json) -> TypeHierarchyKind:
    """Return one TypeHierarchyKind from one JSON value."""
    variant = json_string(value)

    if variant == "class":
        return "class"
    elif variant == "interface":
        return "interface"
    elif variant == "struct":
        return "struct"
    elif variant == "enum":
        return "enum"
    elif variant == "typeAlias":
        return "typeAlias"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class TypeHierarchySubtypesRequest:
    """Request type hierarchy subtypes."""

    # the type hierarchy item to expand
    item: TypeHierarchyItem

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_hierarchy_subtypes_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeHierarchySubtypesRequest:
        """Decode one TypeHierarchySubtypesRequest."""
        return decode_type_hierarchy_subtypes_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_hierarchy_subtypes_request(self)

    @classmethod
    def from_json(cls, value: Json) -> TypeHierarchySubtypesRequest:
        """Return one TypeHierarchySubtypesRequest from one JSON value."""
        return from_json_type_hierarchy_subtypes_request(value)


def encode_type_hierarchy_subtypes_request(
    writer: BinaryWriter, value: TypeHierarchySubtypesRequest
) -> None:
    """Encode one TypeHierarchySubtypesRequest."""
    encode_type_hierarchy_item(writer, value.item)


def decode_type_hierarchy_subtypes_request(
    reader: BinaryReader,
) -> TypeHierarchySubtypesRequest:
    """Decode one TypeHierarchySubtypesRequest."""
    item = decode_type_hierarchy_item(reader)

    return TypeHierarchySubtypesRequest(
        item=item,
    )


def to_json_type_hierarchy_subtypes_request(
    value: TypeHierarchySubtypesRequest,
) -> Json:
    """Return one JSON value for one TypeHierarchySubtypesRequest."""
    return {
        "item": to_json_type_hierarchy_item(value.item),
    }


def from_json_type_hierarchy_subtypes_request(
    value: Json,
) -> TypeHierarchySubtypesRequest:
    """Return one TypeHierarchySubtypesRequest from one JSON value."""
    object_ = json_object(value)

    return TypeHierarchySubtypesRequest(
        item=from_json_type_hierarchy_item(json_field(object_, "item")),
    )


@dataclass(frozen=True, slots=True)
class TypeHierarchyItemResponse:
    """Response payload for type hierarchy item queries."""

    # type hierarchy item, if available
    item: TypeHierarchyItem | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_hierarchy_item_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeHierarchyItemResponse:
        """Decode one TypeHierarchyItemResponse."""
        return decode_type_hierarchy_item_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_hierarchy_item_response(self)

    @classmethod
    def from_json(cls, value: Json) -> TypeHierarchyItemResponse:
        """Return one TypeHierarchyItemResponse from one JSON value."""
        return from_json_type_hierarchy_item_response(value)


def encode_type_hierarchy_item_response(
    writer: BinaryWriter, value: TypeHierarchyItemResponse
) -> None:
    """Encode one TypeHierarchyItemResponse."""
    if value.item is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_type_hierarchy_item(writer, value.item)


def decode_type_hierarchy_item_response(
    reader: BinaryReader,
) -> TypeHierarchyItemResponse:
    """Decode one TypeHierarchyItemResponse."""
    item = reader.read_option(lambda: decode_type_hierarchy_item(reader))

    return TypeHierarchyItemResponse(
        item=item,
    )


def to_json_type_hierarchy_item_response(value: TypeHierarchyItemResponse) -> Json:
    """Return one JSON value for one TypeHierarchyItemResponse."""
    return {
        **(
            {}
            if value.item is None
            else {"item": to_json_type_hierarchy_item(value.item)}
        ),
    }


def from_json_type_hierarchy_item_response(value: Json) -> TypeHierarchyItemResponse:
    """Return one TypeHierarchyItemResponse from one JSON value."""
    object_ = json_object(value)

    return TypeHierarchyItemResponse(
        item=json_optional(
            object_, "item", lambda value: from_json_type_hierarchy_item(value)
        ),
    )


@dataclass(frozen=True, slots=True)
class TypeHierarchySupertypesResponse:
    """Response payload for type hierarchy supertypes queries."""

    # type hierarchy items
    items: Sequence[TypeHierarchyItem]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_hierarchy_supertypes_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeHierarchySupertypesResponse:
        """Decode one TypeHierarchySupertypesResponse."""
        return decode_type_hierarchy_supertypes_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_hierarchy_supertypes_response(self)

    @classmethod
    def from_json(cls, value: Json) -> TypeHierarchySupertypesResponse:
        """Return one TypeHierarchySupertypesResponse from one JSON value."""
        return from_json_type_hierarchy_supertypes_response(value)


def encode_type_hierarchy_supertypes_response(
    writer: BinaryWriter, value: TypeHierarchySupertypesResponse
) -> None:
    """Encode one TypeHierarchySupertypesResponse."""
    writer.write_unsigned(len(value.items))
    for item_value_items_0 in value.items:
        encode_type_hierarchy_item(writer, item_value_items_0)


def decode_type_hierarchy_supertypes_response(
    reader: BinaryReader,
) -> TypeHierarchySupertypesResponse:
    """Decode one TypeHierarchySupertypesResponse."""
    items = [decode_type_hierarchy_item(reader) for _ in range(reader.read_number())]

    return TypeHierarchySupertypesResponse(
        items=items,
    )


def to_json_type_hierarchy_supertypes_response(
    value: TypeHierarchySupertypesResponse,
) -> Json:
    """Return one JSON value for one TypeHierarchySupertypesResponse."""
    return {
        "items": [to_json_type_hierarchy_item(item_0) for item_0 in value.items],
    }


def from_json_type_hierarchy_supertypes_response(
    value: Json,
) -> TypeHierarchySupertypesResponse:
    """Return one TypeHierarchySupertypesResponse from one JSON value."""
    object_ = json_object(value)

    return TypeHierarchySupertypesResponse(
        items=[
            from_json_type_hierarchy_item(item_0)
            for item_0 in json_array(json_field(object_, "items"))
        ],
    )


@dataclass(frozen=True, slots=True)
class TypeHierarchySubtypesResponse:
    """Response payload for type hierarchy subtypes queries."""

    # type hierarchy items
    items: Sequence[TypeHierarchyItem]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_hierarchy_subtypes_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeHierarchySubtypesResponse:
        """Decode one TypeHierarchySubtypesResponse."""
        return decode_type_hierarchy_subtypes_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_hierarchy_subtypes_response(self)

    @classmethod
    def from_json(cls, value: Json) -> TypeHierarchySubtypesResponse:
        """Return one TypeHierarchySubtypesResponse from one JSON value."""
        return from_json_type_hierarchy_subtypes_response(value)


def encode_type_hierarchy_subtypes_response(
    writer: BinaryWriter, value: TypeHierarchySubtypesResponse
) -> None:
    """Encode one TypeHierarchySubtypesResponse."""
    writer.write_unsigned(len(value.items))
    for item_value_items_0 in value.items:
        encode_type_hierarchy_item(writer, item_value_items_0)


def decode_type_hierarchy_subtypes_response(
    reader: BinaryReader,
) -> TypeHierarchySubtypesResponse:
    """Decode one TypeHierarchySubtypesResponse."""
    items = [decode_type_hierarchy_item(reader) for _ in range(reader.read_number())]

    return TypeHierarchySubtypesResponse(
        items=items,
    )


def to_json_type_hierarchy_subtypes_response(
    value: TypeHierarchySubtypesResponse,
) -> Json:
    """Return one JSON value for one TypeHierarchySubtypesResponse."""
    return {
        "items": [to_json_type_hierarchy_item(item_0) for item_0 in value.items],
    }


def from_json_type_hierarchy_subtypes_response(
    value: Json,
) -> TypeHierarchySubtypesResponse:
    """Return one TypeHierarchySubtypesResponse from one JSON value."""
    object_ = json_object(value)

    return TypeHierarchySubtypesResponse(
        items=[
            from_json_type_hierarchy_item(item_0)
            for item_0 in json_array(json_field(object_, "items"))
        ],
    )


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
