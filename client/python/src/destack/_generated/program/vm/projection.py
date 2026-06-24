# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_field,
    json_int,
    json_object,
    json_optional,
)

import destack._generated.mir.tree.node
import destack._generated.program.vm.value


@dataclass(frozen=True, slots=True)
class Projection:
    """Compiled projection from a base address to one value."""

    # the projected value type
    value_type: destack._generated.mir.tree.node.LocalNodeId
    # the fixed byte offset from the base address
    byte_offset: int
    # the byte stride for indexed projections
    byte_stride: int
    # the number of addressable elements when statically known
    length: int
    # the byte width of the projected value
    byte_len: int
    # the cell representation for scalar projections
    cell_layout: destack._generated.program.vm.value.CellLayout | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_projection(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Projection:
        """Decode one Projection."""
        return decode_projection(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_projection(self)

    @classmethod
    def from_json(cls, value: Json) -> Projection:
        """Return one Projection from one JSON value."""
        return from_json_projection(value)


def encode_projection(writer: BinaryWriter, value: Projection) -> None:
    """Encode one Projection."""
    destack._generated.mir.tree.node.encode_local_node_id(writer, value.value_type)
    writer.write_unsigned(value.byte_offset)
    writer.write_unsigned(value.byte_stride)
    writer.write_unsigned(value.length)
    writer.write_unsigned(value.byte_len)
    if value.cell_layout is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.program.vm.value.encode_cell_layout(
            writer, value.cell_layout
        )


def decode_projection(reader: BinaryReader) -> Projection:
    """Decode one Projection."""
    value_type = destack._generated.mir.tree.node.decode_local_node_id(reader)
    byte_offset = reader.read_number()
    byte_stride = reader.read_number()
    length = reader.read_number()
    byte_len = reader.read_number()
    cell_layout = reader.read_option(
        lambda: destack._generated.program.vm.value.decode_cell_layout(reader)
    )

    return Projection(
        value_type=value_type,
        byte_offset=byte_offset,
        byte_stride=byte_stride,
        length=length,
        byte_len=byte_len,
        cell_layout=cell_layout,
    )


def to_json_projection(value: Projection) -> Json:
    """Return one JSON value for one Projection."""
    return {
        "valueType": destack._generated.mir.tree.node.to_json_local_node_id(
            value.value_type
        ),
        "byteOffset": value.byte_offset,
        "byteStride": value.byte_stride,
        "length": value.length,
        "byteLen": value.byte_len,
        **(
            {}
            if value.cell_layout is None
            else {
                "cellLayout": destack._generated.program.vm.value.to_json_cell_layout(
                    value.cell_layout
                )
            }
        ),
    }


def from_json_projection(value: Json) -> Projection:
    """Return one Projection from one JSON value."""
    object_ = json_object(value)

    return Projection(
        value_type=destack._generated.mir.tree.node.from_json_local_node_id(
            json_field(object_, "valueType")
        ),
        byte_offset=json_int(json_field(object_, "byteOffset")),
        byte_stride=json_int(json_field(object_, "byteStride")),
        length=json_int(json_field(object_, "length")),
        byte_len=json_int(json_field(object_, "byteLen")),
        cell_layout=json_optional(
            object_,
            "cellLayout",
            lambda value: destack._generated.program.vm.value.from_json_cell_layout(
                value
            ),
        ),
    )


@dataclass(frozen=True, slots=True)
class SliceProjection:
    """Compiled projection data for one slice descriptor."""

    # the slice data projection
    data: SlotProjection
    # the slice length projection
    length: SlotProjection
    # the backing element projection
    element: Projection

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_slice_projection(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SliceProjection:
        """Decode one SliceProjection."""
        return decode_slice_projection(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_slice_projection(self)

    @classmethod
    def from_json(cls, value: Json) -> SliceProjection:
        """Return one SliceProjection from one JSON value."""
        return from_json_slice_projection(value)


def encode_slice_projection(writer: BinaryWriter, value: SliceProjection) -> None:
    """Encode one SliceProjection."""
    encode_slot_projection(writer, value.data)
    encode_slot_projection(writer, value.length)
    encode_projection(writer, value.element)


def decode_slice_projection(reader: BinaryReader) -> SliceProjection:
    """Decode one SliceProjection."""
    data = decode_slot_projection(reader)
    length = decode_slot_projection(reader)
    element = decode_projection(reader)

    return SliceProjection(
        data=data,
        length=length,
        element=element,
    )


def to_json_slice_projection(value: SliceProjection) -> Json:
    """Return one JSON value for one SliceProjection."""
    return {
        "data": to_json_slot_projection(value.data),
        "length": to_json_slot_projection(value.length),
        "element": to_json_projection(value.element),
    }


def from_json_slice_projection(value: Json) -> SliceProjection:
    """Return one SliceProjection from one JSON value."""
    object_ = json_object(value)

    return SliceProjection(
        data=from_json_slot_projection(json_field(object_, "data")),
        length=from_json_slot_projection(json_field(object_, "length")),
        element=from_json_projection(json_field(object_, "element")),
    )


@dataclass(frozen=True, slots=True)
class SlotProjection:
    """Compiled projection from a base address to one physical cell slot."""

    # the fixed byte offset from the base address
    byte_offset: int
    # the byte width of the slot payload
    byte_len: int
    # the cell representation for this slot
    cell_layout: destack._generated.program.vm.value.CellLayout

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_slot_projection(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SlotProjection:
        """Decode one SlotProjection."""
        return decode_slot_projection(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_slot_projection(self)

    @classmethod
    def from_json(cls, value: Json) -> SlotProjection:
        """Return one SlotProjection from one JSON value."""
        return from_json_slot_projection(value)


def encode_slot_projection(writer: BinaryWriter, value: SlotProjection) -> None:
    """Encode one SlotProjection."""
    writer.write_unsigned(value.byte_offset)
    writer.write_unsigned(value.byte_len)
    destack._generated.program.vm.value.encode_cell_layout(writer, value.cell_layout)


def decode_slot_projection(reader: BinaryReader) -> SlotProjection:
    """Decode one SlotProjection."""
    byte_offset = reader.read_number()
    byte_len = reader.read_number()
    cell_layout = destack._generated.program.vm.value.decode_cell_layout(reader)

    return SlotProjection(
        byte_offset=byte_offset,
        byte_len=byte_len,
        cell_layout=cell_layout,
    )


def to_json_slot_projection(value: SlotProjection) -> Json:
    """Return one JSON value for one SlotProjection."""
    return {
        "byteOffset": value.byte_offset,
        "byteLen": value.byte_len,
        "cellLayout": destack._generated.program.vm.value.to_json_cell_layout(
            value.cell_layout
        ),
    }


def from_json_slot_projection(value: Json) -> SlotProjection:
    """Return one SlotProjection from one JSON value."""
    object_ = json_object(value)

    return SlotProjection(
        byte_offset=json_int(json_field(object_, "byteOffset")),
        byte_len=json_int(json_field(object_, "byteLen")),
        cell_layout=destack._generated.program.vm.value.from_json_cell_layout(
            json_field(object_, "cellLayout")
        ),
    )


__all__ = [
    "Projection",
    "encode_projection",
    "decode_projection",
    "to_json_projection",
    "from_json_projection",
    "SliceProjection",
    "encode_slice_projection",
    "decode_slice_projection",
    "to_json_slice_projection",
    "from_json_slice_projection",
    "SlotProjection",
    "encode_slot_projection",
    "decode_slot_projection",
    "to_json_slot_projection",
    "from_json_slot_projection",
]
