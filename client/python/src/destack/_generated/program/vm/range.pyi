# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.mir.tree.node

@dataclass(frozen=True, slots=True)
class ArgumentRange:
    """Argument range within one function argument pool."""

    # start offset into the argument pool
    start: int
    # number of arguments in the range
    len: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ArgumentRange: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ArgumentRange: ...

def encode_argument_range(writer: BinaryWriter, value: ArgumentRange) -> None: ...
def decode_argument_range(reader: BinaryReader) -> ArgumentRange: ...
def to_json_argument_range(value: ArgumentRange) -> Json: ...
def from_json_argument_range(value: Json) -> ArgumentRange: ...

@dataclass(frozen=True, slots=True)
class MovePair:
    """Move pair for parameter binding."""

    # destination frame slot
    dest: MoveSlot
    # source frame slot or void fill
    source: MoveSource

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> MovePair: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> MovePair: ...

def encode_move_pair(writer: BinaryWriter, value: MovePair) -> None: ...
def decode_move_pair(reader: BinaryReader) -> MovePair: ...
def to_json_move_pair(value: MovePair) -> Json: ...
def from_json_move_pair(value: Json) -> MovePair: ...

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> MoveSlot: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> MoveSlot: ...

def encode_move_slot(writer: BinaryWriter, value: MoveSlot) -> None: ...
def decode_move_slot(reader: BinaryReader) -> MoveSlot: ...
def to_json_move_slot(value: MoveSlot) -> Json: ...
def from_json_move_slot(value: Json) -> MoveSlot: ...

@dataclass(frozen=True, slots=True)
class MoveSourceSlot:
    """Move from a frame slot."""

    slot: MoveSlot
    kind: typing.Literal["slot"] = "slot"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class MoveSourceVoid:
    """Write the canonical void value."""

    kind: typing.Literal["void"] = "void"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Source for one lowered frame move."""
MoveSource: typing.TypeAlias = MoveSourceSlot | MoveSourceVoid

def encode_move_source(writer: BinaryWriter, value: MoveSource) -> None: ...
def decode_move_source(reader: BinaryReader) -> MoveSource: ...
def to_json_move_source(value: MoveSource) -> Json: ...
def from_json_move_source(value: Json) -> MoveSource: ...

@dataclass(frozen=True, slots=True)
class MoveRange:
    """Move range within one function move pool."""

    # start offset into the move pool
    start: int
    # number of pairs in the range
    len: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> MoveRange: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> MoveRange: ...

def encode_move_range(writer: BinaryWriter, value: MoveRange) -> None: ...
def decode_move_range(reader: BinaryReader) -> MoveRange: ...
def to_json_move_range(value: MoveRange) -> Json: ...
def from_json_move_range(value: Json) -> MoveRange: ...

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
