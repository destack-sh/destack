# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_field,
    json_int,
    json_object,
    json_string,
)


@dataclass(frozen=True, slots=True)
class TargetLayout:
    """ABI layout facts for one target."""

    # target byte order
    endian: Endian
    # target pointer layout
    pointer: PointerLayout
    # stack alignment in bytes
    stack_alignment_bytes: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_target_layout(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TargetLayout:
        """Decode one TargetLayout."""
        return decode_target_layout(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_target_layout(self)

    @classmethod
    def from_json(cls, value: Json) -> TargetLayout:
        """Return one TargetLayout from one JSON value."""
        return from_json_target_layout(value)


def encode_target_layout(writer: BinaryWriter, value: TargetLayout) -> None:
    """Encode one TargetLayout."""
    encode_endian(writer, value.endian)
    encode_pointer_layout(writer, value.pointer)
    writer.write_unsigned(value.stack_alignment_bytes)


def decode_target_layout(reader: BinaryReader) -> TargetLayout:
    """Decode one TargetLayout."""
    endian = decode_endian(reader)
    pointer = decode_pointer_layout(reader)
    stack_alignment_bytes = reader.read_number()

    return TargetLayout(
        endian=endian,
        pointer=pointer,
        stack_alignment_bytes=stack_alignment_bytes,
    )


def to_json_target_layout(value: TargetLayout) -> Json:
    """Return one JSON value for one TargetLayout."""
    return {
        "endian": to_json_endian(value.endian),
        "pointer": to_json_pointer_layout(value.pointer),
        "stackAlignmentBytes": value.stack_alignment_bytes,
    }


def from_json_target_layout(value: Json) -> TargetLayout:
    """Return one TargetLayout from one JSON value."""
    object_ = json_object(value)

    return TargetLayout(
        endian=from_json_endian(json_field(object_, "endian")),
        pointer=from_json_pointer_layout(json_field(object_, "pointer")),
        stack_alignment_bytes=json_int(json_field(object_, "stackAlignmentBytes")),
    )


"""Byte order for target scalar memory operations."""
Endian: typing.TypeAlias = typing.Literal["little"] | typing.Literal["big"]


def encode_endian(writer: BinaryWriter, value: Endian) -> None:
    """Encode one Endian."""
    if value == "little":
        writer.write_unsigned(0)
    elif value == "big":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_endian(reader: BinaryReader) -> Endian:
    """Decode one Endian."""
    variant = reader.read_number()

    if variant == 0:
        return "little"
    elif variant == 1:
        return "big"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_endian(value: Endian) -> Json:
    """Return one JSON value for one Endian."""
    return value


def from_json_endian(value: Json) -> Endian:
    """Return one Endian from one JSON value."""
    variant = json_string(value)

    if variant == "little":
        return "little"
    elif variant == "big":
        return "big"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class PointerLayout:
    """Pointer representation on the target."""

    # pointer size in bytes
    size_bytes: int
    # pointer alignment in bytes
    alignment_bytes: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pointer_layout(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> PointerLayout:
        """Decode one PointerLayout."""
        return decode_pointer_layout(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pointer_layout(self)

    @classmethod
    def from_json(cls, value: Json) -> PointerLayout:
        """Return one PointerLayout from one JSON value."""
        return from_json_pointer_layout(value)


def encode_pointer_layout(writer: BinaryWriter, value: PointerLayout) -> None:
    """Encode one PointerLayout."""
    writer.write_byte(value.size_bytes)
    writer.write_byte(value.alignment_bytes)


def decode_pointer_layout(reader: BinaryReader) -> PointerLayout:
    """Decode one PointerLayout."""
    size_bytes = reader.read_byte()
    alignment_bytes = reader.read_byte()

    return PointerLayout(
        size_bytes=size_bytes,
        alignment_bytes=alignment_bytes,
    )


def to_json_pointer_layout(value: PointerLayout) -> Json:
    """Return one JSON value for one PointerLayout."""
    return {
        "sizeBytes": value.size_bytes,
        "alignmentBytes": value.alignment_bytes,
    }


def from_json_pointer_layout(value: Json) -> PointerLayout:
    """Return one PointerLayout from one JSON value."""
    object_ = json_object(value)

    return PointerLayout(
        size_bytes=json_int(json_field(object_, "sizeBytes")),
        alignment_bytes=json_int(json_field(object_, "alignmentBytes")),
    )


__all__ = [
    "TargetLayout",
    "encode_target_layout",
    "decode_target_layout",
    "to_json_target_layout",
    "from_json_target_layout",
    "Endian",
    "encode_endian",
    "decode_endian",
    "to_json_endian",
    "from_json_endian",
    "PointerLayout",
    "encode_pointer_layout",
    "decode_pointer_layout",
    "to_json_pointer_layout",
    "from_json_pointer_layout",
]
