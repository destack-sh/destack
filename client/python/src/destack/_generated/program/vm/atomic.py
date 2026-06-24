# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_bool,
    json_field,
    json_object,
    json_string,
)


@dataclass(frozen=True, slots=True)
class AtomicShape:
    """Address, width, and ordering for one atomic memory operation."""

    # the addressed memory space
    address: AtomicAddress
    # the atomic payload width
    width: AtomicWidth
    # the memory ordering
    order: AtomicOrder
    # whether integer results should be sign-extended
    is_signed: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_atomic_shape(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> AtomicShape:
        """Decode one AtomicShape."""
        return decode_atomic_shape(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_atomic_shape(self)

    @classmethod
    def from_json(cls, value: Json) -> AtomicShape:
        """Return one AtomicShape from one JSON value."""
        return from_json_atomic_shape(value)


def encode_atomic_shape(writer: BinaryWriter, value: AtomicShape) -> None:
    """Encode one AtomicShape."""
    encode_atomic_address(writer, value.address)
    encode_atomic_width(writer, value.width)
    encode_atomic_order(writer, value.order)
    writer.write_bool(value.is_signed)


def decode_atomic_shape(reader: BinaryReader) -> AtomicShape:
    """Decode one AtomicShape."""
    address = decode_atomic_address(reader)
    width = decode_atomic_width(reader)
    order = decode_atomic_order(reader)
    is_signed = reader.read_bool()

    return AtomicShape(
        address=address,
        width=width,
        order=order,
        is_signed=is_signed,
    )


def to_json_atomic_shape(value: AtomicShape) -> Json:
    """Return one JSON value for one AtomicShape."""
    return {
        "address": to_json_atomic_address(value.address),
        "width": to_json_atomic_width(value.width),
        "order": to_json_atomic_order(value.order),
        "isSigned": value.is_signed,
    }


def from_json_atomic_shape(value: Json) -> AtomicShape:
    """Return one AtomicShape from one JSON value."""
    object_ = json_object(value)

    return AtomicShape(
        address=from_json_atomic_address(json_field(object_, "address")),
        width=from_json_atomic_width(json_field(object_, "width")),
        order=from_json_atomic_order(json_field(object_, "order")),
        is_signed=json_bool(json_field(object_, "isSigned")),
    )


"""Memory address representation selected by atomic lowering."""
AtomicAddress: typing.TypeAlias = (
    typing.Literal["heap"]
    | typing.Literal["sharedHeap"]
    | typing.Literal["address"]
    | typing.Literal["stack"]
    | typing.Literal["frame"]
    | typing.Literal["static"]
)


def encode_atomic_address(writer: BinaryWriter, value: AtomicAddress) -> None:
    """Encode one AtomicAddress."""
    if value == "heap":
        writer.write_unsigned(0)
    elif value == "sharedHeap":
        writer.write_unsigned(1)
    elif value == "address":
        writer.write_unsigned(2)
    elif value == "stack":
        writer.write_unsigned(3)
    elif value == "frame":
        writer.write_unsigned(4)
    elif value == "static":
        writer.write_unsigned(5)
    else:
        raise SerdeError("unknown enum variant")


def decode_atomic_address(reader: BinaryReader) -> AtomicAddress:
    """Decode one AtomicAddress."""
    variant = reader.read_number()

    if variant == 0:
        return "heap"
    elif variant == 1:
        return "sharedHeap"
    elif variant == 2:
        return "address"
    elif variant == 3:
        return "stack"
    elif variant == 4:
        return "frame"
    elif variant == 5:
        return "static"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_atomic_address(value: AtomicAddress) -> Json:
    """Return one JSON value for one AtomicAddress."""
    return value


def from_json_atomic_address(value: Json) -> AtomicAddress:
    """Return one AtomicAddress from one JSON value."""
    variant = json_string(value)

    if variant == "heap":
        return "heap"
    elif variant == "sharedHeap":
        return "sharedHeap"
    elif variant == "address":
        return "address"
    elif variant == "stack":
        return "stack"
    elif variant == "frame":
        return "frame"
    elif variant == "static":
        return "static"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Atomic payload width selected by lowering."""
AtomicWidth: typing.TypeAlias = (
    typing.Literal["width8"]
    | typing.Literal["width16"]
    | typing.Literal["width32"]
    | typing.Literal["width64"]
)


def encode_atomic_width(writer: BinaryWriter, value: AtomicWidth) -> None:
    """Encode one AtomicWidth."""
    if value == "width8":
        writer.write_unsigned(0)
    elif value == "width16":
        writer.write_unsigned(1)
    elif value == "width32":
        writer.write_unsigned(2)
    elif value == "width64":
        writer.write_unsigned(3)
    else:
        raise SerdeError("unknown enum variant")


def decode_atomic_width(reader: BinaryReader) -> AtomicWidth:
    """Decode one AtomicWidth."""
    variant = reader.read_number()

    if variant == 0:
        return "width8"
    elif variant == 1:
        return "width16"
    elif variant == 2:
        return "width32"
    elif variant == 3:
        return "width64"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_atomic_width(value: AtomicWidth) -> Json:
    """Return one JSON value for one AtomicWidth."""
    return value


def from_json_atomic_width(value: Json) -> AtomicWidth:
    """Return one AtomicWidth from one JSON value."""
    variant = json_string(value)

    if variant == "width8":
        return "width8"
    elif variant == "width16":
        return "width16"
    elif variant == "width32":
        return "width32"
    elif variant == "width64":
        return "width64"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Atomic memory ordering selected by lowering."""
AtomicOrder: typing.TypeAlias = (
    typing.Literal["relaxed"]
    | typing.Literal["acquire"]
    | typing.Literal["release"]
    | typing.Literal["acquireRelease"]
    | typing.Literal["sequentiallyConsistent"]
)


def encode_atomic_order(writer: BinaryWriter, value: AtomicOrder) -> None:
    """Encode one AtomicOrder."""
    if value == "relaxed":
        writer.write_unsigned(0)
    elif value == "acquire":
        writer.write_unsigned(1)
    elif value == "release":
        writer.write_unsigned(2)
    elif value == "acquireRelease":
        writer.write_unsigned(3)
    elif value == "sequentiallyConsistent":
        writer.write_unsigned(4)
    else:
        raise SerdeError("unknown enum variant")


def decode_atomic_order(reader: BinaryReader) -> AtomicOrder:
    """Decode one AtomicOrder."""
    variant = reader.read_number()

    if variant == 0:
        return "relaxed"
    elif variant == 1:
        return "acquire"
    elif variant == 2:
        return "release"
    elif variant == 3:
        return "acquireRelease"
    elif variant == 4:
        return "sequentiallyConsistent"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_atomic_order(value: AtomicOrder) -> Json:
    """Return one JSON value for one AtomicOrder."""
    return value


def from_json_atomic_order(value: Json) -> AtomicOrder:
    """Return one AtomicOrder from one JSON value."""
    variant = json_string(value)

    if variant == "relaxed":
        return "relaxed"
    elif variant == "acquire":
        return "acquire"
    elif variant == "release":
        return "release"
    elif variant == "acquireRelease":
        return "acquireRelease"
    elif variant == "sequentiallyConsistent":
        return "sequentiallyConsistent"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "AtomicShape",
    "encode_atomic_shape",
    "decode_atomic_shape",
    "to_json_atomic_shape",
    "from_json_atomic_shape",
    "AtomicAddress",
    "encode_atomic_address",
    "decode_atomic_address",
    "to_json_atomic_address",
    "from_json_atomic_address",
    "AtomicWidth",
    "encode_atomic_width",
    "decode_atomic_width",
    "to_json_atomic_width",
    "from_json_atomic_width",
    "AtomicOrder",
    "encode_atomic_order",
    "decode_atomic_order",
    "to_json_atomic_order",
    "from_json_atomic_order",
]
