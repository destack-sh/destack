# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_array,
    json_field,
    json_int,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.core.string
import destack._generated.mir.tree.node

"""Slot index inside a dispatch table."""
DispatchSlot: typing.TypeAlias = int


def encode_dispatch_slot(writer: BinaryWriter, value: DispatchSlot) -> None:
    """Encode one DispatchSlot."""
    writer.write_unsigned(value)


def decode_dispatch_slot(reader: BinaryReader) -> DispatchSlot:
    """Decode one DispatchSlot."""
    return reader.read_number()


def to_json_dispatch_slot(value: DispatchSlot) -> Json:
    """Return one JSON value for one DispatchSlot."""
    return value


def from_json_dispatch_slot(value: Json) -> DispatchSlot:
    """Return one DispatchSlot from one JSON value."""
    return json_int(value)


@dataclass(frozen=True, slots=True)
class DispatchTable:
    """Canonical dispatch table for one MIR module."""

    # virtual dispatch tables
    virtual_tables: Sequence[VirtualTable]
    # dynamic dispatch tables
    dynamic_tables: Sequence[DynamicTable]
    # dynamic dispatch shapes keyed by constraint type id
    dynamic_shapes: Sequence[DynamicShape]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_dispatch_table(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DispatchTable:
        """Decode one DispatchTable."""
        return decode_dispatch_table(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_dispatch_table(self)

    @classmethod
    def from_json(cls, value: Json) -> DispatchTable:
        """Return one DispatchTable from one JSON value."""
        return from_json_dispatch_table(value)


def encode_dispatch_table(writer: BinaryWriter, value: DispatchTable) -> None:
    """Encode one DispatchTable."""
    writer.write_unsigned(len(value.virtual_tables))
    for item_value_virtual_tables_0 in value.virtual_tables:
        encode_virtual_table(writer, item_value_virtual_tables_0)
    writer.write_unsigned(len(value.dynamic_tables))
    for item_value_dynamic_tables_0 in value.dynamic_tables:
        encode_dynamic_table(writer, item_value_dynamic_tables_0)
    writer.write_unsigned(len(value.dynamic_shapes))
    for item_value_dynamic_shapes_0 in value.dynamic_shapes:
        encode_dynamic_shape(writer, item_value_dynamic_shapes_0)


def decode_dispatch_table(reader: BinaryReader) -> DispatchTable:
    """Decode one DispatchTable."""
    virtual_tables = [decode_virtual_table(reader) for _ in range(reader.read_number())]
    dynamic_tables = [decode_dynamic_table(reader) for _ in range(reader.read_number())]
    dynamic_shapes = [decode_dynamic_shape(reader) for _ in range(reader.read_number())]

    return DispatchTable(
        virtual_tables=virtual_tables,
        dynamic_tables=dynamic_tables,
        dynamic_shapes=dynamic_shapes,
    )


def to_json_dispatch_table(value: DispatchTable) -> Json:
    """Return one JSON value for one DispatchTable."""
    return {
        "virtualTables": [
            to_json_virtual_table(item_0) for item_0 in value.virtual_tables
        ],
        "dynamicTables": [
            to_json_dynamic_table(item_0) for item_0 in value.dynamic_tables
        ],
        "dynamicShapes": [
            to_json_dynamic_shape(item_0) for item_0 in value.dynamic_shapes
        ],
    }


def from_json_dispatch_table(value: Json) -> DispatchTable:
    """Return one DispatchTable from one JSON value."""
    object_ = json_object(value)

    return DispatchTable(
        virtual_tables=[
            from_json_virtual_table(item_0)
            for item_0 in json_array(json_field(object_, "virtualTables"))
        ],
        dynamic_tables=[
            from_json_dynamic_table(item_0)
            for item_0 in json_array(json_field(object_, "dynamicTables"))
        ],
        dynamic_shapes=[
            from_json_dynamic_shape(item_0)
            for item_0 in json_array(json_field(object_, "dynamicShapes"))
        ],
    )


@dataclass(frozen=True, slots=True)
class VirtualTable:
    """Table for one class virtual dispatch receiver."""

    # the class type owning this table
    ty: destack._generated.mir.tree.node.LocalNodeId
    # the drop glue function when one exists
    destructor: destack._generated.mir.tree.node.LocalNodeId | None
    # method implementations in declaration order
    methods: Sequence[destack._generated.mir.tree.node.LocalNodeId]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_virtual_table(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> VirtualTable:
        """Decode one VirtualTable."""
        return decode_virtual_table(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_virtual_table(self)

    @classmethod
    def from_json(cls, value: Json) -> VirtualTable:
        """Return one VirtualTable from one JSON value."""
        return from_json_virtual_table(value)


def encode_virtual_table(writer: BinaryWriter, value: VirtualTable) -> None:
    """Encode one VirtualTable."""
    destack._generated.mir.tree.node.encode_local_node_id(writer, value.ty)
    if value.destructor is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.destructor)
    writer.write_unsigned(len(value.methods))
    for item_value_methods_0 in value.methods:
        destack._generated.mir.tree.node.encode_local_node_id(
            writer, item_value_methods_0
        )


def decode_virtual_table(reader: BinaryReader) -> VirtualTable:
    """Decode one VirtualTable."""
    ty = destack._generated.mir.tree.node.decode_local_node_id(reader)
    destructor = reader.read_option(
        lambda: destack._generated.mir.tree.node.decode_local_node_id(reader)
    )
    methods = [
        destack._generated.mir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]

    return VirtualTable(
        ty=ty,
        destructor=destructor,
        methods=methods,
    )


def to_json_virtual_table(value: VirtualTable) -> Json:
    """Return one JSON value for one VirtualTable."""
    return {
        "ty": destack._generated.mir.tree.node.to_json_local_node_id(value.ty),
        **(
            {}
            if value.destructor is None
            else {
                "destructor": destack._generated.mir.tree.node.to_json_local_node_id(
                    value.destructor
                )
            }
        ),
        "methods": [
            destack._generated.mir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.methods
        ],
    }


def from_json_virtual_table(value: Json) -> VirtualTable:
    """Return one VirtualTable from one JSON value."""
    object_ = json_object(value)

    return VirtualTable(
        ty=destack._generated.mir.tree.node.from_json_local_node_id(
            json_field(object_, "ty")
        ),
        destructor=json_optional(
            object_,
            "destructor",
            lambda value: destack._generated.mir.tree.node.from_json_local_node_id(
                value
            ),
        ),
        methods=[
            destack._generated.mir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "methods"))
        ],
    )


@dataclass(frozen=True, slots=True)
class DynamicTable:
    """Table for one concrete implementation of one dynamic constraint."""

    # the concrete type providing the implementation
    concrete: destack._generated.mir.tree.node.LocalNodeId
    # the dynamic constraint type being dispatched
    constraint: destack._generated.mir.tree.node.LocalNodeId
    # entries in dynamic shape order
    entries: Sequence[DynamicEntry]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_dynamic_table(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DynamicTable:
        """Decode one DynamicTable."""
        return decode_dynamic_table(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_dynamic_table(self)

    @classmethod
    def from_json(cls, value: Json) -> DynamicTable:
        """Return one DynamicTable from one JSON value."""
        return from_json_dynamic_table(value)


def encode_dynamic_table(writer: BinaryWriter, value: DynamicTable) -> None:
    """Encode one DynamicTable."""
    destack._generated.mir.tree.node.encode_local_node_id(writer, value.concrete)
    destack._generated.mir.tree.node.encode_local_node_id(writer, value.constraint)
    writer.write_unsigned(len(value.entries))
    for item_value_entries_0 in value.entries:
        encode_dynamic_entry(writer, item_value_entries_0)


def decode_dynamic_table(reader: BinaryReader) -> DynamicTable:
    """Decode one DynamicTable."""
    concrete = destack._generated.mir.tree.node.decode_local_node_id(reader)
    constraint = destack._generated.mir.tree.node.decode_local_node_id(reader)
    entries = [decode_dynamic_entry(reader) for _ in range(reader.read_number())]

    return DynamicTable(
        concrete=concrete,
        constraint=constraint,
        entries=entries,
    )


def to_json_dynamic_table(value: DynamicTable) -> Json:
    """Return one JSON value for one DynamicTable."""
    return {
        "concrete": destack._generated.mir.tree.node.to_json_local_node_id(
            value.concrete
        ),
        "constraint": destack._generated.mir.tree.node.to_json_local_node_id(
            value.constraint
        ),
        "entries": [to_json_dynamic_entry(item_0) for item_0 in value.entries],
    }


def from_json_dynamic_table(value: Json) -> DynamicTable:
    """Return one DynamicTable from one JSON value."""
    object_ = json_object(value)

    return DynamicTable(
        concrete=destack._generated.mir.tree.node.from_json_local_node_id(
            json_field(object_, "concrete")
        ),
        constraint=destack._generated.mir.tree.node.from_json_local_node_id(
            json_field(object_, "constraint")
        ),
        entries=[
            from_json_dynamic_entry(item_0)
            for item_0 in json_array(json_field(object_, "entries"))
        ],
    )


@dataclass(frozen=True, slots=True)
class DynamicEntryFieldOffset:
    """Slot containing a field offset."""

    # the field offset in bytes
    offset: int
    kind: typing.Literal["fieldOffset"] = "fieldOffset"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_dynamic_entry(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_dynamic_entry(self)


@dataclass(frozen=True, slots=True)
class DynamicEntryFunction:
    """Slot containing a concrete function implementation."""

    # the concrete function implementation
    function: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["function"] = "function"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_dynamic_entry(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_dynamic_entry(self)


"""Entry in a dynamic dispatch table."""
DynamicEntry: typing.TypeAlias = DynamicEntryFieldOffset | DynamicEntryFunction


def encode_dynamic_entry(writer: BinaryWriter, value: DynamicEntry) -> None:
    """Encode one DynamicEntry."""
    if value.kind == "fieldOffset":
        writer.write_unsigned(0)
        writer.write_unsigned(value.offset)
    elif value.kind == "function":
        writer.write_unsigned(1)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.function)
    else:
        raise SerdeError("unknown enum variant")


def decode_dynamic_entry(reader: BinaryReader) -> DynamicEntry:
    """Decode one DynamicEntry."""
    variant = reader.read_number()

    if variant == 0:
        offset = reader.read_number()

        return DynamicEntryFieldOffset(
            offset=offset,
        )
    elif variant == 1:
        function = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return DynamicEntryFunction(
            function=function,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_dynamic_entry(value: DynamicEntry) -> Json:
    """Return one JSON value for one DynamicEntry."""
    if value.kind == "fieldOffset":
        return {
            "kind": "fieldOffset",
            "offset": value.offset,
        }
    elif value.kind == "function":
        return {
            "kind": "function",
            "function": destack._generated.mir.tree.node.to_json_local_node_id(
                value.function
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_dynamic_entry(value: Json) -> DynamicEntry:
    """Return one DynamicEntry from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "fieldOffset":
        return DynamicEntryFieldOffset(
            offset=json_int(json_field(object_, "offset")),
        )
    elif kind == "function":
        return DynamicEntryFunction(
            function=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "function")
            ),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class DynamicShape:
    """Table shape for one dynamic constraint."""

    # the dynamic constraint type owning this shape
    constraint: destack._generated.mir.tree.node.LocalNodeId
    # slots in declaration order
    slots: Sequence[DynamicSlot]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_dynamic_shape(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DynamicShape:
        """Decode one DynamicShape."""
        return decode_dynamic_shape(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_dynamic_shape(self)

    @classmethod
    def from_json(cls, value: Json) -> DynamicShape:
        """Return one DynamicShape from one JSON value."""
        return from_json_dynamic_shape(value)


def encode_dynamic_shape(writer: BinaryWriter, value: DynamicShape) -> None:
    """Encode one DynamicShape."""
    destack._generated.mir.tree.node.encode_local_node_id(writer, value.constraint)
    writer.write_unsigned(len(value.slots))
    for item_value_slots_0 in value.slots:
        encode_dynamic_slot(writer, item_value_slots_0)


def decode_dynamic_shape(reader: BinaryReader) -> DynamicShape:
    """Decode one DynamicShape."""
    constraint = destack._generated.mir.tree.node.decode_local_node_id(reader)
    slots = [decode_dynamic_slot(reader) for _ in range(reader.read_number())]

    return DynamicShape(
        constraint=constraint,
        slots=slots,
    )


def to_json_dynamic_shape(value: DynamicShape) -> Json:
    """Return one JSON value for one DynamicShape."""
    return {
        "constraint": destack._generated.mir.tree.node.to_json_local_node_id(
            value.constraint
        ),
        "slots": [to_json_dynamic_slot(item_0) for item_0 in value.slots],
    }


def from_json_dynamic_shape(value: Json) -> DynamicShape:
    """Return one DynamicShape from one JSON value."""
    object_ = json_object(value)

    return DynamicShape(
        constraint=destack._generated.mir.tree.node.from_json_local_node_id(
            json_field(object_, "constraint")
        ),
        slots=[
            from_json_dynamic_slot(item_0)
            for item_0 in json_array(json_field(object_, "slots"))
        ],
    )


@dataclass(frozen=True, slots=True)
class DynamicSlotField:
    """Field slot."""

    # the canonical dispatch field id
    field: destack._generated.mir.tree.node.LocalNodeId
    # the field name
    name: destack._generated.core.string.StringId
    kind: typing.Literal["field"] = "field"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_dynamic_slot(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_dynamic_slot(self)


@dataclass(frozen=True, slots=True)
class DynamicSlotFunction:
    """Function slot."""

    # the function name, absent for call signatures
    name: destack._generated.core.string.StringId | None
    # the function signature
    signature: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["function"] = "function"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_dynamic_slot(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_dynamic_slot(self)


"""Slot descriptor for a dynamic shape."""
DynamicSlot: typing.TypeAlias = DynamicSlotField | DynamicSlotFunction


def encode_dynamic_slot(writer: BinaryWriter, value: DynamicSlot) -> None:
    """Encode one DynamicSlot."""
    if value.kind == "field":
        writer.write_unsigned(0)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.field)
        destack._generated.core.string.encode_string_id(writer, value.name)
    elif value.kind == "function":
        writer.write_unsigned(1)
        if value.name is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.core.string.encode_string_id(writer, value.name)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.signature)
    else:
        raise SerdeError("unknown enum variant")


def decode_dynamic_slot(reader: BinaryReader) -> DynamicSlot:
    """Decode one DynamicSlot."""
    variant = reader.read_number()

    if variant == 0:
        field = destack._generated.mir.tree.node.decode_local_node_id(reader)
        name = destack._generated.core.string.decode_string_id(reader)

        return DynamicSlotField(
            field=field,
            name=name,
        )
    elif variant == 1:
        name = reader.read_option(
            lambda: destack._generated.core.string.decode_string_id(reader)
        )
        signature = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return DynamicSlotFunction(
            name=name,
            signature=signature,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_dynamic_slot(value: DynamicSlot) -> Json:
    """Return one JSON value for one DynamicSlot."""
    if value.kind == "field":
        return {
            "kind": "field",
            "field": destack._generated.mir.tree.node.to_json_local_node_id(
                value.field
            ),
            "name": destack._generated.core.string.to_json_string_id(value.name),
        }
    elif value.kind == "function":
        return {
            "kind": "function",
            **(
                {}
                if value.name is None
                else {
                    "name": destack._generated.core.string.to_json_string_id(value.name)
                }
            ),
            "signature": destack._generated.mir.tree.node.to_json_local_node_id(
                value.signature
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_dynamic_slot(value: Json) -> DynamicSlot:
    """Return one DynamicSlot from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "field":
        return DynamicSlotField(
            field=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "field")
            ),
            name=destack._generated.core.string.from_json_string_id(
                json_field(object_, "name")
            ),
        )
    elif kind == "function":
        return DynamicSlotFunction(
            name=json_optional(
                object_,
                "name",
                lambda value: destack._generated.core.string.from_json_string_id(value),
            ),
            signature=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "signature")
            ),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


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
