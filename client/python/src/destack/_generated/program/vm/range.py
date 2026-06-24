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
    json_int,
    json_object,
    json_string,
)

import destack._generated.mir.tree.node


@dataclass(frozen=True, slots=True)
class ArgumentRange:
    """Argument range within one function argument pool."""

    # start offset into the argument pool
    start: int
    # number of arguments in the range
    len: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_argument_range(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ArgumentRange:
        """Decode one ArgumentRange."""
        return decode_argument_range(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_argument_range(self)

    @classmethod
    def from_json(cls, value: Json) -> ArgumentRange:
        """Return one ArgumentRange from one JSON value."""
        return from_json_argument_range(value)


def encode_argument_range(writer: BinaryWriter, value: ArgumentRange) -> None:
    """Encode one ArgumentRange."""
    writer.write_unsigned(value.start)
    writer.write_unsigned(value.len)


def decode_argument_range(reader: BinaryReader) -> ArgumentRange:
    """Decode one ArgumentRange."""
    start = reader.read_number()
    len = reader.read_number()

    return ArgumentRange(
        start=start,
        len=len,
    )


def to_json_argument_range(value: ArgumentRange) -> Json:
    """Return one JSON value for one ArgumentRange."""
    return {
        "start": value.start,
        "len": value.len,
    }


def from_json_argument_range(value: Json) -> ArgumentRange:
    """Return one ArgumentRange from one JSON value."""
    object_ = json_object(value)

    return ArgumentRange(
        start=json_int(json_field(object_, "start")),
        len=json_int(json_field(object_, "len")),
    )


@dataclass(frozen=True, slots=True)
class MovePair:
    """Move pair for parameter binding."""

    # destination frame slot
    dest: MoveSlot
    # source frame slot or void fill
    source: MoveSource

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_move_pair(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> MovePair:
        """Decode one MovePair."""
        return decode_move_pair(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_move_pair(self)

    @classmethod
    def from_json(cls, value: Json) -> MovePair:
        """Return one MovePair from one JSON value."""
        return from_json_move_pair(value)


def encode_move_pair(writer: BinaryWriter, value: MovePair) -> None:
    """Encode one MovePair."""
    encode_move_slot(writer, value.dest)
    encode_move_source(writer, value.source)


def decode_move_pair(reader: BinaryReader) -> MovePair:
    """Decode one MovePair."""
    dest = decode_move_slot(reader)
    source = decode_move_source(reader)

    return MovePair(
        dest=dest,
        source=source,
    )


def to_json_move_pair(value: MovePair) -> Json:
    """Return one JSON value for one MovePair."""
    return {
        "dest": to_json_move_slot(value.dest),
        "source": to_json_move_source(value.source),
    }


def from_json_move_pair(value: Json) -> MovePair:
    """Return one MovePair from one JSON value."""
    object_ = json_object(value)

    return MovePair(
        dest=from_json_move_slot(json_field(object_, "dest")),
        source=from_json_move_source(json_field(object_, "source")),
    )


@dataclass(frozen=True, slots=True)
class MoveSlot:
    """One lowered frame move slot."""

    # the slot value type
    ty: destack._generated.mir.tree.node.LocalNodeId
    # byte offset from the frame base
    offset: int
    # slot byte length
    byte_len: int
    # whether this slot stores one cell
    is_cell: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_move_slot(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> MoveSlot:
        """Decode one MoveSlot."""
        return decode_move_slot(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_move_slot(self)

    @classmethod
    def from_json(cls, value: Json) -> MoveSlot:
        """Return one MoveSlot from one JSON value."""
        return from_json_move_slot(value)


def encode_move_slot(writer: BinaryWriter, value: MoveSlot) -> None:
    """Encode one MoveSlot."""
    destack._generated.mir.tree.node.encode_local_node_id(writer, value.ty)
    writer.write_unsigned(value.offset)
    writer.write_unsigned(value.byte_len)
    writer.write_bool(value.is_cell)


def decode_move_slot(reader: BinaryReader) -> MoveSlot:
    """Decode one MoveSlot."""
    ty = destack._generated.mir.tree.node.decode_local_node_id(reader)
    offset = reader.read_number()
    byte_len = reader.read_number()
    is_cell = reader.read_bool()

    return MoveSlot(
        ty=ty,
        offset=offset,
        byte_len=byte_len,
        is_cell=is_cell,
    )


def to_json_move_slot(value: MoveSlot) -> Json:
    """Return one JSON value for one MoveSlot."""
    return {
        "ty": destack._generated.mir.tree.node.to_json_local_node_id(value.ty),
        "offset": value.offset,
        "byteLen": value.byte_len,
        "isCell": value.is_cell,
    }


def from_json_move_slot(value: Json) -> MoveSlot:
    """Return one MoveSlot from one JSON value."""
    object_ = json_object(value)

    return MoveSlot(
        ty=destack._generated.mir.tree.node.from_json_local_node_id(
            json_field(object_, "ty")
        ),
        offset=json_int(json_field(object_, "offset")),
        byte_len=json_int(json_field(object_, "byteLen")),
        is_cell=json_bool(json_field(object_, "isCell")),
    )


@dataclass(frozen=True, slots=True)
class MoveSourceSlot:
    """Move from a frame slot."""

    slot: MoveSlot
    kind: typing.Literal["slot"] = "slot"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_move_source(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_move_source(self)


@dataclass(frozen=True, slots=True)
class MoveSourceVoid:
    """Write the canonical void value."""

    kind: typing.Literal["void"] = "void"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_move_source(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_move_source(self)


"""Source for one lowered frame move."""
MoveSource: typing.TypeAlias = MoveSourceSlot | MoveSourceVoid


def encode_move_source(writer: BinaryWriter, value: MoveSource) -> None:
    """Encode one MoveSource."""
    if value.kind == "slot":
        writer.write_unsigned(0)
        encode_move_slot(writer, value.slot)
    elif value.kind == "void":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_move_source(reader: BinaryReader) -> MoveSource:
    """Decode one MoveSource."""
    variant = reader.read_number()

    if variant == 0:
        slot = decode_move_slot(reader)

        return MoveSourceSlot(slot=slot)
    elif variant == 1:
        return MoveSourceVoid()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_move_source(value: MoveSource) -> Json:
    """Return one JSON value for one MoveSource."""
    if value.kind == "slot":
        return {
            "kind": "slot",
            "slot": to_json_move_slot(value.slot),
        }
    elif value.kind == "void":
        return {
            "kind": "void",
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_move_source(value: Json) -> MoveSource:
    """Return one MoveSource from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "slot":
        return MoveSourceSlot(slot=from_json_move_slot(json_field(object_, "slot")))
    elif kind == "void":
        return MoveSourceVoid()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class MoveRange:
    """Move range within one function move pool."""

    # start offset into the move pool
    start: int
    # number of pairs in the range
    len: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_move_range(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> MoveRange:
        """Decode one MoveRange."""
        return decode_move_range(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_move_range(self)

    @classmethod
    def from_json(cls, value: Json) -> MoveRange:
        """Return one MoveRange from one JSON value."""
        return from_json_move_range(value)


def encode_move_range(writer: BinaryWriter, value: MoveRange) -> None:
    """Encode one MoveRange."""
    writer.write_unsigned(value.start)
    writer.write_unsigned(value.len)


def decode_move_range(reader: BinaryReader) -> MoveRange:
    """Decode one MoveRange."""
    start = reader.read_number()
    len = reader.read_number()

    return MoveRange(
        start=start,
        len=len,
    )


def to_json_move_range(value: MoveRange) -> Json:
    """Return one JSON value for one MoveRange."""
    return {
        "start": value.start,
        "len": value.len,
    }


def from_json_move_range(value: Json) -> MoveRange:
    """Return one MoveRange from one JSON value."""
    object_ = json_object(value)

    return MoveRange(
        start=json_int(json_field(object_, "start")),
        len=json_int(json_field(object_, "len")),
    )


__all__ = [
    "ArgumentRange",
    "encode_argument_range",
    "decode_argument_range",
    "to_json_argument_range",
    "from_json_argument_range",
    "MovePair",
    "encode_move_pair",
    "decode_move_pair",
    "to_json_move_pair",
    "from_json_move_pair",
    "MoveSlot",
    "encode_move_slot",
    "decode_move_slot",
    "to_json_move_slot",
    "from_json_move_slot",
    "MoveSource",
    "encode_move_source",
    "decode_move_source",
    "to_json_move_source",
    "from_json_move_source",
    "MoveSourceSlot",
    "MoveSourceVoid",
    "MoveRange",
    "encode_move_range",
    "decode_move_range",
    "to_json_move_range",
    "from_json_move_range",
]
