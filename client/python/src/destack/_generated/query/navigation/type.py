# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_array,
    json_field,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.dir.symbol.symbol
import destack._generated.query.protocol.target


@dataclass(frozen=True, slots=True)
class TypeItemRequest:
    """Request the type item at a cursor position."""

    # the queried position
    position: destack._generated.query.protocol.target.Position

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_item_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeItemRequest:
        """Decode one TypeItemRequest."""
        return decode_type_item_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_item_request(self)

    @classmethod
    def from_json(cls, value: Json) -> TypeItemRequest:
        """Return one TypeItemRequest from one JSON value."""
        return from_json_type_item_request(value)


def encode_type_item_request(writer: BinaryWriter, value: TypeItemRequest) -> None:
    """Encode one TypeItemRequest."""
    destack._generated.query.protocol.target.encode_position(writer, value.position)


def decode_type_item_request(reader: BinaryReader) -> TypeItemRequest:
    """Decode one TypeItemRequest."""
    position = destack._generated.query.protocol.target.decode_position(reader)

    return TypeItemRequest(
        position=position,
    )


def to_json_type_item_request(value: TypeItemRequest) -> Json:
    """Return one JSON value for one TypeItemRequest."""
    return {
        "position": destack._generated.query.protocol.target.to_json_position(
            value.position
        ),
    }


def from_json_type_item_request(value: Json) -> TypeItemRequest:
    """Return one TypeItemRequest from one JSON value."""
    object_ = json_object(value)

    return TypeItemRequest(
        position=destack._generated.query.protocol.target.from_json_position(
            json_field(object_, "position")
        ),
    )


@dataclass(frozen=True, slots=True)
class SupertypesRequest:
    """Request supertypes."""

    # the type item to expand
    item: TypeItem

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_supertypes_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SupertypesRequest:
        """Decode one SupertypesRequest."""
        return decode_supertypes_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_supertypes_request(self)

    @classmethod
    def from_json(cls, value: Json) -> SupertypesRequest:
        """Return one SupertypesRequest from one JSON value."""
        return from_json_supertypes_request(value)


def encode_supertypes_request(writer: BinaryWriter, value: SupertypesRequest) -> None:
    """Encode one SupertypesRequest."""
    encode_type_item(writer, value.item)


def decode_supertypes_request(reader: BinaryReader) -> SupertypesRequest:
    """Decode one SupertypesRequest."""
    item = decode_type_item(reader)

    return SupertypesRequest(
        item=item,
    )


def to_json_supertypes_request(value: SupertypesRequest) -> Json:
    """Return one JSON value for one SupertypesRequest."""
    return {
        "item": to_json_type_item(value.item),
    }


