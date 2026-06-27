# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.string
import destack._generated.mir.tree.node

"""Slot index inside a dispatch table."""
DispatchSlot: typing.TypeAlias = int

def encode_dispatch_slot(writer: BinaryWriter, value: DispatchSlot) -> None: ...
def decode_dispatch_slot(reader: BinaryReader) -> DispatchSlot: ...
def to_json_dispatch_slot(value: DispatchSlot) -> Json: ...
def from_json_dispatch_slot(value: Json) -> DispatchSlot: ...

@dataclass(frozen=True, slots=True)
class DispatchTable:
    """Canonical dispatch table for one MIR module."""

    # virtual dispatch tables
    virtual_tables: Sequence[VirtualTable]
    # dynamic dispatch tables
    dynamic_tables: Sequence[DynamicTable]
    # dynamic dispatch shapes keyed by constraint type id
    dynamic_shapes: Sequence[DynamicShape]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DispatchTable: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DispatchTable: ...

def encode_dispatch_table(writer: BinaryWriter, value: DispatchTable) -> None: ...
def decode_dispatch_table(reader: BinaryReader) -> DispatchTable: ...
def to_json_dispatch_table(value: DispatchTable) -> Json: ...
def from_json_dispatch_table(value: Json) -> DispatchTable: ...

@dataclass(frozen=True, slots=True)
class VirtualTable:
    """Table for one class virtual dispatch receiver."""

    # the class type owning this table
    ty: destack._generated.mir.tree.node.LocalNodeId
    # the drop glue function when one exists
    destructor: destack._generated.mir.tree.node.LocalNodeId | None
    # method implementations in declaration order
    methods: Sequence[destack._generated.mir.tree.node.LocalNodeId]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> VirtualTable: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> VirtualTable: ...

def encode_virtual_table(writer: BinaryWriter, value: VirtualTable) -> None: ...
def decode_virtual_table(reader: BinaryReader) -> VirtualTable: ...
def to_json_virtual_table(value: VirtualTable) -> Json: ...
def from_json_virtual_table(value: Json) -> VirtualTable: ...

@dataclass(frozen=True, slots=True)
class DynamicTable:
    """Table for one concrete implementation of one dynamic constraint."""

    # the concrete type providing the implementation
    concrete: destack._generated.mir.tree.node.LocalNodeId
    # the dynamic constraint type being dispatched
    constraint: destack._generated.mir.tree.node.LocalNodeId
    # entries in dynamic shape order
    entries: Sequence[DynamicEntry]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DynamicTable: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DynamicTable: ...

def encode_dynamic_table(writer: BinaryWriter, value: DynamicTable) -> None: ...
def decode_dynamic_table(reader: BinaryReader) -> DynamicTable: ...
def to_json_dynamic_table(value: DynamicTable) -> Json: ...
def from_json_dynamic_table(value: Json) -> DynamicTable: ...

@dataclass(frozen=True, slots=True)
class DynamicEntryFieldOffset:
    """Slot containing a field offset."""

    # the field offset in bytes
    offset: int
    kind: typing.Literal["fieldOffset"] = "fieldOffset"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DynamicEntryFunction:
    """Slot containing a concrete function implementation."""

    # the concrete function implementation
    function: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["function"] = "function"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Entry in a dynamic dispatch table."""
DynamicEntry: typing.TypeAlias = DynamicEntryFieldOffset | DynamicEntryFunction

def encode_dynamic_entry(writer: BinaryWriter, value: DynamicEntry) -> None: ...
def decode_dynamic_entry(reader: BinaryReader) -> DynamicEntry: ...
def to_json_dynamic_entry(value: DynamicEntry) -> Json: ...
def from_json_dynamic_entry(value: Json) -> DynamicEntry: ...

@dataclass(frozen=True, slots=True)
class DynamicShape:
    """Table shape for one dynamic constraint."""

    # the dynamic constraint type owning this shape
    constraint: destack._generated.mir.tree.node.LocalNodeId
    # slots in declaration order
    slots: Sequence[DynamicSlot]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DynamicShape: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DynamicShape: ...

def encode_dynamic_shape(writer: BinaryWriter, value: DynamicShape) -> None: ...
def decode_dynamic_shape(reader: BinaryReader) -> DynamicShape: ...
def to_json_dynamic_shape(value: DynamicShape) -> Json: ...
def from_json_dynamic_shape(value: Json) -> DynamicShape: ...

@dataclass(frozen=True, slots=True)
class DynamicSlotField:
    """Field slot."""

    # the canonical dispatch field id
    field: destack._generated.mir.tree.node.LocalNodeId
    # the field name
    name: destack._generated.core.string.StringId
    kind: typing.Literal["field"] = "field"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DynamicSlotFunction:
    """Function slot."""

    # the function name, absent for call signatures
    name: destack._generated.core.string.StringId | None
    # the function signature
    signature: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["function"] = "function"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Slot descriptor for a dynamic shape."""
DynamicSlot: typing.TypeAlias = DynamicSlotField | DynamicSlotFunction

def encode_dynamic_slot(writer: BinaryWriter, value: DynamicSlot) -> None: ...
def decode_dynamic_slot(reader: BinaryReader) -> DynamicSlot: ...
def to_json_dynamic_slot(value: DynamicSlot) -> Json: ...
def from_json_dynamic_slot(value: Json) -> DynamicSlot: ...

__all__ = [
    "DispatchSlot",
    "encode_dispatch_slot",
    "decode_dispatch_slot",
    "to_json_dispatch_slot",
    "from_json_dispatch_slot",
    "DispatchTable",
    "encode_dispatch_table",
    "decode_dispatch_table",
    "to_json_dispatch_table",
    "from_json_dispatch_table",
    "VirtualTable",
    "encode_virtual_table",
    "decode_virtual_table",
    "to_json_virtual_table",
    "from_json_virtual_table",
    "DynamicTable",
    "encode_dynamic_table",
    "decode_dynamic_table",
    "to_json_dynamic_table",
    "from_json_dynamic_table",
    "DynamicEntry",
    "encode_dynamic_entry",
    "decode_dynamic_entry",
    "to_json_dynamic_entry",
    "from_json_dynamic_entry",
    "DynamicEntryFieldOffset",
    "DynamicEntryFunction",
    "DynamicShape",
    "encode_dynamic_shape",
    "decode_dynamic_shape",
    "to_json_dynamic_shape",
    "from_json_dynamic_shape",
    "DynamicSlot",
    "encode_dynamic_slot",
    "decode_dynamic_slot",
    "to_json_dynamic_slot",
    "from_json_dynamic_slot",
    "DynamicSlotField",
    "DynamicSlotFunction",
]
