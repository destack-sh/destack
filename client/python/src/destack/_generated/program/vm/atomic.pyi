# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> AtomicShape: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> AtomicShape: ...

def encode_atomic_shape(writer: BinaryWriter, value: AtomicShape) -> None: ...
def decode_atomic_shape(reader: BinaryReader) -> AtomicShape: ...
def to_json_atomic_shape(value: AtomicShape) -> Json: ...
def from_json_atomic_shape(value: Json) -> AtomicShape: ...

"""Memory address representation selected by atomic lowering."""
AtomicAddress: typing.TypeAlias = (
    typing.Literal["heap"]
    | typing.Literal["sharedHeap"]
    | typing.Literal["address"]
    | typing.Literal["stack"]
    | typing.Literal["frame"]
    | typing.Literal["static"]
)

def encode_atomic_address(writer: BinaryWriter, value: AtomicAddress) -> None: ...
def decode_atomic_address(reader: BinaryReader) -> AtomicAddress: ...
def to_json_atomic_address(value: AtomicAddress) -> Json: ...
def from_json_atomic_address(value: Json) -> AtomicAddress: ...

"""Atomic payload width selected by lowering."""
AtomicWidth: typing.TypeAlias = (
    typing.Literal["width8"]
    | typing.Literal["width16"]
    | typing.Literal["width32"]
    | typing.Literal["width64"]
)

def encode_atomic_width(writer: BinaryWriter, value: AtomicWidth) -> None: ...
def decode_atomic_width(reader: BinaryReader) -> AtomicWidth: ...
def to_json_atomic_width(value: AtomicWidth) -> Json: ...
def from_json_atomic_width(value: Json) -> AtomicWidth: ...

"""Atomic memory ordering selected by lowering."""
AtomicOrder: typing.TypeAlias = (
    typing.Literal["relaxed"]
    | typing.Literal["acquire"]
    | typing.Literal["release"]
    | typing.Literal["acquireRelease"]
    | typing.Literal["sequentiallyConsistent"]
)

def encode_atomic_order(writer: BinaryWriter, value: AtomicOrder) -> None: ...
def decode_atomic_order(reader: BinaryReader) -> AtomicOrder: ...
def to_json_atomic_order(value: AtomicOrder) -> Json: ...
def from_json_atomic_order(value: Json) -> AtomicOrder: ...

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