def from_json_supertypes_request(value: Json) -> SupertypesRequest:
    """Return one SupertypesRequest from one JSON value."""
    object_ = json_object(value)

    return SupertypesRequest(
        item=from_json_type_item(json_field(object_, "item")),
    )


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_item(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeItem:
        """Decode one TypeItem."""
        return decode_type_item(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_item(self)

    @classmethod
    def from_json(cls, value: Json) -> TypeItem:
        """Return one TypeItem from one JSON value."""
        return from_json_type_item(value)


def encode_type_item(writer: BinaryWriter, value: TypeItem) -> None:
    """Encode one TypeItem."""
    writer.write_string(value.name)
    destack._generated.dir.symbol.symbol.encode_symbol_kind(writer, value.kind)
    if value.detail is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.detail)
    destack._generated.query.protocol.target.encode_target(writer, value.target)


def decode_type_item(reader: BinaryReader) -> TypeItem:
    """Decode one TypeItem."""
    name = reader.read_string()
    kind = destack._generated.dir.symbol.symbol.decode_symbol_kind(reader)
    detail = reader.read_option(lambda: reader.read_string())
    target = destack._generated.query.protocol.target.decode_target(reader)

    return TypeItem(
        name=name,
        kind=kind,
        detail=detail,
        target=target,
    )


def to_json_type_item(value: TypeItem) -> Json:
    """Return one JSON value for one TypeItem."""
    return {
        "name": value.name,
        "kind": destack._generated.dir.symbol.symbol.to_json_symbol_kind(value.kind),
        **({} if value.detail is None else {"detail": value.detail}),
        "target": destack._generated.query.protocol.target.to_json_target(value.target),
    }


def from_json_type_item(value: Json) -> TypeItem:
    """Return one TypeItem from one JSON value."""
    object_ = json_object(value)

    return TypeItem(
        name=json_string(json_field(object_, "name")),
        kind=destack._generated.dir.symbol.symbol.from_json_symbol_kind(
            json_field(object_, "kind")
        ),
        detail=json_optional(object_, "detail", lambda value: json_string(value)),
        target=destack._generated.query.protocol.target.from_json_target(
            json_field(object_, "target")
        ),
    )


@dataclass(frozen=True, slots=True)
class SubtypesRequest:
    """Request subtypes."""

    # the type item to expand
    item: TypeItem

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_subtypes_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SubtypesRequest:
        """Decode one SubtypesRequest."""
        return decode_subtypes_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_subtypes_request(self)

    @classmethod
    def from_json(cls, value: Json) -> SubtypesRequest:
        """Return one SubtypesRequest from one JSON value."""
        return from_json_subtypes_request(value)


def encode_subtypes_request(writer: BinaryWriter, value: SubtypesRequest) -> None:
    """Encode one SubtypesRequest."""
    encode_type_item(writer, value.item)


def decode_subtypes_request(reader: BinaryReader) -> SubtypesRequest:
    """Decode one SubtypesRequest."""
    item = decode_type_item(reader)

    return SubtypesRequest(
        item=item,
    )


def to_json_subtypes_request(value: SubtypesRequest) -> Json:
    """Return one JSON value for one SubtypesRequest."""
    return {
        "item": to_json_type_item(value.item),
    }


def from_json_subtypes_request(value: Json) -> SubtypesRequest:
    """Return one SubtypesRequest from one JSON value."""
    object_ = json_object(value)

    return SubtypesRequest(
        item=from_json_type_item(json_field(object_, "item")),
    )


@dataclass(frozen=True, slots=True)
class TypeItemResponse:
    """Response payload for type item queries."""

    # type hierarchy item, if available
    item: TypeItem | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_item_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeItemResponse:
        """Decode one TypeItemResponse."""
        return decode_type_item_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_item_response(self)

    @classmethod
    def from_json(cls, value: Json) -> TypeItemResponse:
        """Return one TypeItemResponse from one JSON value."""
        return from_json_type_item_response(value)


def encode_type_item_response(writer: BinaryWriter, value: TypeItemResponse) -> None:
    """Encode one TypeItemResponse."""
    if value.item is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_type_item(writer, value.item)


def decode_type_item_response(reader: BinaryReader) -> TypeItemResponse:
    """Decode one TypeItemResponse."""
    item = reader.read_option(lambda: decode_type_item(reader))

    return TypeItemResponse(
        item=item,
    )


def to_json_type_item_response(value: TypeItemResponse) -> Json:
    """Return one JSON value for one TypeItemResponse."""
    return {
        **({} if value.item is None else {"item": to_json_type_item(value.item)}),
    }


def from_json_type_item_response(value: Json) -> TypeItemResponse:
    """Return one TypeItemResponse from one JSON value."""
    object_ = json_object(value)

    return TypeItemResponse(
        item=json_optional(object_, "item", lambda value: from_json_type_item(value)),
    )


@dataclass(frozen=True, slots=True)
class SupertypesResponse:
    """Response payload for supertypes queries."""

    # type hierarchy items
    items: Sequence[TypeItem]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_supertypes_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SupertypesResponse:
        """Decode one SupertypesResponse."""
        return decode_supertypes_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_supertypes_response(self)

    @classmethod
    def from_json(cls, value: Json) -> SupertypesResponse:
        """Return one SupertypesResponse from one JSON value."""
        return from_json_supertypes_response(value)


def encode_supertypes_response(writer: BinaryWriter, value: SupertypesResponse) -> None:
    """Encode one SupertypesResponse."""
    writer.write_unsigned(len(value.items))
    for item_value_items_0 in value.items:
        encode_type_item(writer, item_value_items_0)


def decode_supertypes_response(reader: BinaryReader) -> SupertypesResponse:
    """Decode one SupertypesResponse."""
    items = [decode_type_item(reader) for _ in range(reader.read_number())]

    return SupertypesResponse(
        items=items,
    )


def to_json_supertypes_response(value: SupertypesResponse) -> Json:
    """Return one JSON value for one SupertypesResponse."""
    return {
        "items": [to_json_type_item(item_0) for item_0 in value.items],
    }


def from_json_supertypes_response(value: Json) -> SupertypesResponse:
    """Return one SupertypesResponse from one JSON value."""
    object_ = json_object(value)

    return SupertypesResponse(
        items=[
            from_json_type_item(item_0)
            for item_0 in json_array(json_field(object_, "items"))
        ],
    )


@dataclass(frozen=True, slots=True)
class SubtypesResponse:
    """Response payload for subtypes queries."""

    # type hierarchy items
    items: Sequence[TypeItem]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_subtypes_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SubtypesResponse:
        """Decode one SubtypesResponse."""
        return decode_subtypes_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_subtypes_response(self)

    @classmethod
    def from_json(cls, value: Json) -> SubtypesResponse:
        """Return one SubtypesResponse from one JSON value."""
        return from_json_subtypes_response(value)


def encode_subtypes_response(writer: BinaryWriter, value: SubtypesResponse) -> None:
    """Encode one SubtypesResponse."""
    writer.write_unsigned(len(value.items))
    for item_value_items_0 in value.items:
        encode_type_item(writer, item_value_items_0)


def decode_subtypes_response(reader: BinaryReader) -> SubtypesResponse:
    """Decode one SubtypesResponse."""
    items = [decode_type_item(reader) for _ in range(reader.read_number())]

    return SubtypesResponse(
        items=items,
    )


def to_json_subtypes_response(value: SubtypesResponse) -> Json:
    """Return one JSON value for one SubtypesResponse."""
    return {
        "items": [to_json_type_item(item_0) for item_0 in value.items],
    }


def from_json_subtypes_response(value: Json) -> SubtypesResponse:
    """Return one SubtypesResponse from one JSON value."""
    object_ = json_object(value)

    return SubtypesResponse(
        items=[
            from_json_type_item(item_0)
            for item_0 in json_array(json_field(object_, "items"))
        ],
    )


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
