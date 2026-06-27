# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

@dataclass(frozen=True, slots=True)
class TargetLayout:
    """ABI layout facts for one target."""

    # target byte order
    endian: Endian
    # target pointer layout
    pointer: PointerLayout
    # stack alignment in bytes
    stack_alignment_bytes: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TargetLayout: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TargetLayout: ...

def encode_target_layout(writer: BinaryWriter, value: TargetLayout) -> None: ...
def decode_target_layout(reader: BinaryReader) -> TargetLayout: ...
def to_json_target_layout(value: TargetLayout) -> Json: ...
def from_json_target_layout(value: Json) -> TargetLayout: ...

"""Byte order for target scalar memory operations."""
Endian: typing.TypeAlias = typing.Literal["little"] | typing.Literal["big"]

def encode_endian(writer: BinaryWriter, value: Endian) -> None: ...
def decode_endian(reader: BinaryReader) -> Endian: ...
def to_json_endian(value: Endian) -> Json: ...
def from_json_endian(value: Json) -> Endian: ...

@dataclass(frozen=True, slots=True)
class PointerLayout:
    """Pointer representation on the target."""

    # pointer size in bytes
    size_bytes: int
    # pointer alignment in bytes
    alignment_bytes: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> PointerLayout: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> PointerLayout: ...

def encode_pointer_layout(writer: BinaryWriter, value: PointerLayout) -> None: ...
def decode_pointer_layout(reader: BinaryReader) -> PointerLayout: ...
def to_json_pointer_layout(value: PointerLayout) -> Json: ...
def from_json_pointer_layout(value: Json) -> PointerLayout: ...

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
