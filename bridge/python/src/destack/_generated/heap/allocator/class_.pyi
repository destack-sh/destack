# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

@dataclass(frozen=True, slots=True)
class SizeClassTable:
    """One canonical size-class table used by the heap."""

    # the ordered size classes in bytes
    classes: Sequence[SizeClass]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SizeClassTable: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SizeClassTable: ...

def encode_size_class_table(writer: BinaryWriter, value: SizeClassTable) -> None: ...
def decode_size_class_table(reader: BinaryReader) -> SizeClassTable: ...
def to_json_size_class_table(value: SizeClassTable) -> Json: ...
def from_json_size_class_table(value: Json) -> SizeClassTable: ...

@dataclass(frozen=True, slots=True)
class SizeClass:
    """One fixed-size small block class."""

    # the slot payload size in bytes
    bytes: int
    # the span width in bytes, or zero for the default span size
    span_size_bytes: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SizeClass: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SizeClass: ...

def encode_size_class(writer: BinaryWriter, value: SizeClass) -> None: ...
def decode_size_class(reader: BinaryReader) -> SizeClass: ...
def to_json_size_class(value: SizeClass) -> Json: ...
def from_json_size_class(value: Json) -> SizeClass: ...

__all__ = [
    "SizeClassTable",
    "encode_size_class_table",
    "decode_size_class_table",
    "to_json_size_class_table",
    "from_json_size_class_table",
    "SizeClass",
    "encode_size_class",
    "decode_size_class",
    "to_json_size_class",
    "from_json_size_class",
]
