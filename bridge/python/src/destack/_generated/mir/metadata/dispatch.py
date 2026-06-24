# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
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
    nested_bytes,
)

from destack._impl.mir.metadata.dispatch import (
    DispatchMetadataImpl,
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
class DispatchMetadata(DispatchMetadataImpl):
    """Canonical dispatch metadata for one MIR module."""

    # class dispatch tables
    vtables: Sequence[Vtable]
    # dynamic dispatch tables
    dynamic_tables: Sequence[DynamicTable]
    # dynamic slot layouts keyed by constraint type id
    dynamic_shapes: Mapping[destack._generated.mir.tree.node.LocalNodeId, DynamicShape]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_dispatch_metadata(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DispatchMetadata:
        """Decode one DispatchMetadata."""
        return decode_dispatch_metadata(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_dispatch_metadata(self)

    @classmethod
    def from_json(cls, value: Json) -> DispatchMetadata:
        """Return one DispatchMetadata from one JSON value."""
        return from_json_dispatch_metadata(value)


def encode_dispatch_metadata(writer: BinaryWriter, value: DispatchMetadata) -> None:
    """Encode one DispatchMetadata."""
    writer.write_unsigned(len(value.vtables))
    for item_value_vtables_0 in value.vtables:
        encode_vtable(writer, item_value_vtables_0)
    writer.write_unsigned(len(value.dynamic_tables))
    for item_value_dynamic_tables_0 in value.dynamic_tables:
        encode_dynamic_table(writer, item_value_dynamic_tables_0)
    entries_value_dynamic_shapes_0 = []
    for (
        key_value_dynamic_shapes_0,
        item_value_dynamic_shapes_0,
    ) in value.dynamic_shapes.items():

        def write_key_value_dynamic_shapes_0(writer: BinaryWriter) -> None:
            destack._generated.mir.tree.node.encode_local_node_id(
                writer, key_value_dynamic_shapes_0
            )

        key_bytes = nested_bytes(write_key_value_dynamic_shapes_0)
        entries_value_dynamic_shapes_0.append(
            (key_value_dynamic_shapes_0, item_value_dynamic_shapes_0, key_bytes)
        )
    entries_value_dynamic_shapes_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_dynamic_shapes_0))
    for entry_value_dynamic_shapes_0 in entries_value_dynamic_shapes_0:
        destack._generated.mir.tree.node.encode_local_node_id(
            writer, entry_value_dynamic_shapes_0[0]
        )
        encode_dynamic_shape(writer, entry_value_dynamic_shapes_0[1])


def decode_dispatch_metadata(reader: BinaryReader) -> DispatchMetadata:
    """Decode one DispatchMetadata."""
    vtables = [decode_vtable(reader) for _ in range(reader.read_number())]
    dynamic_tables = [decode_dynamic_table(reader) for _ in range(reader.read_number())]
    dynamic_shapes = {
        destack._generated.mir.tree.node.decode_local_node_id(
            reader
        ): decode_dynamic_shape(reader)
        for _ in range(reader.read_number())
    }

    return DispatchMetadata(
        vtables=vtables,
        dynamic_tables=dynamic_tables,
        dynamic_shapes=dynamic_shapes,
    )


def to_json_dispatch_metadata(value: DispatchMetadata) -> Json:
    """Return one JSON value for one DispatchMetadata."""
    return {
        "vtables": [to_json_vtable(item_0) for item_0 in value.vtables],
        "dynamicTables": [
            to_json_dynamic_table(item_0) for item_0 in value.dynamic_tables
        ],
        "dynamicShapes": [
            [
                destack._generated.mir.tree.node.to_json_local_node_id(key_0),
                to_json_dynamic_shape(item_0),
            ]
            for key_0, item_0 in value.dynamic_shapes.items()
        ],
    }


def from_json_dispatch_metadata(value: Json) -> DispatchMetadata:
    """Return one DispatchMetadata from one JSON value."""
    object_ = json_object(value)

    return DispatchMetadata(
        vtables=[
            from_json_vtable(item_0)
            for item_0 in json_array(json_field(object_, "vtables"))
        ],
        dynamic_tables=[
            from_json_dynamic_table(item_0)
            for item_0 in json_array(json_field(object_, "dynamicTables"))
        ],
        dynamic_shapes={
            destack._generated.mir.tree.node.from_json_local_node_id(
                key_0
            ): from_json_dynamic_shape(item_0)
            for key_0, item_0 in json_array(json_field(object_, "dynamicShapes"))
        },
    )


@dataclass(frozen=True, slots=True)
class Vtable:
    """Metadata for a class vtable."""

    # the class type owning this table
    ty: destack._generated.mir.tree.node.LocalNodeId
    # the static global containing this table
    global_: destack._generated.mir.tree.node.LocalNodeId
    # entries in declaration order
    entries: Sequence[VtableEntry]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_vtable(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Vtable:
        """Decode one Vtable."""
        return decode_vtable(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_vtable(self)

    @classmethod
    def from_json(cls, value: Json) -> Vtable:
        """Return one Vtable from one JSON value."""
        return from_json_vtable(value)


def encode_vtable(writer: BinaryWriter, value: Vtable) -> None:
    """Encode one Vtable."""
    destack._generated.mir.tree.node.encode_local_node_id(writer, value.ty)
    destack._generated.mir.tree.node.encode_local_node_id(writer, value.global_)
    writer.write_unsigned(len(value.entries))
    for item_value_entries_0 in value.entries:
        encode_vtable_entry(writer, item_value_entries_0)


def decode_vtable(reader: BinaryReader) -> Vtable:
    """Decode one Vtable."""
    ty = destack._generated.mir.tree.node.decode_local_node_id(reader)
    global_ = destack._generated.mir.tree.node.decode_local_node_id(reader)
    entries = [decode_vtable_entry(reader) for _ in range(reader.read_number())]

    return Vtable(
        ty=ty,
        global_=global_,
        entries=entries,
    )


def to_json_vtable(value: Vtable) -> Json:
    """Return one JSON value for one Vtable."""
    return {
        "ty": destack._generated.mir.tree.node.to_json_local_node_id(value.ty),
        "global": destack._generated.mir.tree.node.to_json_local_node_id(value.global_),
        "entries": [to_json_vtable_entry(item_0) for item_0 in value.entries],
    }


def from_json_vtable(value: Json) -> Vtable:
    """Return one Vtable from one JSON value."""
    object_ = json_object(value)

    return Vtable(
        ty=destack._generated.mir.tree.node.from_json_local_node_id(
            json_field(object_, "ty")
        ),
        global_=destack._generated.mir.tree.node.from_json_local_node_id(
            json_field(object_, "global")
        ),
        entries=[
            from_json_vtable_entry(item_0)
            for item_0 in json_array(json_field(object_, "entries"))
        ],
    )


@dataclass(frozen=True, slots=True)
class VtableEntryTypeDescriptor:
    """Slot containing the runtime type descriptor."""

    kind: typing.Literal["typeDescriptor"] = "typeDescriptor"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_vtable_entry(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_vtable_entry(self)


@dataclass(frozen=True, slots=True)
class VtableEntryDestructor:
    """Slot containing a drop glue function."""

    # the drop glue function when present
    function: destack._generated.mir.tree.node.LocalNodeId | None
    kind: typing.Literal["destructor"] = "destructor"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_vtable_entry(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_vtable_entry(self)


@dataclass(frozen=True, slots=True)
class VtableEntryMethod:
    """Slot containing a method implementation."""

    # the concrete method implementation
    function: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["method"] = "method"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_vtable_entry(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_vtable_entry(self)


"""Entry in a class vtable."""
VtableEntry: typing.TypeAlias = (
    VtableEntryTypeDescriptor | VtableEntryDestructor | VtableEntryMethod
)


def encode_vtable_entry(writer: BinaryWriter, value: VtableEntry) -> None:
    """Encode one VtableEntry."""
    if value.kind == "typeDescriptor":
        writer.write_unsigned(0)
    elif value.kind == "destructor":
        writer.write_unsigned(1)
        if value.function is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.mir.tree.node.encode_local_node_id(
                writer, value.function
            )
    elif value.kind == "method":
        writer.write_unsigned(2)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.function)
    else:
        raise SerdeError("unknown enum variant")


def decode_vtable_entry(reader: BinaryReader) -> VtableEntry:
    """Decode one VtableEntry."""
    variant = reader.read_number()

    if variant == 0:
        return VtableEntryTypeDescriptor()
    elif variant == 1:
        function = reader.read_option(
            lambda: destack._generated.mir.tree.node.decode_local_node_id(reader)
        )

        return VtableEntryDestructor(
            function=function,
        )
    elif variant == 2:
        function = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return VtableEntryMethod(
            function=function,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_vtable_entry(value: VtableEntry) -> Json:
    """Return one JSON value for one VtableEntry."""
    if value.kind == "typeDescriptor":
        return {
            "kind": "typeDescriptor",
        }
    elif value.kind == "destructor":
        return {
            "kind": "destructor",
            **(
                {}
                if value.function is None
                else {
                    "function": destack._generated.mir.tree.node.to_json_local_node_id(
                        value.function
                    )
                }
            ),
        }
    elif value.kind == "method":
        return {
            "kind": "method",
            "function": destack._generated.mir.tree.node.to_json_local_node_id(
                value.function
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_vtable_entry(value: Json) -> VtableEntry:
    """Return one VtableEntry from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "typeDescriptor":
        return VtableEntryTypeDescriptor()
    elif kind == "destructor":
        return VtableEntryDestructor(
            function=json_optional(
                object_,
                "function",
                lambda value: destack._generated.mir.tree.node.from_json_local_node_id(
                    value
                ),
            ),
        )
    elif kind == "method":
        return VtableEntryMethod(
            function=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "function")
            ),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


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
    destack._generated.mir.tree.node.encode_local_node_id(writer, value.global_)
    writer.write_unsigned(len(value.entries))
    for item_value_entries_0 in value.entries:
        encode_dynamic_entry(writer, item_value_entries_0)


def decode_dynamic_table(reader: BinaryReader) -> DynamicTable:
    """Decode one DynamicTable."""
    concrete = destack._generated.mir.tree.node.decode_local_node_id(reader)
    constraint = destack._generated.mir.tree.node.decode_local_node_id(reader)
    global_ = destack._generated.mir.tree.node.decode_local_node_id(reader)
    entries = [decode_dynamic_entry(reader) for _ in range(reader.read_number())]

    return DynamicTable(
        concrete=concrete,
        constraint=constraint,
        global_=global_,
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
        "global": destack._generated.mir.tree.node.to_json_local_node_id(value.global_),
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
        global_=destack._generated.mir.tree.node.from_json_local_node_id(
            json_field(object_, "global")
        ),
        entries=[
            from_json_dynamic_entry(item_0)
            for item_0 in json_array(json_field(object_, "entries"))
        ],
    )


@dataclass(frozen=True, slots=True)
class DynamicEntryField:
    """Slot containing a field offset."""

    # the field offset in bytes
    offset: int
    kind: typing.Literal["field"] = "field"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_dynamic_entry(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_dynamic_entry(self)


@dataclass(frozen=True, slots=True)
class DynamicEntryGetter:
    """Slot containing a concrete getter implementation."""

    # the concrete getter implementation
    function: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["getter"] = "getter"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_dynamic_entry(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_dynamic_entry(self)


@dataclass(frozen=True, slots=True)
class DynamicEntrySetter:
    """Slot containing a concrete setter implementation."""

    # the concrete setter implementation
    function: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["setter"] = "setter"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_dynamic_entry(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_dynamic_entry(self)


@dataclass(frozen=True, slots=True)
class DynamicEntryMethod:
    """Slot containing a concrete method implementation."""

    # the concrete method implementation
    function: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["method"] = "method"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_dynamic_entry(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_dynamic_entry(self)


@dataclass(frozen=True, slots=True)
class DynamicEntryCall:
    """Slot containing a concrete call implementation."""

    # the concrete call implementation
    function: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["call"] = "call"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_dynamic_entry(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_dynamic_entry(self)


"""Entry in a dynamic dispatch table."""
DynamicEntry: typing.TypeAlias = (
    DynamicEntryField
    | DynamicEntryGetter
    | DynamicEntrySetter
    | DynamicEntryMethod
    | DynamicEntryCall
)


def encode_dynamic_entry(writer: BinaryWriter, value: DynamicEntry) -> None:
    """Encode one DynamicEntry."""
    if value.kind == "field":
        writer.write_unsigned(0)
        writer.write_unsigned(value.offset)
    elif value.kind == "getter":
        writer.write_unsigned(1)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.function)
    elif value.kind == "setter":
        writer.write_unsigned(2)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.function)
    elif value.kind == "method":
        writer.write_unsigned(3)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.function)
    elif value.kind == "call":
        writer.write_unsigned(4)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.function)
    else:
        raise SerdeError("unknown enum variant")


def decode_dynamic_entry(reader: BinaryReader) -> DynamicEntry:
    """Decode one DynamicEntry."""
    variant = reader.read_number()

    if variant == 0:
        offset = reader.read_number()

        return DynamicEntryField(
            offset=offset,
        )
    elif variant == 1:
        function = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return DynamicEntryGetter(
            function=function,
        )
    elif variant == 2:
        function = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return DynamicEntrySetter(
            function=function,
        )
    elif variant == 3:
        function = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return DynamicEntryMethod(
            function=function,
        )
    elif variant == 4:
        function = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return DynamicEntryCall(
            function=function,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_dynamic_entry(value: DynamicEntry) -> Json:
    """Return one JSON value for one DynamicEntry."""
    if value.kind == "field":
        return {
            "kind": "field",
            "offset": value.offset,
        }
    elif value.kind == "getter":
        return {
            "kind": "getter",
            "function": destack._generated.mir.tree.node.to_json_local_node_id(
                value.function
            ),
        }
    elif value.kind == "setter":
        return {
            "kind": "setter",
            "function": destack._generated.mir.tree.node.to_json_local_node_id(
                value.function
            ),
        }
    elif value.kind == "method":
        return {
            "kind": "method",
            "function": destack._generated.mir.tree.node.to_json_local_node_id(
                value.function
            ),
        }
    elif value.kind == "call":
        return {
            "kind": "call",
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

    if kind == "field":
        return DynamicEntryField(
            offset=json_int(json_field(object_, "offset")),
        )
    elif kind == "getter":
        return DynamicEntryGetter(
            function=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "function")
            ),
        )
    elif kind == "setter":
        return DynamicEntrySetter(
            function=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "function")
            ),
        )
    elif kind == "method":
        return DynamicEntryMethod(
            function=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "function")
            ),
        )
    elif kind == "call":
        return DynamicEntryCall(
            function=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "function")
            ),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class DynamicShape:
    """Slot layout for one dynamic constraint."""

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
class DynamicSlotGetter:
    """Getter slot."""

    # the getter name
    name: destack._generated.core.string.StringId
    # the getter signature
    signature: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["getter"] = "getter"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_dynamic_slot(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_dynamic_slot(self)


@dataclass(frozen=True, slots=True)
class DynamicSlotSetter:
    """Setter slot."""

    # the setter name
    name: destack._generated.core.string.StringId
    # the setter signature
    signature: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["setter"] = "setter"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_dynamic_slot(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_dynamic_slot(self)


@dataclass(frozen=True, slots=True)
class DynamicSlotMethod:
    """Method slot."""

    # the method name
    name: destack._generated.core.string.StringId
    # the method signature
    signature: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["method"] = "method"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_dynamic_slot(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_dynamic_slot(self)


@dataclass(frozen=True, slots=True)
class DynamicSlotCall:
    """Call signature slot."""

    # the call signature
    signature: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["call"] = "call"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_dynamic_slot(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_dynamic_slot(self)


"""Slot descriptor for a dynamic layout."""
DynamicSlot: typing.TypeAlias = (
    DynamicSlotField
    | DynamicSlotGetter
    | DynamicSlotSetter
    | DynamicSlotMethod
    | DynamicSlotCall
)


def encode_dynamic_slot(writer: BinaryWriter, value: DynamicSlot) -> None:
    """Encode one DynamicSlot."""
    if value.kind == "field":
        writer.write_unsigned(0)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.field)
        destack._generated.core.string.encode_string_id(writer, value.name)
    elif value.kind == "getter":
        writer.write_unsigned(1)
        destack._generated.core.string.encode_string_id(writer, value.name)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.signature)
    elif value.kind == "setter":
        writer.write_unsigned(2)
        destack._generated.core.string.encode_string_id(writer, value.name)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.signature)
    elif value.kind == "method":
        writer.write_unsigned(3)
        destack._generated.core.string.encode_string_id(writer, value.name)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.signature)
    elif value.kind == "call":
        writer.write_unsigned(4)
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
        name = destack._generated.core.string.decode_string_id(reader)
        signature = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return DynamicSlotGetter(
            name=name,
            signature=signature,
        )
    elif variant == 2:
        name = destack._generated.core.string.decode_string_id(reader)
        signature = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return DynamicSlotSetter(
            name=name,
            signature=signature,
        )
    elif variant == 3:
        name = destack._generated.core.string.decode_string_id(reader)
        signature = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return DynamicSlotMethod(
            name=name,
            signature=signature,
        )
    elif variant == 4:
        signature = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return DynamicSlotCall(
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
    elif value.kind == "getter":
        return {
            "kind": "getter",
            "name": destack._generated.core.string.to_json_string_id(value.name),
            "signature": destack._generated.mir.tree.node.to_json_local_node_id(
                value.signature
            ),
        }
    elif value.kind == "setter":
        return {
            "kind": "setter",
            "name": destack._generated.core.string.to_json_string_id(value.name),
            "signature": destack._generated.mir.tree.node.to_json_local_node_id(
                value.signature
            ),
        }
    elif value.kind == "method":
        return {
            "kind": "method",
            "name": destack._generated.core.string.to_json_string_id(value.name),
            "signature": destack._generated.mir.tree.node.to_json_local_node_id(
                value.signature
            ),
        }
    elif value.kind == "call":
        return {
            "kind": "call",
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
    elif kind == "getter":
        return DynamicSlotGetter(
            name=destack._generated.core.string.from_json_string_id(
                json_field(object_, "name")
            ),
            signature=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "signature")
            ),
        )
    elif kind == "setter":
        return DynamicSlotSetter(
            name=destack._generated.core.string.from_json_string_id(
                json_field(object_, "name")
            ),
            signature=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "signature")
            ),
        )
    elif kind == "method":
        return DynamicSlotMethod(
            name=destack._generated.core.string.from_json_string_id(
                json_field(object_, "name")
            ),
            signature=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "signature")
            ),
        )
    elif kind == "call":
        return DynamicSlotCall(
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
