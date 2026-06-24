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

import destack._generated.dir.symbol.key
import destack._generated.dir.type.type
import destack._generated.source.file.model.module


@dataclass(frozen=True, slots=True)
class LayoutSegment:
    """Layouts added by one DIR phase."""

    # the module id of the layout segment
    module_id: destack._generated.source.file.model.module.ModuleId
    # the first layout id owned by this table segment
    first_layout_id: int
    # concrete layouts
    layouts: Sequence[Layout]
    # layout ids keyed by canonical type id
    type_layouts: Mapping[destack._generated.dir.type.type.GlobalTypeId, LocalLayoutId]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_layout_segment(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> LayoutSegment:
        """Decode one LayoutSegment."""
        return decode_layout_segment(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_layout_segment(self)

    @classmethod
    def from_json(cls, value: Json) -> LayoutSegment:
        """Return one LayoutSegment from one JSON value."""
        return from_json_layout_segment(value)


def encode_layout_segment(writer: BinaryWriter, value: LayoutSegment) -> None:
    """Encode one LayoutSegment."""
    destack._generated.source.file.model.module.encode_module_id(
        writer, value.module_id
    )
    writer.write_unsigned(value.first_layout_id)
    writer.write_unsigned(len(value.layouts))
    for item_value_layouts_0 in value.layouts:
        encode_layout(writer, item_value_layouts_0)
    entries_value_type_layouts_0 = []
    for (
        key_value_type_layouts_0,
        item_value_type_layouts_0,
    ) in value.type_layouts.items():

        def write_key_value_type_layouts_0(writer: BinaryWriter) -> None:
            destack._generated.dir.type.type.encode_global_type_id(
                writer, key_value_type_layouts_0
            )

        key_bytes = nested_bytes(write_key_value_type_layouts_0)
        entries_value_type_layouts_0.append(
            (key_value_type_layouts_0, item_value_type_layouts_0, key_bytes)
        )
    entries_value_type_layouts_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_type_layouts_0))
    for entry_value_type_layouts_0 in entries_value_type_layouts_0:
        destack._generated.dir.type.type.encode_global_type_id(
            writer, entry_value_type_layouts_0[0]
        )
        encode_local_layout_id(writer, entry_value_type_layouts_0[1])


def decode_layout_segment(reader: BinaryReader) -> LayoutSegment:
    """Decode one LayoutSegment."""
    module_id = destack._generated.source.file.model.module.decode_module_id(reader)
    first_layout_id = reader.read_number()
    layouts = [decode_layout(reader) for _ in range(reader.read_number())]
    type_layouts = {
        destack._generated.dir.type.type.decode_global_type_id(
            reader
        ): decode_local_layout_id(reader)
        for _ in range(reader.read_number())
    }

    return LayoutSegment(
        module_id=module_id,
        first_layout_id=first_layout_id,
        layouts=layouts,
        type_layouts=type_layouts,
    )


def to_json_layout_segment(value: LayoutSegment) -> Json:
    """Return one JSON value for one LayoutSegment."""
    return {
        "moduleId": destack._generated.source.file.model.module.to_json_module_id(
            value.module_id
        ),
        "firstLayoutId": value.first_layout_id,
        "layouts": [to_json_layout(item_0) for item_0 in value.layouts],
        "typeLayouts": [
            [
                destack._generated.dir.type.type.to_json_global_type_id(key_0),
                to_json_local_layout_id(item_0),
            ]
            for key_0, item_0 in value.type_layouts.items()
        ],
    }


def from_json_layout_segment(value: Json) -> LayoutSegment:
    """Return one LayoutSegment from one JSON value."""
    object_ = json_object(value)

    return LayoutSegment(
        module_id=destack._generated.source.file.model.module.from_json_module_id(
            json_field(object_, "moduleId")
        ),
        first_layout_id=json_int(json_field(object_, "firstLayoutId")),
        layouts=[
            from_json_layout(item_0)
            for item_0 in json_array(json_field(object_, "layouts"))
        ],
        type_layouts={
            destack._generated.dir.type.type.from_json_global_type_id(
                key_0
            ): from_json_local_layout_id(item_0)
            for key_0, item_0 in json_array(json_field(object_, "typeLayouts"))
        },
    )


