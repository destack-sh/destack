# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FrameTable: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FrameTable: ...

def encode_frame_table(writer: BinaryWriter, value: FrameTable) -> None: ...
def decode_frame_table(reader: BinaryReader) -> FrameTable: ...
def to_json_frame_table(value: FrameTable) -> Json: ...
def from_json_frame_table(value: Json) -> FrameTable: ...

@dataclass(frozen=True, slots=True)
class FrameMaterialization(FrameMaterializationImpl):
    """Plan for reconstructing one execution frame."""

    # the reconstructed frame layout
    frame_layout: FrameLayoutId
    # frame slots copied into the materialized frame
    copied_slots: Sequence[FrameSlotId]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FrameMaterialization: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FrameMaterialization: ...

def encode_frame_materialization(
    writer: BinaryWriter, value: FrameMaterialization
) -> None: ...
def decode_frame_materialization(reader: BinaryReader) -> FrameMaterialization: ...
def to_json_frame_materialization(value: FrameMaterialization) -> Json: ...
def from_json_frame_materialization(value: Json) -> FrameMaterialization: ...

"""One physical frame layout."""
FrameLayoutId: typing.TypeAlias = int

def encode_frame_layout_id(writer: BinaryWriter, value: FrameLayoutId) -> None: ...
def decode_frame_layout_id(reader: BinaryReader) -> FrameLayoutId: ...
def to_json_frame_layout_id(value: FrameLayoutId) -> Json: ...
def from_json_frame_layout_id(value: Json) -> FrameLayoutId: ...

"""One physical frame slot."""
FrameSlotId: typing.TypeAlias = int

def encode_frame_slot_id(writer: BinaryWriter, value: FrameSlotId) -> None: ...
def decode_frame_slot_id(reader: BinaryReader) -> FrameSlotId: ...
def to_json_frame_slot_id(value: FrameSlotId) -> Json: ...
def from_json_frame_slot_id(value: Json) -> FrameSlotId: ...

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FrameLayout: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FrameLayout: ...

def encode_frame_layout(writer: BinaryWriter, value: FrameLayout) -> None: ...
def decode_frame_layout(reader: BinaryReader) -> FrameLayout: ...
def to_json_frame_layout(value: FrameLayout) -> Json: ...
def from_json_frame_layout(value: Json) -> FrameLayout: ...

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FrameSlot: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FrameSlot: ...

def encode_frame_slot(writer: BinaryWriter, value: FrameSlot) -> None: ...
def decode_frame_slot(reader: BinaryReader) -> FrameSlot: ...
def to_json_frame_slot(value: FrameSlot) -> Json: ...
def from_json_frame_slot(value: Json) -> FrameSlot: ...

"""Logical frame state id at a resumable MIR point."""
FrameStateId: typing.TypeAlias = int

def encode_frame_state_id(writer: BinaryWriter, value: FrameStateId) -> None: ...
def decode_frame_state_id(reader: BinaryReader) -> FrameStateId: ...
def to_json_frame_state_id(value: FrameStateId) -> Json: ...
def from_json_frame_state_id(value: Json) -> FrameStateId: ...

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
