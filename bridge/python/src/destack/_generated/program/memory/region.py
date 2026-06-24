# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_bool,
    json_field,
    json_int,
    json_object,
)

import destack._generated.program.type


@dataclass(frozen=True, slots=True)
class StaticRegion:
    """One typed region inside static memory."""

    # the region id
    id: StaticId
    # the region byte offset
    offset: int
    # the region byte length
    byte_len: int
    # the region value type
    ty: destack._generated.program.type.TypeId
    # whether this region allows stores
    is_mutable: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_static_region(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> StaticRegion:
        """Decode one StaticRegion."""
        return decode_static_region(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_static_region(self)

    @classmethod
    def from_json(cls, value: Json) -> StaticRegion:
        """Return one StaticRegion from one JSON value."""
        return from_json_static_region(value)


def encode_static_region(writer: BinaryWriter, value: StaticRegion) -> None:
    """Encode one StaticRegion."""
    encode_static_id(writer, value.id)
    writer.write_unsigned(value.offset)
    writer.write_unsigned(value.byte_len)
    destack._generated.program.type.encode_type_id(writer, value.ty)
    writer.write_bool(value.is_mutable)


def decode_static_region(reader: BinaryReader) -> StaticRegion:
    """Decode one StaticRegion."""
    id = decode_static_id(reader)
    offset = reader.read_number()
    byte_len = reader.read_number()
    ty = destack._generated.program.type.decode_type_id(reader)
    is_mutable = reader.read_bool()

    return StaticRegion(
        id=id,
        offset=offset,
        byte_len=byte_len,
        ty=ty,
        is_mutable=is_mutable,
    )


def to_json_static_region(value: StaticRegion) -> Json:
    """Return one JSON value for one StaticRegion."""
    return {
        "id": to_json_static_id(value.id),
        "offset": value.offset,
        "byteLen": value.byte_len,
        "ty": destack._generated.program.type.to_json_type_id(value.ty),
        "isMutable": value.is_mutable,
    }


def from_json_static_region(value: Json) -> StaticRegion:
    """Return one StaticRegion from one JSON value."""
    object_ = json_object(value)

    return StaticRegion(
        id=from_json_static_id(json_field(object_, "id")),
        offset=json_int(json_field(object_, "offset")),
        byte_len=json_int(json_field(object_, "byteLen")),
        ty=destack._generated.program.type.from_json_type_id(json_field(object_, "ty")),
        is_mutable=json_bool(json_field(object_, "isMutable")),
    )


"""One static region."""
StaticId: typing.TypeAlias = int


def encode_static_id(writer: BinaryWriter, value: StaticId) -> None:
    """Encode one StaticId."""
    writer.write_unsigned(value)


def decode_static_id(reader: BinaryReader) -> StaticId:
    """Decode one StaticId."""
    return reader.read_number()


def to_json_static_id(value: StaticId) -> Json:
    """Return one JSON value for one StaticId."""
    return value


def from_json_static_id(value: Json) -> StaticId:
    """Return one StaticId from one JSON value."""
    return json_int(value)


__all__ = [
    "StaticRegion",
    "encode_static_region",
    "decode_static_region",
    "to_json_static_region",
    "from_json_static_region",
    "StaticId",
    "encode_static_id",
    "decode_static_id",
    "to_json_static_id",
    "from_json_static_id",
]
