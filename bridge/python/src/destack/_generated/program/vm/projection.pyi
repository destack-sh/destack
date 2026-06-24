# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Projection: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Projection: ...

def encode_projection(writer: BinaryWriter, value: Projection) -> None: ...
def decode_projection(reader: BinaryReader) -> Projection: ...
def to_json_projection(value: Projection) -> Json: ...
def from_json_projection(value: Json) -> Projection: ...

@dataclass(frozen=True, slots=True)
class SliceProjection:
    """Compiled projection data for one slice descriptor."""

    # the slice data projection
    data: SlotProjection
    # the slice length projection
    length: SlotProjection
    # the backing element projection
    element: Projection

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SliceProjection: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SliceProjection: ...

def encode_slice_projection(writer: BinaryWriter, value: SliceProjection) -> None: ...
def decode_slice_projection(reader: BinaryReader) -> SliceProjection: ...
def to_json_slice_projection(value: SliceProjection) -> Json: ...
def from_json_slice_projection(value: Json) -> SliceProjection: ...

@dataclass(frozen=True, slots=True)
class SlotProjection:
    """Compiled projection from a base address to one physical cell slot."""

    # the fixed byte offset from the base address
    byte_offset: int
    # the byte width of the slot payload
    byte_len: int
    # the cell representation for this slot
    cell_layout: destack._generated.program.vm.value.CellLayout

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SlotProjection: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SlotProjection: ...

def encode_slot_projection(writer: BinaryWriter, value: SlotProjection) -> None: ...
def decode_slot_projection(reader: BinaryReader) -> SlotProjection: ...
def to_json_slot_projection(value: SlotProjection) -> Json: ...
def from_json_slot_projection(value: Json) -> SlotProjection: ...

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