@dataclass(frozen=True, slots=True)
class Layout:
    """Concrete memory layout for a checked type."""

    # the layout shape
    shape: LayoutShape
    # the size in bytes
    size: int
    # the alignment in bytes
    alignment: int
    # the largest niche of free scalar values, when one exists
    niche: Niche | None

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
    if value.niche is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_niche(writer, value.niche)


def decode_layout(reader: BinaryReader) -> Layout:
    """Decode one Layout."""
    shape = decode_layout_shape(reader)
    size = reader.read_number()
    alignment = reader.read_number()
    niche = reader.read_option(lambda: decode_niche(reader))

    return Layout(
        shape=shape,
        size=size,
        alignment=alignment,
        niche=niche,
    )


def to_json_layout(value: Layout) -> Json:
    """Return one JSON value for one Layout."""
    return {
        "shape": to_json_layout_shape(value.shape),
        "size": value.size,
        "alignment": value.alignment,
        **({} if value.niche is None else {"niche": to_json_niche(value.niche)}),
    }


def from_json_layout(value: Json) -> Layout:
    """Return one Layout from one JSON value."""
    object_ = json_object(value)

    return Layout(
        shape=from_json_layout_shape(json_field(object_, "shape")),
        size=json_int(json_field(object_, "size")),
        alignment=json_int(json_field(object_, "alignment")),
        niche=json_optional(object_, "niche", lambda value: from_json_niche(value)),
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
    """Runtime dynamic value storage."""

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


@dataclass(frozen=True, slots=True)
class LayoutShapePointer:
    """Pointer storage slot for an indirectly stored value."""

    pointer: PointerLayout
    kind: typing.Literal["pointer"] = "pointer"

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
    | LayoutShapePointer
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
    elif value.kind == "pointer":
        writer.write_unsigned(14)
        encode_pointer_layout(writer, value.pointer)
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
    elif variant == 14:
        pointer = decode_pointer_layout(reader)

        return LayoutShapePointer(pointer=pointer)
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
    elif value.kind == "pointer":
        return {
            "kind": "pointer",
            "pointer": to_json_pointer_layout(value.pointer),
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
    elif kind == "pointer":
        return LayoutShapePointer(
            pointer=from_json_pointer_layout(json_field(object_, "pointer"))
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
    """Concrete field or tuple-element layout."""

    # the field key
    key: destack._generated.dir.symbol.key.StaticKey | None
    # the field type
    ty: destack._generated.dir.type.type.GlobalTypeId
    # the field layout
    layout: LocalLayoutId
    # the offset in bytes
    offset: int
    # the size in bytes
    size: int
    # the alignment in bytes
    alignment: int

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
    if value.key is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.symbol.key.encode_static_key(writer, value.key)
    destack._generated.dir.type.type.encode_global_type_id(writer, value.ty)
    encode_local_layout_id(writer, value.layout)
    writer.write_unsigned(value.offset)
    writer.write_unsigned(value.size)
    writer.write_unsigned(value.alignment)


def decode_layout_field(reader: BinaryReader) -> LayoutField:
    """Decode one LayoutField."""
    key = reader.read_option(
        lambda: destack._generated.dir.symbol.key.decode_static_key(reader)
    )
    ty = destack._generated.dir.type.type.decode_global_type_id(reader)
    layout = decode_local_layout_id(reader)
    offset = reader.read_number()
    size = reader.read_number()
    alignment = reader.read_number()

    return LayoutField(
        key=key,
        ty=ty,
        layout=layout,
        offset=offset,
        size=size,
        alignment=alignment,
    )


def to_json_layout_field(value: LayoutField) -> Json:
    """Return one JSON value for one LayoutField."""
    return {
        **(
            {}
            if value.key is None
            else {
                "key": destack._generated.dir.symbol.key.to_json_static_key(value.key)
            }
        ),
        "ty": destack._generated.dir.type.type.to_json_global_type_id(value.ty),
        "layout": to_json_local_layout_id(value.layout),
        "offset": value.offset,
        "size": value.size,
        "alignment": value.alignment,
    }


def from_json_layout_field(value: Json) -> LayoutField:
    """Return one LayoutField from one JSON value."""
    object_ = json_object(value)

    return LayoutField(
        key=json_optional(
            object_,
            "key",
            lambda value: destack._generated.dir.symbol.key.from_json_static_key(value),
        ),
        ty=destack._generated.dir.type.type.from_json_global_type_id(
            json_field(object_, "ty")
        ),
        layout=from_json_local_layout_id(json_field(object_, "layout")),
        offset=json_int(json_field(object_, "offset")),
        size=json_int(json_field(object_, "size")),
        alignment=json_int(json_field(object_, "alignment")),
    )


"""Unique identifier for a concrete layout."""
LocalLayoutId: typing.TypeAlias = int


def encode_local_layout_id(writer: BinaryWriter, value: LocalLayoutId) -> None:
    """Encode one LocalLayoutId."""
    writer.write_unsigned(value)


def decode_local_layout_id(reader: BinaryReader) -> LocalLayoutId:
    """Decode one LocalLayoutId."""
    return reader.read_number()


def to_json_local_layout_id(value: LocalLayoutId) -> Json:
    """Return one JSON value for one LocalLayoutId."""
    return value


def from_json_local_layout_id(value: Json) -> LocalLayoutId:
    """Return one LocalLayoutId from one JSON value."""
    return json_int(value)


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
    element: destack._generated.dir.type.type.GlobalTypeId
    # the byte stride between elements
    stride: int
    # the fixed element count when known
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
    destack._generated.dir.type.type.encode_global_type_id(writer, value.element)
    writer.write_unsigned(value.stride)
    writer.write_unsigned(value.count)


def decode_element_layout(reader: BinaryReader) -> ElementLayout:
    """Decode one ElementLayout."""
    element = destack._generated.dir.type.type.decode_global_type_id(reader)
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
        "element": destack._generated.dir.type.type.to_json_global_type_id(
            value.element
        ),
        "stride": value.stride,
        "count": value.count,
    }


def from_json_element_layout(value: Json) -> ElementLayout:
    """Return one ElementLayout from one JSON value."""
    object_ = json_object(value)

    return ElementLayout(
        element=destack._generated.dir.type.type.from_json_global_type_id(
            json_field(object_, "element")
        ),
        stride=json_int(json_field(object_, "stride")),
        count=json_int(json_field(object_, "count")),
    )


@dataclass(frozen=True, slots=True)
class TensorLayout:
    """Concrete layout for a tensor handle."""

    # the tensor element type
    element: destack._generated.dir.type.type.GlobalTypeId
    # the tensor storage format
    format: TensorFormat
    # the tensor placement
    sharding: TensorSharding
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
    destack._generated.dir.type.type.encode_global_type_id(writer, value.element)
    encode_tensor_format(writer, value.format)
    encode_tensor_sharding(writer, value.sharding)
    writer.write_unsigned(value.rank)


def decode_tensor_layout(reader: BinaryReader) -> TensorLayout:
    """Decode one TensorLayout."""
    element = destack._generated.dir.type.type.decode_global_type_id(reader)
    format = decode_tensor_format(reader)
    sharding = decode_tensor_sharding(reader)
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
        "element": destack._generated.dir.type.type.to_json_global_type_id(
            value.element
        ),
        "format": to_json_tensor_format(value.format),
        "sharding": to_json_tensor_sharding(value.sharding),
        "rank": value.rank,
    }


def from_json_tensor_layout(value: Json) -> TensorLayout:
    """Return one TensorLayout from one JSON value."""
    object_ = json_object(value)

    return TensorLayout(
        element=destack._generated.dir.type.type.from_json_global_type_id(
            json_field(object_, "element")
        ),
        format=from_json_tensor_format(json_field(object_, "format")),
        sharding=from_json_tensor_sharding(json_field(object_, "sharding")),
        rank=json_int(json_field(object_, "rank")),
    )


@dataclass(frozen=True, slots=True)
class TensorFormatDense:
    """Dense contiguous format."""

    # the dimension order
    order: TensorDimensionOrder
    kind: typing.Literal["dense"] = "dense"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_format(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_format(self)


"""Format for an owning tensor value."""
TensorFormat: typing.TypeAlias = TensorFormatDense


def encode_tensor_format(writer: BinaryWriter, value: TensorFormat) -> None:
    """Encode one TensorFormat."""
    if value.kind == "dense":
        writer.write_unsigned(0)
        encode_tensor_dimension_order(writer, value.order)
    else:
        raise SerdeError("unknown enum variant")


def decode_tensor_format(reader: BinaryReader) -> TensorFormat:
    """Decode one TensorFormat."""
    variant = reader.read_number()

    if variant == 0:
        order = decode_tensor_dimension_order(reader)

        return TensorFormatDense(
            order=order,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_tensor_format(value: TensorFormat) -> Json:
    """Return one JSON value for one TensorFormat."""
    if value.kind == "dense":
        return {
            "kind": "dense",
            "order": to_json_tensor_dimension_order(value.order),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_tensor_format(value: Json) -> TensorFormat:
    """Return one TensorFormat from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "dense":
        return TensorFormatDense(
            order=from_json_tensor_dimension_order(json_field(object_, "order")),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


"""Dimension order for dense tensor storage."""
TensorDimensionOrder: typing.TypeAlias = (
    typing.Literal["rowMajor"] | typing.Literal["columnMajor"]
)


def encode_tensor_dimension_order(
    writer: BinaryWriter, value: TensorDimensionOrder
) -> None:
    """Encode one TensorDimensionOrder."""
    if value == "rowMajor":
        writer.write_unsigned(0)
    elif value == "columnMajor":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_tensor_dimension_order(reader: BinaryReader) -> TensorDimensionOrder:
    """Decode one TensorDimensionOrder."""
    variant = reader.read_number()

    if variant == 0:
        return "rowMajor"
    elif variant == 1:
        return "columnMajor"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_tensor_dimension_order(value: TensorDimensionOrder) -> Json:
    """Return one JSON value for one TensorDimensionOrder."""
    return value


def from_json_tensor_dimension_order(value: Json) -> TensorDimensionOrder:
    """Return one TensorDimensionOrder from one JSON value."""
    variant = json_string(value)

    if variant == "rowMajor":
        return "rowMajor"
    elif variant == "columnMajor":
        return "columnMajor"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class TensorShardingUnsharded:
    """Tensor storage is not partitioned across a mesh."""

    kind: typing.Literal["unsharded"] = "unsharded"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_sharding(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_sharding(self)


@dataclass(frozen=True, slots=True)
class TensorShardingSharding:
    """Tensor storage is mapped across a mesh axis by axis."""

    # the per-axis placement descriptors
    axes: Sequence[TensorShardingAxis]
    kind: typing.Literal["sharding"] = "sharding"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_sharding(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_sharding(self)


"""Placement descriptor for tensor storage."""
TensorSharding: typing.TypeAlias = TensorShardingUnsharded | TensorShardingSharding


def encode_tensor_sharding(writer: BinaryWriter, value: TensorSharding) -> None:
    """Encode one TensorSharding."""
    if value.kind == "unsharded":
        writer.write_unsigned(0)
    elif value.kind == "sharding":
        writer.write_unsigned(1)
        writer.write_unsigned(len(value.axes))
        for item_value_axes_0 in value.axes:
            encode_tensor_sharding_axis(writer, item_value_axes_0)
    else:
        raise SerdeError("unknown enum variant")


def decode_tensor_sharding(reader: BinaryReader) -> TensorSharding:
    """Decode one TensorSharding."""
    variant = reader.read_number()

    if variant == 0:
        return TensorShardingUnsharded()
    elif variant == 1:
        axes = [
            decode_tensor_sharding_axis(reader) for _ in range(reader.read_number())
        ]

        return TensorShardingSharding(
            axes=axes,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_tensor_sharding(value: TensorSharding) -> Json:
    """Return one JSON value for one TensorSharding."""
    if value.kind == "unsharded":
        return {
            "kind": "unsharded",
        }
    elif value.kind == "sharding":
        return {
            "kind": "sharding",
            "axes": [to_json_tensor_sharding_axis(item_0) for item_0 in value.axes],
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_tensor_sharding(value: Json) -> TensorSharding:
    """Return one TensorSharding from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "unsharded":
        return TensorShardingUnsharded()
    elif kind == "sharding":
        return TensorShardingSharding(
            axes=[
                from_json_tensor_sharding_axis(item_0)
                for item_0 in json_array(json_field(object_, "axes"))
            ],
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class TensorShardingAxisShard:
    """Split one tensor axis across one mesh axis."""

    # the tensor axis being split
    axis: int
    kind: typing.Literal["shard"] = "shard"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_sharding_axis(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_sharding_axis(self)


@dataclass(frozen=True, slots=True)
class TensorShardingAxisReplicate:
    """Replicate values across one mesh axis."""

    kind: typing.Literal["replicate"] = "replicate"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_sharding_axis(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_sharding_axis(self)


@dataclass(frozen=True, slots=True)
class TensorShardingAxisPartial:
    """Store partial results across one mesh axis."""

    # the reduction used to combine partial values
    reduction: TensorReduction
    kind: typing.Literal["partial"] = "partial"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_sharding_axis(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_sharding_axis(self)


"""Per-axis placement descriptor for a sharded tensor."""
TensorShardingAxis: typing.TypeAlias = (
    TensorShardingAxisShard | TensorShardingAxisReplicate | TensorShardingAxisPartial
)


def encode_tensor_sharding_axis(
    writer: BinaryWriter, value: TensorShardingAxis
) -> None:
    """Encode one TensorShardingAxis."""
    if value.kind == "shard":
        writer.write_unsigned(0)
        writer.write_signed(value.axis)
    elif value.kind == "replicate":
        writer.write_unsigned(1)
    elif value.kind == "partial":
        writer.write_unsigned(2)
        encode_tensor_reduction(writer, value.reduction)
    else:
        raise SerdeError("unknown enum variant")


def decode_tensor_sharding_axis(reader: BinaryReader) -> TensorShardingAxis:
    """Decode one TensorShardingAxis."""
    variant = reader.read_number()

    if variant == 0:
        axis = reader.read_signed_number()

        return TensorShardingAxisShard(
            axis=axis,
        )
    elif variant == 1:
        return TensorShardingAxisReplicate()
    elif variant == 2:
        reduction = decode_tensor_reduction(reader)

        return TensorShardingAxisPartial(
            reduction=reduction,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_tensor_sharding_axis(value: TensorShardingAxis) -> Json:
    """Return one JSON value for one TensorShardingAxis."""
    if value.kind == "shard":
        return {
            "kind": "shard",
            "axis": value.axis,
        }
    elif value.kind == "replicate":
        return {
            "kind": "replicate",
        }
    elif value.kind == "partial":
        return {
            "kind": "partial",
            "reduction": to_json_tensor_reduction(value.reduction),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_tensor_sharding_axis(value: Json) -> TensorShardingAxis:
    """Return one TensorShardingAxis from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "shard":
        return TensorShardingAxisShard(
            axis=json_int(json_field(object_, "axis")),
        )
    elif kind == "replicate":
        return TensorShardingAxisReplicate()
    elif kind == "partial":
        return TensorShardingAxisPartial(
            reduction=from_json_tensor_reduction(json_field(object_, "reduction")),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


"""Reduction used when partial tensor shards are combined."""
TensorReduction: typing.TypeAlias = (
    typing.Literal["add"]
    | typing.Literal["multiply"]
    | typing.Literal["minimum"]
    | typing.Literal["maximum"]
    | typing.Literal["and"]
    | typing.Literal["or"]
)


def encode_tensor_reduction(writer: BinaryWriter, value: TensorReduction) -> None:
    """Encode one TensorReduction."""
    if value == "add":
        writer.write_unsigned(0)
    elif value == "multiply":
        writer.write_unsigned(1)
    elif value == "minimum":
        writer.write_unsigned(2)
    elif value == "maximum":
        writer.write_unsigned(3)
    elif value == "and":
        writer.write_unsigned(4)
    elif value == "or":
        writer.write_unsigned(5)
    else:
        raise SerdeError("unknown enum variant")


def decode_tensor_reduction(reader: BinaryReader) -> TensorReduction:
    """Decode one TensorReduction."""
    variant = reader.read_number()

    if variant == 0:
        return "add"
    elif variant == 1:
        return "multiply"
    elif variant == 2:
        return "minimum"
    elif variant == 3:
        return "maximum"
    elif variant == 4:
        return "and"
    elif variant == 5:
        return "or"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_tensor_reduction(value: TensorReduction) -> Json:
    """Return one JSON value for one TensorReduction."""
    return value


def from_json_tensor_reduction(value: Json) -> TensorReduction:
    """Return one TensorReduction from one JSON value."""
    variant = json_string(value)

    if variant == "add":
        return "add"
    elif variant == "multiply":
        return "multiply"
    elif variant == "minimum":
        return "minimum"
    elif variant == "maximum":
        return "maximum"
    elif variant == "and":
        return "and"
    elif variant == "or":
        return "or"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class TensorViewLayout:
    """Concrete layout for a tensor view descriptor."""

    # the viewed element type
    element: destack._generated.dir.type.type.GlobalTypeId
    # the tensor view format
    format: TensorViewFormat
    # the tensor placement
    sharding: TensorSharding
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
    destack._generated.dir.type.type.encode_global_type_id(writer, value.element)
    encode_tensor_view_format(writer, value.format)
    encode_tensor_sharding(writer, value.sharding)
    writer.write_unsigned(value.rank)


def decode_tensor_view_layout(reader: BinaryReader) -> TensorViewLayout:
    """Decode one TensorViewLayout."""
    element = destack._generated.dir.type.type.decode_global_type_id(reader)
    format = decode_tensor_view_format(reader)
    sharding = decode_tensor_sharding(reader)
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
        "element": destack._generated.dir.type.type.to_json_global_type_id(
            value.element
        ),
        "format": to_json_tensor_view_format(value.format),
        "sharding": to_json_tensor_sharding(value.sharding),
        "rank": value.rank,
    }


def from_json_tensor_view_layout(value: Json) -> TensorViewLayout:
    """Return one TensorViewLayout from one JSON value."""
    object_ = json_object(value)

    return TensorViewLayout(
        element=destack._generated.dir.type.type.from_json_global_type_id(
            json_field(object_, "element")
        ),
        format=from_json_tensor_view_format(json_field(object_, "format")),
        sharding=from_json_tensor_sharding(json_field(object_, "sharding")),
        rank=json_int(json_field(object_, "rank")),
    )


@dataclass(frozen=True, slots=True)
class TensorViewFormatDense:
    """Dense contiguous view."""

    # the dimension order
    order: TensorDimensionOrder
    kind: typing.Literal["dense"] = "dense"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_view_format(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_view_format(self)


@dataclass(frozen=True, slots=True)
class TensorViewFormatStrided:
    """Explicit strided view."""

    kind: typing.Literal["strided"] = "strided"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_view_format(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_view_format(self)


"""Format descriptor for a tensor view."""
TensorViewFormat: typing.TypeAlias = TensorViewFormatDense | TensorViewFormatStrided


def encode_tensor_view_format(writer: BinaryWriter, value: TensorViewFormat) -> None:
    """Encode one TensorViewFormat."""
    if value.kind == "dense":
        writer.write_unsigned(0)
        encode_tensor_dimension_order(writer, value.order)
    elif value.kind == "strided":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_tensor_view_format(reader: BinaryReader) -> TensorViewFormat:
    """Decode one TensorViewFormat."""
    variant = reader.read_number()

    if variant == 0:
        order = decode_tensor_dimension_order(reader)

        return TensorViewFormatDense(
            order=order,
        )
    elif variant == 1:
        return TensorViewFormatStrided()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_tensor_view_format(value: TensorViewFormat) -> Json:
    """Return one JSON value for one TensorViewFormat."""
    if value.kind == "dense":
        return {
            "kind": "dense",
            "order": to_json_tensor_dimension_order(value.order),
        }
    elif value.kind == "strided":
        return {
            "kind": "strided",
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_tensor_view_format(value: Json) -> TensorViewFormat:
    """Return one TensorViewFormat from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "dense":
        return TensorViewFormatDense(
            order=from_json_tensor_dimension_order(json_field(object_, "order")),
        )
    elif kind == "strided":
        return TensorViewFormatStrided()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class VariantLayout:
    """Concrete layout for a variant value."""

    # the tag layout
    tag: VariantTagLayout
    # the variant payload byte offset
    payload_offset: int | None
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
    if value.payload_offset is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(value.payload_offset)
    writer.write_unsigned(len(value.variants))
    for item_value_variants_0 in value.variants:
        encode_variant_case_layout(writer, item_value_variants_0)


def decode_variant_layout(reader: BinaryReader) -> VariantLayout:
    """Decode one VariantLayout."""
    tag = decode_variant_tag_layout(reader)
    payload_offset = reader.read_option(lambda: reader.read_number())
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
        **(
            {}
            if value.payload_offset is None
            else {"payloadOffset": value.payload_offset}
        ),
        "variants": [to_json_variant_case_layout(item_0) for item_0 in value.variants],
    }


def from_json_variant_layout(value: Json) -> VariantLayout:
    """Return one VariantLayout from one JSON value."""
    object_ = json_object(value)

    return VariantLayout(
        tag=from_json_variant_tag_layout(json_field(object_, "tag")),
        payload_offset=json_optional(
            object_, "payloadOffset", lambda value: json_int(value)
        ),
        variants=[
            from_json_variant_case_layout(item_0)
            for item_0 in json_array(json_field(object_, "variants"))
        ],
    )


@dataclass(frozen=True, slots=True)
class VariantTagLayout:
    """Concrete layout for a variant tag."""

    # the tag type when it has been materialized
    ty: destack._generated.dir.type.type.GlobalTypeId | None
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
        destack._generated.dir.type.type.encode_global_type_id(writer, value.ty)
    writer.write_unsigned(value.size)
    writer.write_unsigned(value.alignment)


def decode_variant_tag_layout(reader: BinaryReader) -> VariantTagLayout:
    """Decode one VariantTagLayout."""
    ty = reader.read_option(
        lambda: destack._generated.dir.type.type.decode_global_type_id(reader)
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
                "ty": destack._generated.dir.type.type.to_json_global_type_id(value.ty)
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
            lambda value: destack._generated.dir.type.type.from_json_global_type_id(
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
    ty: destack._generated.dir.type.type.GlobalTypeId
    # the case layout
    layout: LocalLayoutId

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
    destack._generated.dir.type.type.encode_global_type_id(writer, value.ty)
    encode_local_layout_id(writer, value.layout)


def decode_variant_case_layout(reader: BinaryReader) -> VariantCaseLayout:
    """Decode one VariantCaseLayout."""
    ty = destack._generated.dir.type.type.decode_global_type_id(reader)
    layout = decode_local_layout_id(reader)

    return VariantCaseLayout(
        ty=ty,
        layout=layout,
    )


def to_json_variant_case_layout(value: VariantCaseLayout) -> Json:
    """Return one JSON value for one VariantCaseLayout."""
    return {
        "ty": destack._generated.dir.type.type.to_json_global_type_id(value.ty),
        "layout": to_json_local_layout_id(value.layout),
    }


def from_json_variant_case_layout(value: Json) -> VariantCaseLayout:
    """Return one VariantCaseLayout from one JSON value."""
    object_ = json_object(value)

    return VariantCaseLayout(
        ty=destack._generated.dir.type.type.from_json_global_type_id(
            json_field(object_, "ty")
        ),
        layout=from_json_local_layout_id(json_field(object_, "layout")),
    )


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
    backing_type: destack._generated.dir.type.type.GlobalTypeId
    # the backing type layout
    backing_layout: LocalLayoutId

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
    destack._generated.dir.type.type.encode_global_type_id(writer, value.backing_type)
    encode_local_layout_id(writer, value.backing_layout)


def decode_newtype_layout(reader: BinaryReader) -> NewtypeLayout:
    """Decode one NewtypeLayout."""
    backing_type = destack._generated.dir.type.type.decode_global_type_id(reader)
    backing_layout = decode_local_layout_id(reader)

    return NewtypeLayout(
        backing_type=backing_type,
        backing_layout=backing_layout,
    )


def to_json_newtype_layout(value: NewtypeLayout) -> Json:
    """Return one JSON value for one NewtypeLayout."""
    return {
        "backingType": destack._generated.dir.type.type.to_json_global_type_id(
            value.backing_type
        ),
        "backingLayout": to_json_local_layout_id(value.backing_layout),
    }


def from_json_newtype_layout(value: Json) -> NewtypeLayout:
    """Return one NewtypeLayout from one JSON value."""
    object_ = json_object(value)

    return NewtypeLayout(
        backing_type=destack._generated.dir.type.type.from_json_global_type_id(
            json_field(object_, "backingType")
        ),
        backing_layout=from_json_local_layout_id(json_field(object_, "backingLayout")),
    )


@dataclass(frozen=True, slots=True)
class PointerLayout:
    """Concrete layout for a pointer storage slot."""

    # the pointed-to value type
    pointee: destack._generated.dir.type.type.GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pointer_layout(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> PointerLayout:
        """Decode one PointerLayout."""
        return decode_pointer_layout(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pointer_layout(self)

    @classmethod
    def from_json(cls, value: Json) -> PointerLayout:
        """Return one PointerLayout from one JSON value."""
        return from_json_pointer_layout(value)


def encode_pointer_layout(writer: BinaryWriter, value: PointerLayout) -> None:
    """Encode one PointerLayout."""
    destack._generated.dir.type.type.encode_global_type_id(writer, value.pointee)


def decode_pointer_layout(reader: BinaryReader) -> PointerLayout:
    """Decode one PointerLayout."""
    pointee = destack._generated.dir.type.type.decode_global_type_id(reader)

    return PointerLayout(
        pointee=pointee,
    )


def to_json_pointer_layout(value: PointerLayout) -> Json:
    """Return one JSON value for one PointerLayout."""
    return {
        "pointee": destack._generated.dir.type.type.to_json_global_type_id(
            value.pointee
        ),
    }


def from_json_pointer_layout(value: Json) -> PointerLayout:
    """Return one PointerLayout from one JSON value."""
    object_ = json_object(value)

    return PointerLayout(
        pointee=destack._generated.dir.type.type.from_json_global_type_id(
            json_field(object_, "pointee")
        ),
    )


@dataclass(frozen=True, slots=True)
class Niche:
    """One niche of free values inside a layout."""

    # the byte offset of the niched scalar
    offset: int
    # the niched scalar width in bytes
    width: int
    # the first valid value stored by the type
    start: int
    # the last valid value stored by the type
    end: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_niche(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Niche:
        """Decode one Niche."""
        return decode_niche(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_niche(self)

    @classmethod
    def from_json(cls, value: Json) -> Niche:
        """Return one Niche from one JSON value."""
        return from_json_niche(value)


def encode_niche(writer: BinaryWriter, value: Niche) -> None:
    """Encode one Niche."""
    writer.write_unsigned(value.offset)
    writer.write_unsigned(value.width)
    writer.write_unsigned(value.start)
    writer.write_unsigned(value.end)


def decode_niche(reader: BinaryReader) -> Niche:
    """Decode one Niche."""
    offset = reader.read_number()
    width = reader.read_number()
    start = reader.read_unsigned()
    end = reader.read_unsigned()

    return Niche(
        offset=offset,
        width=width,
        start=start,
        end=end,
    )


def to_json_niche(value: Niche) -> Json:
    """Return one JSON value for one Niche."""
    return {
        "offset": value.offset,
        "width": value.width,
        "start": value.start,
        "end": value.end,
    }


def from_json_niche(value: Json) -> Niche:
    """Return one Niche from one JSON value."""
    object_ = json_object(value)

    return Niche(
        offset=json_int(json_field(object_, "offset")),
        width=json_int(json_field(object_, "width")),
        start=json_int(json_field(object_, "start")),
        end=json_int(json_field(object_, "end")),
    )


__all__ = [
    "LayoutSegment",
    "encode_layout_segment",
    "decode_layout_segment",
    "to_json_layout_segment",
    "from_json_layout_segment",
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
    "LayoutShapePointer",
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
    "LocalLayoutId",
    "encode_local_layout_id",
    "decode_local_layout_id",
    "to_json_local_layout_id",
    "from_json_local_layout_id",
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
    "TensorFormat",
    "encode_tensor_format",
    "decode_tensor_format",
    "to_json_tensor_format",
    "from_json_tensor_format",
    "TensorFormatDense",
    "TensorDimensionOrder",
    "encode_tensor_dimension_order",
    "decode_tensor_dimension_order",
    "to_json_tensor_dimension_order",
    "from_json_tensor_dimension_order",
    "TensorSharding",
    "encode_tensor_sharding",
    "decode_tensor_sharding",
    "to_json_tensor_sharding",
    "from_json_tensor_sharding",
    "TensorShardingUnsharded",
    "TensorShardingSharding",
    "TensorShardingAxis",
    "encode_tensor_sharding_axis",
    "decode_tensor_sharding_axis",
    "to_json_tensor_sharding_axis",
    "from_json_tensor_sharding_axis",
    "TensorShardingAxisShard",
    "TensorShardingAxisReplicate",
    "TensorShardingAxisPartial",
    "TensorReduction",
    "encode_tensor_reduction",
    "decode_tensor_reduction",
    "to_json_tensor_reduction",
    "from_json_tensor_reduction",
    "TensorViewLayout",
    "encode_tensor_view_layout",
    "decode_tensor_view_layout",
    "to_json_tensor_view_layout",
    "from_json_tensor_view_layout",
    "TensorViewFormat",
    "encode_tensor_view_format",
    "decode_tensor_view_format",
    "to_json_tensor_view_format",
    "from_json_tensor_view_format",
    "TensorViewFormatDense",
    "TensorViewFormatStrided",
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
    "PointerLayout",
    "encode_pointer_layout",
    "decode_pointer_layout",
    "to_json_pointer_layout",
    "from_json_pointer_layout",
    "Niche",
    "encode_niche",
    "decode_niche",
    "to_json_niche",
    "from_json_niche",
]
