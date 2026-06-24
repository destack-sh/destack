# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_array,
    json_field,
    json_int,
    json_object,
    json_optional,
)

from destack._impl.mir.metadata.frame import (
    FrameTableImpl,
)
from destack._impl.mir.metadata.frame import (
    FrameMaterializationImpl,
)
from destack._impl.mir.metadata.frame import (
    FrameLayoutImpl,
)

import destack._generated.mir.tree.node


@dataclass(frozen=True, slots=True)
class FrameTable(FrameTableImpl):
    """Physical execution frame layout and materialization metadata."""

    # frame materializations by frame state id
    materializations: Sequence[FrameMaterialization]
    # frame layouts by id
    layouts: Sequence[FrameLayout]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_frame_table(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FrameTable:
        """Decode one FrameTable."""
        return decode_frame_table(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_frame_table(self)

    @classmethod
    def from_json(cls, value: Json) -> FrameTable:
        """Return one FrameTable from one JSON value."""
        return from_json_frame_table(value)


def encode_frame_table(writer: BinaryWriter, value: FrameTable) -> None:
    """Encode one FrameTable."""
    writer.write_unsigned(len(value.materializations))
    for item_value_materializations_0 in value.materializations:
        encode_frame_materialization(writer, item_value_materializations_0)
    writer.write_unsigned(len(value.layouts))
    for item_value_layouts_0 in value.layouts:
        encode_frame_layout(writer, item_value_layouts_0)


def decode_frame_table(reader: BinaryReader) -> FrameTable:
    """Decode one FrameTable."""
    materializations = [
        decode_frame_materialization(reader) for _ in range(reader.read_number())
    ]
    layouts = [decode_frame_layout(reader) for _ in range(reader.read_number())]

    return FrameTable(
        materializations=materializations,
        layouts=layouts,
    )


def to_json_frame_table(value: FrameTable) -> Json:
    """Return one JSON value for one FrameTable."""
    return {
        "materializations": [
            to_json_frame_materialization(item_0) for item_0 in value.materializations
        ],
        "layouts": [to_json_frame_layout(item_0) for item_0 in value.layouts],
    }


def from_json_frame_table(value: Json) -> FrameTable:
    """Return one FrameTable from one JSON value."""
    object_ = json_object(value)

    return FrameTable(
        materializations=[
            from_json_frame_materialization(item_0)
            for item_0 in json_array(json_field(object_, "materializations"))
        ],
        layouts=[
            from_json_frame_layout(item_0)
            for item_0 in json_array(json_field(object_, "layouts"))
        ],
    )


@dataclass(frozen=True, slots=True)
class FrameMaterialization(FrameMaterializationImpl):
    """Plan for reconstructing one execution frame."""

    # the reconstructed frame layout
    frame_layout: FrameLayoutId
    # frame slots copied into the materialized frame
    copied_slots: Sequence[FrameSlotId]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_frame_materialization(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FrameMaterialization:
        """Decode one FrameMaterialization."""
        return decode_frame_materialization(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_frame_materialization(self)

    @classmethod
    def from_json(cls, value: Json) -> FrameMaterialization:
        """Return one FrameMaterialization from one JSON value."""
        return from_json_frame_materialization(value)


def encode_frame_materialization(
    writer: BinaryWriter, value: FrameMaterialization
) -> None:
    """Encode one FrameMaterialization."""
    encode_frame_layout_id(writer, value.frame_layout)
    writer.write_unsigned(len(value.copied_slots))
    for item_value_copied_slots_0 in value.copied_slots:
        encode_frame_slot_id(writer, item_value_copied_slots_0)


def decode_frame_materialization(reader: BinaryReader) -> FrameMaterialization:
    """Decode one FrameMaterialization."""
    frame_layout = decode_frame_layout_id(reader)
    copied_slots = [decode_frame_slot_id(reader) for _ in range(reader.read_number())]

    return FrameMaterialization(
        frame_layout=frame_layout,
        copied_slots=copied_slots,
    )


def to_json_frame_materialization(value: FrameMaterialization) -> Json:
    """Return one JSON value for one FrameMaterialization."""
    return {
        "frameLayout": to_json_frame_layout_id(value.frame_layout),
        "copiedSlots": [to_json_frame_slot_id(item_0) for item_0 in value.copied_slots],
    }


def from_json_frame_materialization(value: Json) -> FrameMaterialization:
    """Return one FrameMaterialization from one JSON value."""
    object_ = json_object(value)

    return FrameMaterialization(
        frame_layout=from_json_frame_layout_id(json_field(object_, "frameLayout")),
        copied_slots=[
            from_json_frame_slot_id(item_0)
            for item_0 in json_array(json_field(object_, "copiedSlots"))
        ],
    )


"""One physical frame layout."""
FrameLayoutId: typing.TypeAlias = int


def encode_frame_layout_id(writer: BinaryWriter, value: FrameLayoutId) -> None:
    """Encode one FrameLayoutId."""
    writer.write_unsigned(value)


def decode_frame_layout_id(reader: BinaryReader) -> FrameLayoutId:
    """Decode one FrameLayoutId."""
    return reader.read_number()


def to_json_frame_layout_id(value: FrameLayoutId) -> Json:
    """Return one JSON value for one FrameLayoutId."""
    return value


def from_json_frame_layout_id(value: Json) -> FrameLayoutId:
    """Return one FrameLayoutId from one JSON value."""
    return json_int(value)


"""One physical frame slot."""
FrameSlotId: typing.TypeAlias = int


def encode_frame_slot_id(writer: BinaryWriter, value: FrameSlotId) -> None:
    """Encode one FrameSlotId."""
    writer.write_unsigned(value)


def decode_frame_slot_id(reader: BinaryReader) -> FrameSlotId:
    """Decode one FrameSlotId."""
    return reader.read_number()


def to_json_frame_slot_id(value: FrameSlotId) -> Json:
    """Return one JSON value for one FrameSlotId."""
    return value


def from_json_frame_slot_id(value: Json) -> FrameSlotId:
    """Return one FrameSlotId from one JSON value."""
    return json_int(value)


@dataclass(frozen=True, slots=True)
class FrameLayout(FrameLayoutImpl):
    """Physical byte layout for one frame."""

    # slots in frame order
    slots: Sequence[FrameSlot]
    # number of SSA value slots
    value_count: int
    # number of local slots
    local_count: int
    # callable environment slot
    environment_slot: FrameSlotId | None
    # the frame byte length
    byte_len: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_frame_layout(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FrameLayout:
        """Decode one FrameLayout."""
        return decode_frame_layout(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_frame_layout(self)

    @classmethod
    def from_json(cls, value: Json) -> FrameLayout:
        """Return one FrameLayout from one JSON value."""
        return from_json_frame_layout(value)


def encode_frame_layout(writer: BinaryWriter, value: FrameLayout) -> None:
    """Encode one FrameLayout."""
    writer.write_unsigned(len(value.slots))
    for item_value_slots_0 in value.slots:
        encode_frame_slot(writer, item_value_slots_0)
    writer.write_unsigned(value.value_count)
    writer.write_unsigned(value.local_count)
    if value.environment_slot is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_frame_slot_id(writer, value.environment_slot)
    writer.write_unsigned(value.byte_len)


def decode_frame_layout(reader: BinaryReader) -> FrameLayout:
    """Decode one FrameLayout."""
    slots = [decode_frame_slot(reader) for _ in range(reader.read_number())]
    value_count = reader.read_number()
    local_count = reader.read_number()
    environment_slot = reader.read_option(lambda: decode_frame_slot_id(reader))
    byte_len = reader.read_number()

    return FrameLayout(
        slots=slots,
        value_count=value_count,
        local_count=local_count,
        environment_slot=environment_slot,
        byte_len=byte_len,
    )


def to_json_frame_layout(value: FrameLayout) -> Json:
    """Return one JSON value for one FrameLayout."""
    return {
        "slots": [to_json_frame_slot(item_0) for item_0 in value.slots],
        "valueCount": value.value_count,
        "localCount": value.local_count,
        **(
            {}
            if value.environment_slot is None
            else {"environmentSlot": to_json_frame_slot_id(value.environment_slot)}
        ),
        "byteLen": value.byte_len,
    }


def from_json_frame_layout(value: Json) -> FrameLayout:
    """Return one FrameLayout from one JSON value."""
    object_ = json_object(value)

    return FrameLayout(
        slots=[
            from_json_frame_slot(item_0)
            for item_0 in json_array(json_field(object_, "slots"))
        ],
        value_count=json_int(json_field(object_, "valueCount")),
        local_count=json_int(json_field(object_, "localCount")),
        environment_slot=json_optional(
            object_, "environmentSlot", lambda value: from_json_frame_slot_id(value)
        ),
        byte_len=json_int(json_field(object_, "byteLen")),
    )


@dataclass(frozen=True, slots=True)
class FrameSlot:
    """Physical storage slot inside one frame."""

    # the byte offset from the frame base
    offset: int
    # the slot byte length
    byte_len: int
    # the slot byte alignment
    alignment: int
    # the slot value type
    ty: destack._generated.mir.tree.node.LocalNodeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_frame_slot(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FrameSlot:
        """Decode one FrameSlot."""
        return decode_frame_slot(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_frame_slot(self)

    @classmethod
    def from_json(cls, value: Json) -> FrameSlot:
        """Return one FrameSlot from one JSON value."""
        return from_json_frame_slot(value)


def encode_frame_slot(writer: BinaryWriter, value: FrameSlot) -> None:
    """Encode one FrameSlot."""
    writer.write_unsigned(value.offset)
    writer.write_unsigned(value.byte_len)
    writer.write_unsigned(value.alignment)
    destack._generated.mir.tree.node.encode_local_node_id(writer, value.ty)


def decode_frame_slot(reader: BinaryReader) -> FrameSlot:
    """Decode one FrameSlot."""
    offset = reader.read_number()
    byte_len = reader.read_number()
    alignment = reader.read_number()
    ty = destack._generated.mir.tree.node.decode_local_node_id(reader)

    return FrameSlot(
        offset=offset,
        byte_len=byte_len,
        alignment=alignment,
        ty=ty,
    )


def to_json_frame_slot(value: FrameSlot) -> Json:
    """Return one JSON value for one FrameSlot."""
    return {
        "offset": value.offset,
        "byteLen": value.byte_len,
        "alignment": value.alignment,
        "ty": destack._generated.mir.tree.node.to_json_local_node_id(value.ty),
    }


def from_json_frame_slot(value: Json) -> FrameSlot:
    """Return one FrameSlot from one JSON value."""
    object_ = json_object(value)

    return FrameSlot(
        offset=json_int(json_field(object_, "offset")),
        byte_len=json_int(json_field(object_, "byteLen")),
        alignment=json_int(json_field(object_, "alignment")),
        ty=destack._generated.mir.tree.node.from_json_local_node_id(
            json_field(object_, "ty")
        ),
    )


"""Logical frame state id at a resumable MIR point."""
FrameStateId: typing.TypeAlias = int


def encode_frame_state_id(writer: BinaryWriter, value: FrameStateId) -> None:
    """Encode one FrameStateId."""
    writer.write_unsigned(value)


def decode_frame_state_id(reader: BinaryReader) -> FrameStateId:
    """Decode one FrameStateId."""
    return reader.read_number()


def to_json_frame_state_id(value: FrameStateId) -> Json:
    """Return one JSON value for one FrameStateId."""
    return value


def from_json_frame_state_id(value: Json) -> FrameStateId:
    """Return one FrameStateId from one JSON value."""
    return json_int(value)


__all__ = [
    "FrameTable",
    "encode_frame_table",
    "decode_frame_table",
    "to_json_frame_table",
    "from_json_frame_table",
    "FrameMaterialization",
    "encode_frame_materialization",
    "decode_frame_materialization",
    "to_json_frame_materialization",
    "from_json_frame_materialization",
    "FrameLayoutId",
    "encode_frame_layout_id",
    "decode_frame_layout_id",
    "to_json_frame_layout_id",
    "from_json_frame_layout_id",
    "FrameSlotId",
    "encode_frame_slot_id",
    "decode_frame_slot_id",
    "to_json_frame_slot_id",
    "from_json_frame_slot_id",
    "FrameLayout",
    "encode_frame_layout",
    "decode_frame_layout",
    "to_json_frame_layout",
    "from_json_frame_layout",
    "FrameSlot",
    "encode_frame_slot",
    "decode_frame_slot",
    "to_json_frame_slot",
    "from_json_frame_slot",
    "FrameStateId",
    "encode_frame_state_id",
    "decode_frame_state_id",
    "to_json_frame_state_id",
    "from_json_frame_state_id",
]
