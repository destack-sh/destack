# generated client target, do not edit

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

import destack._generated.core.string
import destack._generated.mir.table.trace
import destack._generated.mir.tree.node
import destack._generated.mir.tree.type


@dataclass(frozen=True, slots=True)
class LayoutTable:
    """Canonical layout table for one MIR module."""

    # layout entries indexed by LayoutId
    entries: Sequence[Layout]
    # layout ids keyed by type id
    types: Mapping[destack._generated.mir.tree.node.LocalNodeId, LayoutId]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_layout_table(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> LayoutTable:
        """Decode one LayoutTable."""
        return decode_layout_table(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_layout_table(self)

    @classmethod
    def from_json(cls, value: Json) -> LayoutTable:
        """Return one LayoutTable from one JSON value."""
        return from_json_layout_table(value)


def encode_layout_table(writer: BinaryWriter, value: LayoutTable) -> None:
    """Encode one LayoutTable."""
    writer.write_unsigned(len(value.entries))
    for item_value_entries_0 in value.entries:
        encode_layout(writer, item_value_entries_0)
    entries_value_types_0 = []
    for key_value_types_0, item_value_types_0 in value.types.items():

        def write_key_value_types_0(writer: BinaryWriter) -> None:
            destack._generated.mir.tree.node.encode_local_node_id(
                writer, key_value_types_0
            )

        key_bytes = nested_bytes(write_key_value_types_0)
        entries_value_types_0.append((key_value_types_0, item_value_types_0, key_bytes))
    entries_value_types_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_types_0))
    for entry_value_types_0 in entries_value_types_0:
        destack._generated.mir.tree.node.encode_local_node_id(
            writer, entry_value_types_0[0]
        )
        encode_layout_id(writer, entry_value_types_0[1])


def decode_layout_table(reader: BinaryReader) -> LayoutTable:
    """Decode one LayoutTable."""
    entries = [decode_layout(reader) for _ in range(reader.read_number())]
    types = {
        destack._generated.mir.tree.node.decode_local_node_id(reader): decode_layout_id(
            reader
        )
        for _ in range(reader.read_number())
    }

    return LayoutTable(
        entries=entries,
        types=types,
    )


def to_json_layout_table(value: LayoutTable) -> Json:
    """Return one JSON value for one LayoutTable."""
    return {
        "entries": [to_json_layout(item_0) for item_0 in value.entries],
        "types": [
            [
                destack._generated.mir.tree.node.to_json_local_node_id(key_0),
                to_json_layout_id(item_0),
            ]
            for key_0, item_0 in value.types.items()
        ],
    }


def from_json_layout_table(value: Json) -> LayoutTable:
    """Return one LayoutTable from one JSON value."""
    object_ = json_object(value)

    return LayoutTable(
        entries=[
            from_json_layout(item_0)
            for item_0 in json_array(json_field(object_, "entries"))
        ],
        types={
            destack._generated.mir.tree.node.from_json_local_node_id(
                key_0
            ): from_json_layout_id(item_0)
            for key_0, item_0 in json_array(json_field(object_, "types"))
        },
    )


