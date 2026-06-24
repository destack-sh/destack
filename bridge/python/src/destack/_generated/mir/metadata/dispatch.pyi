# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

from destack._impl.mir.metadata.dispatch import (
    DispatchMetadataImpl,
)

import destack._generated.core.string
import destack._generated.mir.tree.node

"""Slot index inside a dispatch table."""
DispatchSlot: typing.TypeAlias = int

def encode_dispatch_slot(writer: BinaryWriter, value: DispatchSlot) -> None: ...
def decode_dispatch_slot(reader: BinaryReader) -> DispatchSlot: ...
def to_json_dispatch_slot(value: DispatchSlot) -> Json: ...
def from_json_dispatch_slot(value: Json) -> DispatchSlot: ...

@dataclass(frozen=True, slots=True)
class DispatchMetadata(DispatchMetadataImpl):
    """Canonical dispatch metadata for one MIR module."""

    # class dispatch tables
    vtables: Sequence[Vtable]
    # dynamic dispatch tables
    dynamic_tables: Sequence[DynamicTable]
    # dynamic slot layouts keyed by constraint type id
    dynamic_shapes: Mapping[destack._generated.mir.tree.node.LocalNodeId, DynamicShape]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DispatchMetadata: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DispatchMetadata: ...

def encode_dispatch_metadata(writer: BinaryWriter, value: DispatchMetadata) -> None: ...
def decode_dispatch_metadata(reader: BinaryReader) -> DispatchMetadata: ...
def to_json_dispatch_metadata(value: DispatchMetadata) -> Json: ...
def from_json_dispatch_metadata(value: Json) -> DispatchMetadata: ...

@dataclass(frozen=True, slots=True)
class Vtable:
    """Metadata for a class vtable."""

    # the class type owning this table
    ty: destack._generated.mir.tree.node.LocalNodeId
    # the static global containing this table
    global_: destack._generated.mir.tree.node.LocalNodeId
    # entries in declaration order
    entries: Sequence[VtableEntry]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Vtable: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Vtable: ...

def encode_vtable(writer: BinaryWriter, value: Vtable) -> None: ...
def decode_vtable(reader: BinaryReader) -> Vtable: ...
def to_json_vtable(value: Vtable) -> Json: ...
def from_json_vtable(value: Json) -> Vtable: ...

@dataclass(frozen=True, slots=True)
class VtableEntryTypeDescriptor:
    """Slot containing the runtime type descriptor."""

    kind: typing.Literal["typeDescriptor"] = "typeDescriptor"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class VtableEntryDestructor:
    """Slot containing a drop glue function."""

    # the drop glue function when present
    function: destack._generated.mir.tree.node.LocalNodeId | None
    kind: typing.Literal["destructor"] = "destructor"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class VtableEntryMethod:
    """Slot containing a method implementation."""

    # the concrete method implementation
    function: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["method"] = "method"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Entry in a class vtable."""
VtableEntry: typing.TypeAlias = (
    VtableEntryTypeDescriptor | VtableEntryDestructor | VtableEntryMethod
)

def encode_vtable_entry(writer: BinaryWriter, value: VtableEntry) -> None: ...
def decode_vtable_entry(reader: BinaryReader) -> VtableEntry: ...
def to_json_vtable_entry(value: VtableEntry) -> Json: ...
def from_json_vtable_entry(value: Json) -> VtableEntry: ...

@dataclass(frozen=True, slots=True)
class DynamicTable:
    """Metadata for one concrete implementation of one dynamic constraint."""

    # the concrete type providing the implementation
    concrete: destack._generated.mir.tree.node.LocalNodeId
    # the dynamic constraint type being dispatched
    constraint: destack._generated.mir.tree.node.LocalNodeId
    # the static global containing this table
    global_: destack._generated.mir.tree.node.LocalNodeId
    # slots in dynamic shape order
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
class DynamicEntryField:
    """Slot containing a field offset."""

    # the field offset in bytes
    offset: int
    kind: typing.Literal["field"] = "field"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DynamicEntryGetter:
    """Slot containing a concrete getter implementation."""

    # the concrete getter implementation
    function: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["getter"] = "getter"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DynamicEntrySetter:
    """Slot containing a concrete setter implementation."""

    # the concrete setter implementation
    function: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["setter"] = "setter"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DynamicEntryMethod:
    """Slot containing a concrete method implementation."""

    # the concrete method implementation
    function: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["method"] = "method"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DynamicEntryCall:
    """Slot containing a concrete call implementation."""

    # the concrete call implementation
    function: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["call"] = "call"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Entry in a dynamic dispatch table."""
DynamicEntry: typing.TypeAlias = (
    DynamicEntryField
    | DynamicEntryGetter
    | DynamicEntrySetter
    | DynamicEntryMethod
    | DynamicEntryCall
)

def encode_dynamic_entry(writer: BinaryWriter, value: DynamicEntry) -> None: ...
def decode_dynamic_entry(reader: BinaryReader) -> DynamicEntry: ...
def to_json_dynamic_entry(value: DynamicEntry) -> Json: ...
def from_json_dynamic_entry(value: Json) -> DynamicEntry: ...

@dataclass(frozen=True, slots=True)
class DynamicShape:
    """Slot layout for one dynamic constraint."""

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
class DynamicSlotGetter:
    """Getter slot."""

    # the getter name
    name: destack._generated.core.string.StringId
    # the getter signature
    signature: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["getter"] = "getter"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DynamicSlotSetter:
    """Setter slot."""

    # the setter name
    name: destack._generated.core.string.StringId
    # the setter signature
    signature: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["setter"] = "setter"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DynamicSlotMethod:
    """Method slot."""

    # the method name
    name: destack._generated.core.string.StringId
    # the method signature
    signature: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["method"] = "method"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DynamicSlotCall:
    """Call signature slot."""

    # the call signature
    signature: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["call"] = "call"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Slot descriptor for a dynamic layout."""
DynamicSlot: typing.TypeAlias = (
    DynamicSlotField
    | DynamicSlotGetter
    | DynamicSlotSetter
    | DynamicSlotMethod
    | DynamicSlotCall
)

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
    "DispatchMetadata",
    "encode_dispatch_metadata",
    "decode_dispatch_metadata",
    "to_json_dispatch_metadata",
    "from_json_dispatch_metadata",
    "Vtable",
    "encode_vtable",
    "decode_vtable",
    "to_json_vtable",
    "from_json_vtable",
    "VtableEntry",
    "encode_vtable_entry",
    "decode_vtable_entry",
    "to_json_vtable_entry",
    "from_json_vtable_entry",
    "VtableEntryTypeDescriptor",
    "VtableEntryDestructor",
    "VtableEntryMethod",
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
    "DynamicEntryField",
    "DynamicEntryGetter",
    "DynamicEntrySetter",
    "DynamicEntryMethod",
    "DynamicEntryCall",
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
    "DynamicSlotGetter",
    "DynamicSlotSetter",
    "DynamicSlotMethod",
    "DynamicSlotCall",
]