@dataclass(frozen=True, slots=True)
class Layout:
    """Concrete memory layout for an aggregate type."""

    # the layout shape
    shape: LayoutShape
    # total size in bytes, including trailing padding
    size: int
    # alignment requirement in bytes
    alignment: int
    # managed-reference trace map for this layout
    trace_map: destack._generated.mir.table.trace.TraceMap

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_layout(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Layout:
        """Decode one Layout."""
        return decode_layout(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_layout(self)

    @classmethod
    def from_json(cls, value: Json) -> Layout:
        """Return one Layout from one JSON value."""
        return from_json_layout(value)


def encode_layout(writer: BinaryWriter, value: Layout) -> None:
    """Encode one Layout."""
    encode_layout_shape(writer, value.shape)
    writer.write_unsigned(value.size)
    writer.write_unsigned(value.alignment)
    destack._generated.mir.table.trace.encode_trace_map(writer, value.trace_map)


def decode_layout(reader: BinaryReader) -> Layout:
    """Decode one Layout."""
    shape = decode_layout_shape(reader)
    size = reader.read_number()
    alignment = reader.read_number()
    trace_map = destack._generated.mir.table.trace.decode_trace_map(reader)

    return Layout(
        shape=shape,
        size=size,
        alignment=alignment,
        trace_map=trace_map,
    )


def to_json_layout(value: Layout) -> Json:
    """Return one JSON value for one Layout."""
    return {
        "shape": to_json_layout_shape(value.shape),
        "size": value.size,
        "alignment": value.alignment,
        "traceMap": destack._generated.mir.table.trace.to_json_trace_map(
            value.trace_map
        ),
    }


def from_json_layout(value: Json) -> Layout:
    """Return one Layout from one JSON value."""
    object_ = json_object(value)

    return Layout(
        shape=from_json_layout_shape(json_field(object_, "shape")),
        size=json_int(json_field(object_, "size")),
        alignment=json_int(json_field(object_, "alignment")),
        trace_map=destack._generated.mir.table.trace.from_json_trace_map(
            json_field(object_, "traceMap")
        ),
    )


@dataclass(frozen=True, slots=True)
class LayoutShapeNone:
    """No runtime storage."""

    kind: typing.Literal["none"] = "none"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_layout_shape(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_layout_shape(self)


@dataclass(frozen=True, slots=True)
class LayoutShapeScalar:
    """Builtin scalar storage."""

    kind: typing.Literal["scalar"] = "scalar"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_layout_shape(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_layout_shape(self)


@dataclass(frozen=True, slots=True)
class LayoutShapeStruct:
    """Struct storage."""

    struct: StructLayout
    kind: typing.Literal["struct"] = "struct"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_layout_shape(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_layout_shape(self)


@dataclass(frozen=True, slots=True)
class LayoutShapeTuple:
    """Tuple storage."""

    tuple: TupleLayout
    kind: typing.Literal["tuple"] = "tuple"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_layout_shape(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_layout_shape(self)


@dataclass(frozen=True, slots=True)
class LayoutShapeSlice:
    """Slice header storage."""

    kind: typing.Literal["slice"] = "slice"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_layout_shape(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_layout_shape(self)


@dataclass(frozen=True, slots=True)
class LayoutShapeArray:
    """Fixed array storage."""

    array: ElementLayout
    kind: typing.Literal["array"] = "array"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_layout_shape(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_layout_shape(self)


@dataclass(frozen=True, slots=True)
class LayoutShapeVector:
    """Vector value storage."""

    vector: ElementLayout
    kind: typing.Literal["vector"] = "vector"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_layout_shape(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_layout_shape(self)


@dataclass(frozen=True, slots=True)
class LayoutShapeTensor:
    """Tensor handle storage."""

    tensor: TensorLayout
    kind: typing.Literal["tensor"] = "tensor"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_layout_shape(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_layout_shape(self)


@dataclass(frozen=True, slots=True)
class LayoutShapeTensorView:
    """Tensor view descriptor storage."""

    tensor_view: TensorViewLayout
    kind: typing.Literal["tensorView"] = "tensorView"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_layout_shape(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_layout_shape(self)


@dataclass(frozen=True, slots=True)
class LayoutShapeVariant:
    """Variant value storage."""

    variant: VariantLayout
    kind: typing.Literal["variant"] = "variant"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_layout_shape(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_layout_shape(self)


@dataclass(frozen=True, slots=True)
class LayoutShapeObject:
    """Object storage with a dispatch table header."""

    object: ObjectLayout
    kind: typing.Literal["object"] = "object"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_layout_shape(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_layout_shape(self)


@dataclass(frozen=True, slots=True)
class LayoutShapeDynamic:
    """Runtime dynamic value layout."""

    kind: typing.Literal["dynamic"] = "dynamic"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_layout_shape(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_layout_shape(self)


@dataclass(frozen=True, slots=True)
class LayoutShapeFunction:
    """Runtime function value storage."""

    kind: typing.Literal["function"] = "function"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_layout_shape(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_layout_shape(self)


@dataclass(frozen=True, slots=True)
class LayoutShapeNewtype:
    """Transparent nominal storage."""

    newtype: NewtypeLayout
    kind: typing.Literal["newtype"] = "newtype"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_layout_shape(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_layout_shape(self)


"""Concrete memory layout shape."""
LayoutShape: typing.TypeAlias = (
    LayoutShapeNone
    | LayoutShapeScalar
    | LayoutShapeStruct
    | LayoutShapeTuple
    | LayoutShapeSlice
    | LayoutShapeArray
    | LayoutShapeVector
    | LayoutShapeTensor
    | LayoutShapeTensorView
    | LayoutShapeVariant
    | LayoutShapeObject
    | LayoutShapeDynamic
    | LayoutShapeFunction
    | LayoutShapeNewtype
)


def encode_layout_shape(writer: BinaryWriter, value: LayoutShape) -> None:
    """Encode one LayoutShape."""
    if value.kind == "none":
        writer.write_unsigned(0)
    elif value.kind == "scalar":
        writer.write_unsigned(1)
    elif value.kind == "struct":
        writer.write_unsigned(2)
        encode_struct_layout(writer, value.struct)
    elif value.kind == "tuple":
        writer.write_unsigned(3)
        encode_tuple_layout(writer, value.tuple)
    elif value.kind == "slice":
        writer.write_unsigned(4)
    elif value.kind == "array":
        writer.write_unsigned(5)
        encode_element_layout(writer, value.array)
    elif value.kind == "vector":
        writer.write_unsigned(6)
        encode_element_layout(writer, value.vector)
    elif value.kind == "tensor":
        writer.write_unsigned(7)
        encode_tensor_layout(writer, value.tensor)
    elif value.kind == "tensorView":
        writer.write_unsigned(8)
        encode_tensor_view_layout(writer, value.tensor_view)
    elif value.kind == "variant":
        writer.write_unsigned(9)
        encode_variant_layout(writer, value.variant)
    elif value.kind == "object":
        writer.write_unsigned(10)
        encode_object_layout(writer, value.object)
    elif value.kind == "dynamic":
        writer.write_unsigned(11)
    elif value.kind == "function":
        writer.write_unsigned(12)
    elif value.kind == "newtype":
        writer.write_unsigned(13)
        encode_newtype_layout(writer, value.newtype)
    else:
        raise SerdeError("unknown enum variant")


def decode_layout_shape(reader: BinaryReader) -> LayoutShape:
    """Decode one LayoutShape."""
    variant = reader.read_number()

    if variant == 0:
        return LayoutShapeNone()
    elif variant == 1:
        return LayoutShapeScalar()
    elif variant == 2:
        struct = decode_struct_layout(reader)

        return LayoutShapeStruct(struct=struct)
    elif variant == 3:
        tuple = decode_tuple_layout(reader)

        return LayoutShapeTuple(tuple=tuple)
    elif variant == 4:
        return LayoutShapeSlice()
    elif variant == 5:
        array = decode_element_layout(reader)

        return LayoutShapeArray(array=array)
    elif variant == 6:
        vector = decode_element_layout(reader)

        return LayoutShapeVector(vector=vector)
    elif variant == 7:
        tensor = decode_tensor_layout(reader)

        return LayoutShapeTensor(tensor=tensor)
    elif variant == 8:
        tensor_view = decode_tensor_view_layout(reader)

        return LayoutShapeTensorView(tensor_view=tensor_view)
    elif variant == 9:
        variant = decode_variant_layout(reader)

        return LayoutShapeVariant(variant=variant)
    elif variant == 10:
        object = decode_object_layout(reader)

        return LayoutShapeObject(object=object)
    elif variant == 11:
        return LayoutShapeDynamic()
    elif variant == 12:
        return LayoutShapeFunction()
    elif variant == 13:
        newtype = decode_newtype_layout(reader)

        return LayoutShapeNewtype(newtype=newtype)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_layout_shape(value: LayoutShape) -> Json:
    """Return one JSON value for one LayoutShape."""
    if value.kind == "none":
        return {
            "kind": "none",
        }
    elif value.kind == "scalar":
        return {
            "kind": "scalar",
        }
    elif value.kind == "struct":
        return {
            "kind": "struct",
            "struct": to_json_struct_layout(value.struct),
        }
    elif value.kind == "tuple":
        return {
            "kind": "tuple",
            "tuple": to_json_tuple_layout(value.tuple),
        }
    elif value.kind == "slice":
        return {
            "kind": "slice",
        }
    elif value.kind == "array":
        return {
            "kind": "array",
            "array": to_json_element_layout(value.array),
        }
    elif value.kind == "vector":
        return {
            "kind": "vector",
            "vector": to_json_element_layout(value.vector),
        }
    elif value.kind == "tensor":
        return {
            "kind": "tensor",
            "tensor": to_json_tensor_layout(value.tensor),
        }
    elif value.kind == "tensorView":
        return {
            "kind": "tensorView",
            "tensor_view": to_json_tensor_view_layout(value.tensor_view),
        }
    elif value.kind == "variant":
        return {
            "kind": "variant",
            "variant": to_json_variant_layout(value.variant),
        }
    elif value.kind == "object":
        return {
            "kind": "object",
            "object": to_json_object_layout(value.object),
        }
    elif value.kind == "dynamic":
        return {
            "kind": "dynamic",
        }
    elif value.kind == "function":
        return {
            "kind": "function",
        }
    elif value.kind == "newtype":
        return {
            "kind": "newtype",
            "newtype": to_json_newtype_layout(value.newtype),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_layout_shape(value: Json) -> LayoutShape:
    """Return one LayoutShape from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "none":
        return LayoutShapeNone()
    elif kind == "scalar":
        return LayoutShapeScalar()
    elif kind == "struct":
        return LayoutShapeStruct(
            struct=from_json_struct_layout(json_field(object_, "struct"))
        )
    elif kind == "tuple":
        return LayoutShapeTuple(
            tuple=from_json_tuple_layout(json_field(object_, "tuple"))
        )
    elif kind == "slice":
        return LayoutShapeSlice()
    elif kind == "array":
        return LayoutShapeArray(
            array=from_json_element_layout(json_field(object_, "array"))
        )
    elif kind == "vector":
        return LayoutShapeVector(
            vector=from_json_element_layout(json_field(object_, "vector"))
        )
    elif kind == "tensor":
        return LayoutShapeTensor(
            tensor=from_json_tensor_layout(json_field(object_, "tensor"))
        )
    elif kind == "tensorView":
        return LayoutShapeTensorView(
            tensor_view=from_json_tensor_view_layout(json_field(object_, "tensor_view"))
        )
    elif kind == "variant":
        return LayoutShapeVariant(
            variant=from_json_variant_layout(json_field(object_, "variant"))
        )
    elif kind == "object":
        return LayoutShapeObject(
            object=from_json_object_layout(json_field(object_, "object"))
        )
    elif kind == "dynamic":
        return LayoutShapeDynamic()
    elif kind == "function":
        return LayoutShapeFunction()
    elif kind == "newtype":
        return LayoutShapeNewtype(
            newtype=from_json_newtype_layout(json_field(object_, "newtype"))
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class StructLayout:
    """Concrete layout for a struct."""

    # the fields in layout order
    fields: Sequence[LayoutField]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_struct_layout(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> StructLayout:
        """Decode one StructLayout."""
        return decode_struct_layout(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_struct_layout(self)

    @classmethod
    def from_json(cls, value: Json) -> StructLayout:
        """Return one StructLayout from one JSON value."""
        return from_json_struct_layout(value)


def encode_struct_layout(writer: BinaryWriter, value: StructLayout) -> None:
    """Encode one StructLayout."""
    writer.write_unsigned(len(value.fields))
    for item_value_fields_0 in value.fields:
        encode_layout_field(writer, item_value_fields_0)


def decode_struct_layout(reader: BinaryReader) -> StructLayout:
    """Decode one StructLayout."""
    fields = [decode_layout_field(reader) for _ in range(reader.read_number())]

    return StructLayout(
        fields=fields,
    )


def to_json_struct_layout(value: StructLayout) -> Json:
    """Return one JSON value for one StructLayout."""
    return {
        "fields": [to_json_layout_field(item_0) for item_0 in value.fields],
    }


def from_json_struct_layout(value: Json) -> StructLayout:
    """Return one StructLayout from one JSON value."""
    object_ = json_object(value)

    return StructLayout(
        fields=[
            from_json_layout_field(item_0)
            for item_0 in json_array(json_field(object_, "fields"))
        ],
    )


@dataclass(frozen=True, slots=True)
class LayoutField:
    """Memory layout for a single field."""

    # field name for lookup and debugging
    name: destack._generated.core.string.StringId | None
    # MIR type of the field
    ty: destack._generated.mir.tree.node.LocalNodeId
    # byte offset from the start of the aggregate
    offset: int
    # size of the field in bytes
    size: int
    # alignment requirement of the field in bytes
    alignment: int
    # original source index for stable mapping
    source_index: int | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_layout_field(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> LayoutField:
        """Decode one LayoutField."""
        return decode_layout_field(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_layout_field(self)

    @classmethod
    def from_json(cls, value: Json) -> LayoutField:
        """Return one LayoutField from one JSON value."""
        return from_json_layout_field(value)


def encode_layout_field(writer: BinaryWriter, value: LayoutField) -> None:
    """Encode one LayoutField."""
    if value.name is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.core.string.encode_string_id(writer, value.name)
    destack._generated.mir.tree.node.encode_local_node_id(writer, value.ty)
    writer.write_unsigned(value.offset)
    writer.write_unsigned(value.size)
    writer.write_unsigned(value.alignment)
    if value.source_index is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(value.source_index)


def decode_layout_field(reader: BinaryReader) -> LayoutField:
    """Decode one LayoutField."""
    name = reader.read_option(
        lambda: destack._generated.core.string.decode_string_id(reader)
    )
    ty = destack._generated.mir.tree.node.decode_local_node_id(reader)
    offset = reader.read_number()
    size = reader.read_number()
    alignment = reader.read_number()
    source_index = reader.read_option(lambda: reader.read_number())

    return LayoutField(
        name=name,
        ty=ty,
        offset=offset,
        size=size,
        alignment=alignment,
        source_index=source_index,
    )


def to_json_layout_field(value: LayoutField) -> Json:
    """Return one JSON value for one LayoutField."""
    return {
        **(
            {}
            if value.name is None
            else {"name": destack._generated.core.string.to_json_string_id(value.name)}
        ),
        "ty": destack._generated.mir.tree.node.to_json_local_node_id(value.ty),
        "offset": value.offset,
        "size": value.size,
        "alignment": value.alignment,
        **({} if value.source_index is None else {"sourceIndex": value.source_index}),
    }


def from_json_layout_field(value: Json) -> LayoutField:
    """Return one LayoutField from one JSON value."""
    object_ = json_object(value)

    return LayoutField(
        name=json_optional(
            object_,
            "name",
            lambda value: destack._generated.core.string.from_json_string_id(value),
        ),
        ty=destack._generated.mir.tree.node.from_json_local_node_id(
            json_field(object_, "ty")
        ),
        offset=json_int(json_field(object_, "offset")),
        size=json_int(json_field(object_, "size")),
        alignment=json_int(json_field(object_, "alignment")),
        source_index=json_optional(
            object_, "sourceIndex", lambda value: json_int(value)
        ),
    )


@dataclass(frozen=True, slots=True)
class TupleLayout:
    """Concrete layout for a tuple."""

    # the tuple elements in layout order
    elements: Sequence[LayoutField]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tuple_layout(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TupleLayout:
        """Decode one TupleLayout."""
        return decode_tuple_layout(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tuple_layout(self)

    @classmethod
    def from_json(cls, value: Json) -> TupleLayout:
        """Return one TupleLayout from one JSON value."""
        return from_json_tuple_layout(value)


def encode_tuple_layout(writer: BinaryWriter, value: TupleLayout) -> None:
    """Encode one TupleLayout."""
    writer.write_unsigned(len(value.elements))
    for item_value_elements_0 in value.elements:
        encode_layout_field(writer, item_value_elements_0)


def decode_tuple_layout(reader: BinaryReader) -> TupleLayout:
    """Decode one TupleLayout."""
    elements = [decode_layout_field(reader) for _ in range(reader.read_number())]

    return TupleLayout(
        elements=elements,
    )


def to_json_tuple_layout(value: TupleLayout) -> Json:
    """Return one JSON value for one TupleLayout."""
    return {
        "elements": [to_json_layout_field(item_0) for item_0 in value.elements],
    }


def from_json_tuple_layout(value: Json) -> TupleLayout:
    """Return one TupleLayout from one JSON value."""
    object_ = json_object(value)

    return TupleLayout(
        elements=[
            from_json_layout_field(item_0)
            for item_0 in json_array(json_field(object_, "elements"))
        ],
    )


@dataclass(frozen=True, slots=True)
class ElementLayout:
    """Layout for inline indexed element storage."""

    # the stored element type
    element: destack._generated.mir.tree.node.LocalNodeId
    # the byte stride between elements
    stride: int
    # the fixed element count
    count: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_element_layout(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ElementLayout:
        """Decode one ElementLayout."""
        return decode_element_layout(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_element_layout(self)

    @classmethod
    def from_json(cls, value: Json) -> ElementLayout:
        """Return one ElementLayout from one JSON value."""
        return from_json_element_layout(value)


def encode_element_layout(writer: BinaryWriter, value: ElementLayout) -> None:
    """Encode one ElementLayout."""
    destack._generated.mir.tree.node.encode_local_node_id(writer, value.element)
    writer.write_unsigned(value.stride)
    writer.write_unsigned(value.count)


def decode_element_layout(reader: BinaryReader) -> ElementLayout:
    """Decode one ElementLayout."""
    element = destack._generated.mir.tree.node.decode_local_node_id(reader)
    stride = reader.read_number()
    count = reader.read_number()

    return ElementLayout(
        element=element,
        stride=stride,
        count=count,
    )


def to_json_element_layout(value: ElementLayout) -> Json:
    """Return one JSON value for one ElementLayout."""
    return {
        "element": destack._generated.mir.tree.node.to_json_local_node_id(
            value.element
        ),
        "stride": value.stride,
        "count": value.count,
    }


def from_json_element_layout(value: Json) -> ElementLayout:
    """Return one ElementLayout from one JSON value."""
    object_ = json_object(value)

    return ElementLayout(
        element=destack._generated.mir.tree.node.from_json_local_node_id(
            json_field(object_, "element")
        ),
        stride=json_int(json_field(object_, "stride")),
        count=json_int(json_field(object_, "count")),
    )


@dataclass(frozen=True, slots=True)
class TensorLayout:
    """Concrete layout for a tensor handle."""

    # the tensor element type
    element: destack._generated.mir.tree.node.LocalNodeId
    # the tensor storage format
    format: destack._generated.mir.tree.type.TensorFormat
    # the tensor placement
    sharding: destack._generated.mir.tree.type.TensorSharding
    # the tensor rank
    rank: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_layout(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorLayout:
        """Decode one TensorLayout."""
        return decode_tensor_layout(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_layout(self)

    @classmethod
    def from_json(cls, value: Json) -> TensorLayout:
        """Return one TensorLayout from one JSON value."""
        return from_json_tensor_layout(value)


def encode_tensor_layout(writer: BinaryWriter, value: TensorLayout) -> None:
    """Encode one TensorLayout."""
    destack._generated.mir.tree.node.encode_local_node_id(writer, value.element)
    destack._generated.mir.tree.type.encode_tensor_format(writer, value.format)
    destack._generated.mir.tree.type.encode_tensor_sharding(writer, value.sharding)
    writer.write_unsigned(value.rank)


def decode_tensor_layout(reader: BinaryReader) -> TensorLayout:
    """Decode one TensorLayout."""
    element = destack._generated.mir.tree.node.decode_local_node_id(reader)
    format = destack._generated.mir.tree.type.decode_tensor_format(reader)
    sharding = destack._generated.mir.tree.type.decode_tensor_sharding(reader)
    rank = reader.read_number()

    return TensorLayout(
        element=element,
        format=format,
        sharding=sharding,
        rank=rank,
    )


def to_json_tensor_layout(value: TensorLayout) -> Json:
    """Return one JSON value for one TensorLayout."""
    return {
        "element": destack._generated.mir.tree.node.to_json_local_node_id(
            value.element
        ),
        "format": destack._generated.mir.tree.type.to_json_tensor_format(value.format),
        "sharding": destack._generated.mir.tree.type.to_json_tensor_sharding(
            value.sharding
        ),
        "rank": value.rank,
    }


def from_json_tensor_layout(value: Json) -> TensorLayout:
    """Return one TensorLayout from one JSON value."""
    object_ = json_object(value)

    return TensorLayout(
        element=destack._generated.mir.tree.node.from_json_local_node_id(
            json_field(object_, "element")
        ),
        format=destack._generated.mir.tree.type.from_json_tensor_format(
            json_field(object_, "format")
        ),
        sharding=destack._generated.mir.tree.type.from_json_tensor_sharding(
            json_field(object_, "sharding")
        ),
        rank=json_int(json_field(object_, "rank")),
    )


@dataclass(frozen=True, slots=True)
class TensorViewLayout:
    """Concrete layout for a tensor view descriptor."""

    # the viewed element type
    element: destack._generated.mir.tree.node.LocalNodeId
    # the tensor view format
    format: destack._generated.mir.tree.type.TensorViewFormat
    # the tensor placement
    sharding: destack._generated.mir.tree.type.TensorSharding
    # the tensor rank
    rank: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_view_layout(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorViewLayout:
        """Decode one TensorViewLayout."""
        return decode_tensor_view_layout(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_view_layout(self)

    @classmethod
    def from_json(cls, value: Json) -> TensorViewLayout:
        """Return one TensorViewLayout from one JSON value."""
        return from_json_tensor_view_layout(value)


def encode_tensor_view_layout(writer: BinaryWriter, value: TensorViewLayout) -> None:
    """Encode one TensorViewLayout."""
    destack._generated.mir.tree.node.encode_local_node_id(writer, value.element)
    destack._generated.mir.tree.type.encode_tensor_view_format(writer, value.format)
    destack._generated.mir.tree.type.encode_tensor_sharding(writer, value.sharding)
    writer.write_unsigned(value.rank)


def decode_tensor_view_layout(reader: BinaryReader) -> TensorViewLayout:
    """Decode one TensorViewLayout."""
    element = destack._generated.mir.tree.node.decode_local_node_id(reader)
    format = destack._generated.mir.tree.type.decode_tensor_view_format(reader)
    sharding = destack._generated.mir.tree.type.decode_tensor_sharding(reader)
    rank = reader.read_number()

    return TensorViewLayout(
        element=element,
        format=format,
        sharding=sharding,
        rank=rank,
    )


def to_json_tensor_view_layout(value: TensorViewLayout) -> Json:
    """Return one JSON value for one TensorViewLayout."""
    return {
        "element": destack._generated.mir.tree.node.to_json_local_node_id(
            value.element
        ),
        "format": destack._generated.mir.tree.type.to_json_tensor_view_format(
            value.format
        ),
        "sharding": destack._generated.mir.tree.type.to_json_tensor_sharding(
            value.sharding
        ),
        "rank": value.rank,
    }


def from_json_tensor_view_layout(value: Json) -> TensorViewLayout:
    """Return one TensorViewLayout from one JSON value."""
    object_ = json_object(value)

    return TensorViewLayout(
        element=destack._generated.mir.tree.node.from_json_local_node_id(
            json_field(object_, "element")
        ),
        format=destack._generated.mir.tree.type.from_json_tensor_view_format(
            json_field(object_, "format")
        ),
        sharding=destack._generated.mir.tree.type.from_json_tensor_sharding(
            json_field(object_, "sharding")
        ),
        rank=json_int(json_field(object_, "rank")),
    )


@dataclass(frozen=True, slots=True)
class VariantLayout:
    """Concrete layout for a variant value."""

    # the tag layout
    tag: VariantTagLayout
    # the variant payload byte offset
    payload_offset: int
    # the variant cases
    variants: Sequence[VariantCaseLayout]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_variant_layout(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> VariantLayout:
        """Decode one VariantLayout."""
        return decode_variant_layout(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_variant_layout(self)

    @classmethod
    def from_json(cls, value: Json) -> VariantLayout:
        """Return one VariantLayout from one JSON value."""
        return from_json_variant_layout(value)


def encode_variant_layout(writer: BinaryWriter, value: VariantLayout) -> None:
    """Encode one VariantLayout."""
    encode_variant_tag_layout(writer, value.tag)
    writer.write_unsigned(value.payload_offset)
    writer.write_unsigned(len(value.variants))
    for item_value_variants_0 in value.variants:
        encode_variant_case_layout(writer, item_value_variants_0)


def decode_variant_layout(reader: BinaryReader) -> VariantLayout:
    """Decode one VariantLayout."""
    tag = decode_variant_tag_layout(reader)
    payload_offset = reader.read_number()
    variants = [decode_variant_case_layout(reader) for _ in range(reader.read_number())]

    return VariantLayout(
        tag=tag,
        payload_offset=payload_offset,
        variants=variants,
    )


def to_json_variant_layout(value: VariantLayout) -> Json:
    """Return one JSON value for one VariantLayout."""
    return {
        "tag": to_json_variant_tag_layout(value.tag),
        "payloadOffset": value.payload_offset,
        "variants": [to_json_variant_case_layout(item_0) for item_0 in value.variants],
    }


def from_json_variant_layout(value: Json) -> VariantLayout:
    """Return one VariantLayout from one JSON value."""
    object_ = json_object(value)

    return VariantLayout(
        tag=from_json_variant_tag_layout(json_field(object_, "tag")),
        payload_offset=json_int(json_field(object_, "payloadOffset")),
        variants=[
            from_json_variant_case_layout(item_0)
            for item_0 in json_array(json_field(object_, "variants"))
        ],
    )


@dataclass(frozen=True, slots=True)
class VariantTagLayout:
    """Concrete layout for a variant tag."""

    # the tag type when it has been materialized
    ty: destack._generated.mir.tree.node.LocalNodeId | None
    # the tag size in bytes
    size: int
    # the tag alignment in bytes
    alignment: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_variant_tag_layout(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> VariantTagLayout:
        """Decode one VariantTagLayout."""
        return decode_variant_tag_layout(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_variant_tag_layout(self)

    @classmethod
    def from_json(cls, value: Json) -> VariantTagLayout:
        """Return one VariantTagLayout from one JSON value."""
        return from_json_variant_tag_layout(value)


def encode_variant_tag_layout(writer: BinaryWriter, value: VariantTagLayout) -> None:
    """Encode one VariantTagLayout."""
    if value.ty is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.ty)
    writer.write_unsigned(value.size)
    writer.write_unsigned(value.alignment)


def decode_variant_tag_layout(reader: BinaryReader) -> VariantTagLayout:
    """Decode one VariantTagLayout."""
    ty = reader.read_option(
        lambda: destack._generated.mir.tree.node.decode_local_node_id(reader)
    )
    size = reader.read_number()
    alignment = reader.read_number()

    return VariantTagLayout(
        ty=ty,
        size=size,
        alignment=alignment,
    )


def to_json_variant_tag_layout(value: VariantTagLayout) -> Json:
    """Return one JSON value for one VariantTagLayout."""
    return {
        **(
            {}
            if value.ty is None
            else {
                "ty": destack._generated.mir.tree.node.to_json_local_node_id(value.ty)
            }
        ),
        "size": value.size,
        "alignment": value.alignment,
    }


def from_json_variant_tag_layout(value: Json) -> VariantTagLayout:
    """Return one VariantTagLayout from one JSON value."""
    object_ = json_object(value)

    return VariantTagLayout(
        ty=json_optional(
            object_,
            "ty",
            lambda value: destack._generated.mir.tree.node.from_json_local_node_id(
                value
            ),
        ),
        size=json_int(json_field(object_, "size")),
        alignment=json_int(json_field(object_, "alignment")),
    )


@dataclass(frozen=True, slots=True)
class VariantCaseLayout:
    """Concrete layout for one variant case."""

    # the logical case type
    ty: destack._generated.mir.tree.node.LocalNodeId
    # the case layout
    layout: LayoutId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_variant_case_layout(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> VariantCaseLayout:
        """Decode one VariantCaseLayout."""
        return decode_variant_case_layout(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_variant_case_layout(self)

    @classmethod
    def from_json(cls, value: Json) -> VariantCaseLayout:
        """Return one VariantCaseLayout from one JSON value."""
        return from_json_variant_case_layout(value)


def encode_variant_case_layout(writer: BinaryWriter, value: VariantCaseLayout) -> None:
    """Encode one VariantCaseLayout."""
    destack._generated.mir.tree.node.encode_local_node_id(writer, value.ty)
    encode_layout_id(writer, value.layout)


def decode_variant_case_layout(reader: BinaryReader) -> VariantCaseLayout:
    """Decode one VariantCaseLayout."""
    ty = destack._generated.mir.tree.node.decode_local_node_id(reader)
    layout = decode_layout_id(reader)

    return VariantCaseLayout(
        ty=ty,
        layout=layout,
    )


def to_json_variant_case_layout(value: VariantCaseLayout) -> Json:
    """Return one JSON value for one VariantCaseLayout."""
    return {
        "ty": destack._generated.mir.tree.node.to_json_local_node_id(value.ty),
        "layout": to_json_layout_id(value.layout),
    }


def from_json_variant_case_layout(value: Json) -> VariantCaseLayout:
    """Return one VariantCaseLayout from one JSON value."""
    object_ = json_object(value)

    return VariantCaseLayout(
        ty=destack._generated.mir.tree.node.from_json_local_node_id(
            json_field(object_, "ty")
        ),
        layout=from_json_layout_id(json_field(object_, "layout")),
    )


"""Opaque identifier for a concrete memory layout."""
LayoutId: typing.TypeAlias = int


def encode_layout_id(writer: BinaryWriter, value: LayoutId) -> None:
    """Encode one LayoutId."""
    writer.write_unsigned(value)


def decode_layout_id(reader: BinaryReader) -> LayoutId:
    """Decode one LayoutId."""
    return reader.read_number()


def to_json_layout_id(value: LayoutId) -> Json:
    """Return one JSON value for one LayoutId."""
    return value


def from_json_layout_id(value: Json) -> LayoutId:
    """Return one LayoutId from one JSON value."""
    return json_int(value)


@dataclass(frozen=True, slots=True)
class ObjectLayout:
    """Concrete layout for an object."""

    # the fields in layout order
    fields: Sequence[LayoutField]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_object_layout(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ObjectLayout:
        """Decode one ObjectLayout."""
        return decode_object_layout(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_object_layout(self)

    @classmethod
    def from_json(cls, value: Json) -> ObjectLayout:
        """Return one ObjectLayout from one JSON value."""
        return from_json_object_layout(value)


def encode_object_layout(writer: BinaryWriter, value: ObjectLayout) -> None:
    """Encode one ObjectLayout."""
    writer.write_unsigned(len(value.fields))
    for item_value_fields_0 in value.fields:
        encode_layout_field(writer, item_value_fields_0)


def decode_object_layout(reader: BinaryReader) -> ObjectLayout:
    """Decode one ObjectLayout."""
    fields = [decode_layout_field(reader) for _ in range(reader.read_number())]

    return ObjectLayout(
        fields=fields,
    )


def to_json_object_layout(value: ObjectLayout) -> Json:
    """Return one JSON value for one ObjectLayout."""
    return {
        "fields": [to_json_layout_field(item_0) for item_0 in value.fields],
    }


def from_json_object_layout(value: Json) -> ObjectLayout:
    """Return one ObjectLayout from one JSON value."""
    object_ = json_object(value)

    return ObjectLayout(
        fields=[
            from_json_layout_field(item_0)
            for item_0 in json_array(json_field(object_, "fields"))
        ],
    )


@dataclass(frozen=True, slots=True)
class NewtypeLayout:
    """Concrete layout for a nominal newtype."""

    # the backing type
    backing_type: destack._generated.mir.tree.node.LocalNodeId
    # the backing type layout
    backing_layout: LayoutId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_newtype_layout(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> NewtypeLayout:
        """Decode one NewtypeLayout."""
        return decode_newtype_layout(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_newtype_layout(self)

    @classmethod
    def from_json(cls, value: Json) -> NewtypeLayout:
        """Return one NewtypeLayout from one JSON value."""
        return from_json_newtype_layout(value)


def encode_newtype_layout(writer: BinaryWriter, value: NewtypeLayout) -> None:
    """Encode one NewtypeLayout."""
    destack._generated.mir.tree.node.encode_local_node_id(writer, value.backing_type)
    encode_layout_id(writer, value.backing_layout)


def decode_newtype_layout(reader: BinaryReader) -> NewtypeLayout:
    """Decode one NewtypeLayout."""
    backing_type = destack._generated.mir.tree.node.decode_local_node_id(reader)
    backing_layout = decode_layout_id(reader)

    return NewtypeLayout(
        backing_type=backing_type,
        backing_layout=backing_layout,
    )


def to_json_newtype_layout(value: NewtypeLayout) -> Json:
    """Return one JSON value for one NewtypeLayout."""
    return {
        "backingType": destack._generated.mir.tree.node.to_json_local_node_id(
            value.backing_type
        ),
        "backingLayout": to_json_layout_id(value.backing_layout),
    }


def from_json_newtype_layout(value: Json) -> NewtypeLayout:
    """Return one NewtypeLayout from one JSON value."""
    object_ = json_object(value)

    return NewtypeLayout(
        backing_type=destack._generated.mir.tree.node.from_json_local_node_id(
            json_field(object_, "backingType")
        ),
        backing_layout=from_json_layout_id(json_field(object_, "backingLayout")),
    )


__all__ = [
    "LayoutTable",
    "encode_layout_table",
    "decode_layout_table",
    "to_json_layout_table",
    "from_json_layout_table",
    "Layout",
    "encode_layout",
    "decode_layout",
    "to_json_layout",
    "from_json_layout",
    "LayoutShape",
    "encode_layout_shape",
    "decode_layout_shape",
    "to_json_layout_shape",
    "from_json_layout_shape",
    "LayoutShapeNone",
    "LayoutShapeScalar",
    "LayoutShapeStruct",
    "LayoutShapeTuple",
    "LayoutShapeSlice",
    "LayoutShapeArray",
    "LayoutShapeVector",
    "LayoutShapeTensor",
    "LayoutShapeTensorView",
    "LayoutShapeVariant",
    "LayoutShapeObject",
    "LayoutShapeDynamic",
    "LayoutShapeFunction",
    "LayoutShapeNewtype",
    "StructLayout",
    "encode_struct_layout",
    "decode_struct_layout",
    "to_json_struct_layout",
    "from_json_struct_layout",
    "LayoutField",
    "encode_layout_field",
    "decode_layout_field",
    "to_json_layout_field",
    "from_json_layout_field",
    "TupleLayout",
    "encode_tuple_layout",
    "decode_tuple_layout",
    "to_json_tuple_layout",
    "from_json_tuple_layout",
    "ElementLayout",
    "encode_element_layout",
    "decode_element_layout",
    "to_json_element_layout",
    "from_json_element_layout",
    "TensorLayout",
    "encode_tensor_layout",
    "decode_tensor_layout",
    "to_json_tensor_layout",
    "from_json_tensor_layout",
    "TensorViewLayout",
    "encode_tensor_view_layout",
    "decode_tensor_view_layout",
    "to_json_tensor_view_layout",
    "from_json_tensor_view_layout",
    "VariantLayout",
    "encode_variant_layout",
    "decode_variant_layout",
    "to_json_variant_layout",
    "from_json_variant_layout",
    "VariantTagLayout",
    "encode_variant_tag_layout",
    "decode_variant_tag_layout",
    "to_json_variant_tag_layout",
    "from_json_variant_tag_layout",
    "VariantCaseLayout",
    "encode_variant_case_layout",
    "decode_variant_case_layout",
    "to_json_variant_case_layout",
    "from_json_variant_case_layout",
    "LayoutId",
    "encode_layout_id",
    "decode_layout_id",
    "to_json_layout_id",
    "from_json_layout_id",
    "ObjectLayout",
    "encode_object_layout",
    "decode_object_layout",
    "to_json_object_layout",
    "from_json_object_layout",
    "NewtypeLayout",
    "encode_newtype_layout",
    "decode_newtype_layout",
    "to_json_newtype_layout",
    "from_json_newtype_layout",
]
