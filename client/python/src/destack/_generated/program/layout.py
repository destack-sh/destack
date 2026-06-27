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
    json_bool,
    json_field,
    json_int,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.core.string
import destack._generated.mir.table.trace
import destack._generated.mir.tree.type
import destack._generated.program.function
import destack._generated.program.type

"""Opaque identifier for one executable memory layout."""
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
class LayoutTable:
    """Shared executable layout table for all runtime value layouts."""

    # layout entries indexed by LayoutId
    entries: Sequence[Layout]

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


def decode_layout_table(reader: BinaryReader) -> LayoutTable:
    """Decode one LayoutTable."""
    entries = [decode_layout(reader) for _ in range(reader.read_number())]

    return LayoutTable(
        entries=entries,
    )


def to_json_layout_table(value: LayoutTable) -> Json:
    """Return one JSON value for one LayoutTable."""
    return {
        "entries": [to_json_layout(item_0) for item_0 in value.entries],
    }


def from_json_layout_table(value: Json) -> LayoutTable:
    """Return one LayoutTable from one JSON value."""
    object_ = json_object(value)

    return LayoutTable(
        entries=[
            from_json_layout(item_0)
            for item_0 in json_array(json_field(object_, "entries"))
        ],
    )


@dataclass(frozen=True, slots=True)
class Layout:
    """Concrete executable memory layout for one runtime value."""

    # the layout shape
    shape: LayoutShape
    # total size in bytes, including trailing padding
    size: int
    # alignment requirement in bytes
    alignment: int
    # managed-reference trace id for this layout
    trace: destack._generated.mir.table.trace.TraceId

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
    destack._generated.mir.table.trace.encode_trace_id(writer, value.trace)


def decode_layout(reader: BinaryReader) -> Layout:
    """Decode one Layout."""
    shape = decode_layout_shape(reader)
    size = reader.read_number()
    alignment = reader.read_number()
    trace = destack._generated.mir.table.trace.decode_trace_id(reader)

    return Layout(
        shape=shape,
        size=size,
        alignment=alignment,
        trace=trace,
    )


def to_json_layout(value: Layout) -> Json:
    """Return one JSON value for one Layout."""
    return {
        "shape": to_json_layout_shape(value.shape),
        "size": value.size,
        "alignment": value.alignment,
        "trace": destack._generated.mir.table.trace.to_json_trace_id(value.trace),
    }


def from_json_layout(value: Json) -> Layout:
    """Return one Layout from one JSON value."""
    object_ = json_object(value)

    return Layout(
        shape=from_json_layout_shape(json_field(object_, "shape")),
        size=json_int(json_field(object_, "size")),
        alignment=json_int(json_field(object_, "alignment")),
        trace=destack._generated.mir.table.trace.from_json_trace_id(
            json_field(object_, "trace")
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

    scalar: ScalarFormat
    kind: typing.Literal["scalar"] = "scalar"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_layout_shape(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_layout_shape(self)


@dataclass(frozen=True, slots=True)
class LayoutShapeReference:
    """Reference storage."""

    reference: ReferenceLayout
    kind: typing.Literal["reference"] = "reference"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_layout_shape(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_layout_shape(self)


@dataclass(frozen=True, slots=True)
class LayoutShapeFunctionPointer:
    """Function pointer storage."""

    function_pointer: destack._generated.program.function.Signature
    kind: typing.Literal["functionPointer"] = "functionPointer"

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

    slice: SliceLayout
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
    """Object storage."""

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

    function: FunctionLayout
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


"""Concrete executable layout shape."""
LayoutShape: typing.TypeAlias = (
    LayoutShapeNone
    | LayoutShapeScalar
    | LayoutShapeReference
    | LayoutShapeFunctionPointer
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
        encode_scalar_format(writer, value.scalar)
    elif value.kind == "reference":
        writer.write_unsigned(2)
        encode_reference_layout(writer, value.reference)
    elif value.kind == "functionPointer":
        writer.write_unsigned(3)
        destack._generated.program.function.encode_signature(
            writer, value.function_pointer
        )
    elif value.kind == "struct":
        writer.write_unsigned(4)
        encode_struct_layout(writer, value.struct)
    elif value.kind == "tuple":
        writer.write_unsigned(5)
        encode_tuple_layout(writer, value.tuple)
    elif value.kind == "slice":
        writer.write_unsigned(6)
        encode_slice_layout(writer, value.slice)
    elif value.kind == "array":
        writer.write_unsigned(7)
        encode_element_layout(writer, value.array)
    elif value.kind == "vector":
        writer.write_unsigned(8)
        encode_element_layout(writer, value.vector)
    elif value.kind == "tensor":
        writer.write_unsigned(9)
        encode_tensor_layout(writer, value.tensor)
    elif value.kind == "tensorView":
        writer.write_unsigned(10)
        encode_tensor_view_layout(writer, value.tensor_view)
    elif value.kind == "variant":
        writer.write_unsigned(11)
        encode_variant_layout(writer, value.variant)
    elif value.kind == "object":
        writer.write_unsigned(12)
        encode_object_layout(writer, value.object)
    elif value.kind == "dynamic":
        writer.write_unsigned(13)
    elif value.kind == "function":
        writer.write_unsigned(14)
        encode_function_layout(writer, value.function)
    elif value.kind == "newtype":
        writer.write_unsigned(15)
        encode_newtype_layout(writer, value.newtype)
    else:
        raise SerdeError("unknown enum variant")


def decode_layout_shape(reader: BinaryReader) -> LayoutShape:
    """Decode one LayoutShape."""
    variant = reader.read_number()

    if variant == 0:
        return LayoutShapeNone()
    elif variant == 1:
        scalar = decode_scalar_format(reader)

        return LayoutShapeScalar(scalar=scalar)
    elif variant == 2:
        reference = decode_reference_layout(reader)

        return LayoutShapeReference(reference=reference)
    elif variant == 3:
        function_pointer = destack._generated.program.function.decode_signature(reader)

        return LayoutShapeFunctionPointer(function_pointer=function_pointer)
    elif variant == 4:
        struct = decode_struct_layout(reader)

        return LayoutShapeStruct(struct=struct)
    elif variant == 5:
        tuple = decode_tuple_layout(reader)

        return LayoutShapeTuple(tuple=tuple)
    elif variant == 6:
        slice = decode_slice_layout(reader)

        return LayoutShapeSlice(slice=slice)
    elif variant == 7:
        array = decode_element_layout(reader)

        return LayoutShapeArray(array=array)
    elif variant == 8:
        vector = decode_element_layout(reader)

        return LayoutShapeVector(vector=vector)
    elif variant == 9:
        tensor = decode_tensor_layout(reader)

        return LayoutShapeTensor(tensor=tensor)
    elif variant == 10:
        tensor_view = decode_tensor_view_layout(reader)

        return LayoutShapeTensorView(tensor_view=tensor_view)
    elif variant == 11:
        variant = decode_variant_layout(reader)

        return LayoutShapeVariant(variant=variant)
    elif variant == 12:
        object = decode_object_layout(reader)

        return LayoutShapeObject(object=object)
    elif variant == 13:
        return LayoutShapeDynamic()
    elif variant == 14:
        function = decode_function_layout(reader)

        return LayoutShapeFunction(function=function)
    elif variant == 15:
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
            "scalar": to_json_scalar_format(value.scalar),
        }
    elif value.kind == "reference":
        return {
            "kind": "reference",
            "reference": to_json_reference_layout(value.reference),
        }
    elif value.kind == "functionPointer":
        return {
            "kind": "functionPointer",
            "function_pointer": destack._generated.program.function.to_json_signature(
                value.function_pointer
            ),
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
            "slice": to_json_slice_layout(value.slice),
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
            "function": to_json_function_layout(value.function),
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
        return LayoutShapeScalar(
            scalar=from_json_scalar_format(json_field(object_, "scalar"))
        )
    elif kind == "reference":
        return LayoutShapeReference(
            reference=from_json_reference_layout(json_field(object_, "reference"))
        )
    elif kind == "functionPointer":
        return LayoutShapeFunctionPointer(
            function_pointer=destack._generated.program.function.from_json_signature(
                json_field(object_, "function_pointer")
            )
        )
    elif kind == "struct":
        return LayoutShapeStruct(
            struct=from_json_struct_layout(json_field(object_, "struct"))
        )
    elif kind == "tuple":
        return LayoutShapeTuple(
            tuple=from_json_tuple_layout(json_field(object_, "tuple"))
        )
    elif kind == "slice":
        return LayoutShapeSlice(
            slice=from_json_slice_layout(json_field(object_, "slice"))
        )
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
        return LayoutShapeFunction(
            function=from_json_function_layout(json_field(object_, "function"))
        )
    elif kind == "newtype":
        return LayoutShapeNewtype(
            newtype=from_json_newtype_layout(json_field(object_, "newtype"))
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class ScalarFormatInt:
    """Signed or unsigned integers with a bit width."""

    # the bit width
    width: int
    # whether the integer is signed
    is_signed: bool
    kind: typing.Literal["int"] = "int"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_scalar_format(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_scalar_format(self)


@dataclass(frozen=True, slots=True)
class ScalarFormatFloat:
    """Floating-point values with a concrete format."""

    # the concrete float format
    format: destack._generated.mir.tree.type.FloatType
    kind: typing.Literal["float"] = "float"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_scalar_format(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_scalar_format(self)


@dataclass(frozen=True, slots=True)
class ScalarFormatBoolean:
    """Boolean values."""

    kind: typing.Literal["boolean"] = "boolean"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_scalar_format(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_scalar_format(self)


"""Scalar storage format for vector and tensor element operations."""
ScalarFormat: typing.TypeAlias = (
    ScalarFormatInt | ScalarFormatFloat | ScalarFormatBoolean
)


def encode_scalar_format(writer: BinaryWriter, value: ScalarFormat) -> None:
    """Encode one ScalarFormat."""
    if value.kind == "int":
        writer.write_unsigned(0)
        writer.write_unsigned(value.width)
        writer.write_bool(value.is_signed)
    elif value.kind == "float":
        writer.write_unsigned(1)
        destack._generated.mir.tree.type.encode_float_type(writer, value.format)
    elif value.kind == "boolean":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_scalar_format(reader: BinaryReader) -> ScalarFormat:
    """Decode one ScalarFormat."""
    variant = reader.read_number()

    if variant == 0:
        width = reader.read_number()
        is_signed = reader.read_bool()

        return ScalarFormatInt(
            width=width,
            is_signed=is_signed,
        )
    elif variant == 1:
        format = destack._generated.mir.tree.type.decode_float_type(reader)

        return ScalarFormatFloat(
            format=format,
        )
    elif variant == 2:
        return ScalarFormatBoolean()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_scalar_format(value: ScalarFormat) -> Json:
    """Return one JSON value for one ScalarFormat."""
    if value.kind == "int":
        return {
            "kind": "int",
            "width": value.width,
            "isSigned": value.is_signed,
        }
    elif value.kind == "float":
        return {
            "kind": "float",
            "format": destack._generated.mir.tree.type.to_json_float_type(value.format),
        }
    elif value.kind == "boolean":
        return {
            "kind": "boolean",
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_scalar_format(value: Json) -> ScalarFormat:
    """Return one ScalarFormat from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "int":
        return ScalarFormatInt(
            width=json_int(json_field(object_, "width")),
            is_signed=json_bool(json_field(object_, "isSigned")),
        )
    elif kind == "float":
        return ScalarFormatFloat(
            format=destack._generated.mir.tree.type.from_json_float_type(
                json_field(object_, "format")
            ),
        )
    elif kind == "boolean":
        return ScalarFormatBoolean()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class ReferenceLayout:
    """Concrete layout for one reference value."""

    # the referenced value type
    pointee: destack._generated.program.type.TypeId
    # the packed reference flags
    flags: ReferenceFlags

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_reference_layout(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ReferenceLayout:
        """Decode one ReferenceLayout."""
        return decode_reference_layout(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_reference_layout(self)

    @classmethod
    def from_json(cls, value: Json) -> ReferenceLayout:
        """Return one ReferenceLayout from one JSON value."""
        return from_json_reference_layout(value)


def encode_reference_layout(writer: BinaryWriter, value: ReferenceLayout) -> None:
    """Encode one ReferenceLayout."""
    destack._generated.program.type.encode_type_id(writer, value.pointee)
    encode_reference_flags(writer, value.flags)


def decode_reference_layout(reader: BinaryReader) -> ReferenceLayout:
    """Decode one ReferenceLayout."""
    pointee = destack._generated.program.type.decode_type_id(reader)
    flags = decode_reference_flags(reader)

    return ReferenceLayout(
        pointee=pointee,
        flags=flags,
    )


def to_json_reference_layout(value: ReferenceLayout) -> Json:
    """Return one JSON value for one ReferenceLayout."""
    return {
        "pointee": destack._generated.program.type.to_json_type_id(value.pointee),
        "flags": to_json_reference_flags(value.flags),
    }


def from_json_reference_layout(value: Json) -> ReferenceLayout:
    """Return one ReferenceLayout from one JSON value."""
    object_ = json_object(value)

    return ReferenceLayout(
        pointee=destack._generated.program.type.from_json_type_id(
            json_field(object_, "pointee")
        ),
        flags=from_json_reference_flags(json_field(object_, "flags")),
    )


@dataclass(frozen=True, slots=True)
class ReferenceFlags:
    """Packed reference flags used by executable layouts."""

    bits: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_reference_flags(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ReferenceFlags:
        """Decode one ReferenceFlags."""
        return decode_reference_flags(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_reference_flags(self)

    @classmethod
    def from_json(cls, value: Json) -> ReferenceFlags:
        """Return one ReferenceFlags from one JSON value."""
        return from_json_reference_flags(value)


def encode_reference_flags(writer: BinaryWriter, value: ReferenceFlags) -> None:
    """Encode one ReferenceFlags."""
    writer.write_unsigned(value.bits)


def decode_reference_flags(reader: BinaryReader) -> ReferenceFlags:
    """Decode one ReferenceFlags."""
    bits = reader.read_number()

    return ReferenceFlags(
        bits=bits,
    )


def to_json_reference_flags(value: ReferenceFlags) -> Json:
    """Return one JSON value for one ReferenceFlags."""
    return {
        "bits": value.bits,
    }


def from_json_reference_flags(value: Json) -> ReferenceFlags:
    """Return one ReferenceFlags from one JSON value."""
    object_ = json_object(value)

    return ReferenceFlags(
        bits=json_int(json_field(object_, "bits")),
    )


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
    # program type of the field
    ty: destack._generated.program.type.TypeId
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
    destack._generated.program.type.encode_type_id(writer, value.ty)
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
    ty = destack._generated.program.type.decode_type_id(reader)
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
        "ty": destack._generated.program.type.to_json_type_id(value.ty),
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
        ty=destack._generated.program.type.from_json_type_id(json_field(object_, "ty")),
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
class SliceLayout:
    """Concrete layout for one slice descriptor."""

    # the backing data reference
    data: ReferenceLayout

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_slice_layout(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SliceLayout:
        """Decode one SliceLayout."""
        return decode_slice_layout(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_slice_layout(self)

    @classmethod
    def from_json(cls, value: Json) -> SliceLayout:
        """Return one SliceLayout from one JSON value."""
        return from_json_slice_layout(value)


def encode_slice_layout(writer: BinaryWriter, value: SliceLayout) -> None:
    """Encode one SliceLayout."""
    encode_reference_layout(writer, value.data)


def decode_slice_layout(reader: BinaryReader) -> SliceLayout:
    """Decode one SliceLayout."""
    data = decode_reference_layout(reader)

    return SliceLayout(
        data=data,
    )


def to_json_slice_layout(value: SliceLayout) -> Json:
    """Return one JSON value for one SliceLayout."""
    return {
        "data": to_json_reference_layout(value.data),
    }


def from_json_slice_layout(value: Json) -> SliceLayout:
    """Return one SliceLayout from one JSON value."""
    object_ = json_object(value)

    return SliceLayout(
        data=from_json_reference_layout(json_field(object_, "data")),
    )


@dataclass(frozen=True, slots=True)
class ElementLayout:
    """Layout for inline indexed element storage."""

    # the stored element type
    element: destack._generated.program.type.TypeId
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
    destack._generated.program.type.encode_type_id(writer, value.element)
    writer.write_unsigned(value.stride)
    writer.write_unsigned(value.count)


def decode_element_layout(reader: BinaryReader) -> ElementLayout:
    """Decode one ElementLayout."""
    element = destack._generated.program.type.decode_type_id(reader)
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
        "element": destack._generated.program.type.to_json_type_id(value.element),
        "stride": value.stride,
        "count": value.count,
    }


def from_json_element_layout(value: Json) -> ElementLayout:
    """Return one ElementLayout from one JSON value."""
    object_ = json_object(value)

    return ElementLayout(
        element=destack._generated.program.type.from_json_type_id(
            json_field(object_, "element")
        ),
        stride=json_int(json_field(object_, "stride")),
        count=json_int(json_field(object_, "count")),
    )


@dataclass(frozen=True, slots=True)
class TensorLayout:
    """Concrete layout for a tensor handle."""

    # the tensor element type
    element: destack._generated.program.type.TypeId
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
    destack._generated.program.type.encode_type_id(writer, value.element)
    destack._generated.mir.tree.type.encode_tensor_format(writer, value.format)
    destack._generated.mir.tree.type.encode_tensor_sharding(writer, value.sharding)
    writer.write_unsigned(value.rank)


def decode_tensor_layout(reader: BinaryReader) -> TensorLayout:
    """Decode one TensorLayout."""
    element = destack._generated.program.type.decode_type_id(reader)
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
        "element": destack._generated.program.type.to_json_type_id(value.element),
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
        element=destack._generated.program.type.from_json_type_id(
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
    element: destack._generated.program.type.TypeId
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
    destack._generated.program.type.encode_type_id(writer, value.element)
    destack._generated.mir.tree.type.encode_tensor_view_format(writer, value.format)
    destack._generated.mir.tree.type.encode_tensor_sharding(writer, value.sharding)
    writer.write_unsigned(value.rank)


def decode_tensor_view_layout(reader: BinaryReader) -> TensorViewLayout:
    """Decode one TensorViewLayout."""
    element = destack._generated.program.type.decode_type_id(reader)
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
        "element": destack._generated.program.type.to_json_type_id(value.element),
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
        element=destack._generated.program.type.from_json_type_id(
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
    ty: destack._generated.program.type.TypeId | None
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
        destack._generated.program.type.encode_type_id(writer, value.ty)
    writer.write_unsigned(value.size)
    writer.write_unsigned(value.alignment)


def decode_variant_tag_layout(reader: BinaryReader) -> VariantTagLayout:
    """Decode one VariantTagLayout."""
    ty = reader.read_option(
        lambda: destack._generated.program.type.decode_type_id(reader)
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
            else {"ty": destack._generated.program.type.to_json_type_id(value.ty)}
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
            lambda value: destack._generated.program.type.from_json_type_id(value),
        ),
        size=json_int(json_field(object_, "size")),
        alignment=json_int(json_field(object_, "alignment")),
    )


@dataclass(frozen=True, slots=True)
class VariantCaseLayout:
    """Concrete layout for one variant case."""

    # the logical case type
    ty: destack._generated.program.type.TypeId
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
    destack._generated.program.type.encode_type_id(writer, value.ty)
    encode_layout_id(writer, value.layout)


def decode_variant_case_layout(reader: BinaryReader) -> VariantCaseLayout:
    """Decode one VariantCaseLayout."""
    ty = destack._generated.program.type.decode_type_id(reader)
    layout = decode_layout_id(reader)

    return VariantCaseLayout(
        ty=ty,
        layout=layout,
    )


def to_json_variant_case_layout(value: VariantCaseLayout) -> Json:
    """Return one JSON value for one VariantCaseLayout."""
    return {
        "ty": destack._generated.program.type.to_json_type_id(value.ty),
        "layout": to_json_layout_id(value.layout),
    }


def from_json_variant_case_layout(value: Json) -> VariantCaseLayout:
    """Return one VariantCaseLayout from one JSON value."""
    object_ = json_object(value)

    return VariantCaseLayout(
        ty=destack._generated.program.type.from_json_type_id(json_field(object_, "ty")),
        layout=from_json_layout_id(json_field(object_, "layout")),
    )


@dataclass(frozen=True, slots=True)
class ObjectLayout:
    """Concrete layout for an object."""

    # byte offset of the virtual dispatch pointer when this object carries one
    dispatch_offset: int | None
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
    if value.dispatch_offset is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(value.dispatch_offset)
    writer.write_unsigned(len(value.fields))
    for item_value_fields_0 in value.fields:
        encode_layout_field(writer, item_value_fields_0)


def decode_object_layout(reader: BinaryReader) -> ObjectLayout:
    """Decode one ObjectLayout."""
    dispatch_offset = reader.read_option(lambda: reader.read_number())
    fields = [decode_layout_field(reader) for _ in range(reader.read_number())]

    return ObjectLayout(
        dispatch_offset=dispatch_offset,
        fields=fields,
    )


def to_json_object_layout(value: ObjectLayout) -> Json:
    """Return one JSON value for one ObjectLayout."""
    return {
        **(
            {}
            if value.dispatch_offset is None
            else {"dispatchOffset": value.dispatch_offset}
        ),
        "fields": [to_json_layout_field(item_0) for item_0 in value.fields],
    }


def from_json_object_layout(value: Json) -> ObjectLayout:
    """Return one ObjectLayout from one JSON value."""
    object_ = json_object(value)

    return ObjectLayout(
        dispatch_offset=json_optional(
            object_, "dispatchOffset", lambda value: json_int(value)
        ),
        fields=[
            from_json_layout_field(item_0)
            for item_0 in json_array(json_field(object_, "fields"))
        ],
    )


@dataclass(frozen=True, slots=True)
class FunctionLayout:
    """Concrete layout for one closure value."""

    # the callable signature
    signature: destack._generated.program.function.Signature
    # the captured environment type
    environment: destack._generated.program.type.TypeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_function_layout(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionLayout:
        """Decode one FunctionLayout."""
        return decode_function_layout(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_function_layout(self)

    @classmethod
    def from_json(cls, value: Json) -> FunctionLayout:
        """Return one FunctionLayout from one JSON value."""
        return from_json_function_layout(value)


def encode_function_layout(writer: BinaryWriter, value: FunctionLayout) -> None:
    """Encode one FunctionLayout."""
    destack._generated.program.function.encode_signature(writer, value.signature)
    destack._generated.program.type.encode_type_id(writer, value.environment)


def decode_function_layout(reader: BinaryReader) -> FunctionLayout:
    """Decode one FunctionLayout."""
    signature = destack._generated.program.function.decode_signature(reader)
    environment = destack._generated.program.type.decode_type_id(reader)

    return FunctionLayout(
        signature=signature,
        environment=environment,
    )


def to_json_function_layout(value: FunctionLayout) -> Json:
    """Return one JSON value for one FunctionLayout."""
    return {
        "signature": destack._generated.program.function.to_json_signature(
            value.signature
        ),
        "environment": destack._generated.program.type.to_json_type_id(
            value.environment
        ),
    }


def from_json_function_layout(value: Json) -> FunctionLayout:
    """Return one FunctionLayout from one JSON value."""
    object_ = json_object(value)

    return FunctionLayout(
        signature=destack._generated.program.function.from_json_signature(
            json_field(object_, "signature")
        ),
        environment=destack._generated.program.type.from_json_type_id(
            json_field(object_, "environment")
        ),
    )


@dataclass(frozen=True, slots=True)
class NewtypeLayout:
    """Concrete layout for a nominal newtype."""

    # the backing type
    backing_type: destack._generated.program.type.TypeId
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
    destack._generated.program.type.encode_type_id(writer, value.backing_type)
    encode_layout_id(writer, value.backing_layout)


def decode_newtype_layout(reader: BinaryReader) -> NewtypeLayout:
    """Decode one NewtypeLayout."""
    backing_type = destack._generated.program.type.decode_type_id(reader)
    backing_layout = decode_layout_id(reader)

    return NewtypeLayout(
        backing_type=backing_type,
        backing_layout=backing_layout,
    )


def to_json_newtype_layout(value: NewtypeLayout) -> Json:
    """Return one JSON value for one NewtypeLayout."""
    return {
        "backingType": destack._generated.program.type.to_json_type_id(
            value.backing_type
        ),
        "backingLayout": to_json_layout_id(value.backing_layout),
    }


def from_json_newtype_layout(value: Json) -> NewtypeLayout:
    """Return one NewtypeLayout from one JSON value."""
    object_ = json_object(value)

    return NewtypeLayout(
        backing_type=destack._generated.program.type.from_json_type_id(
            json_field(object_, "backingType")
        ),
        backing_layout=from_json_layout_id(json_field(object_, "backingLayout")),
    )


@dataclass(frozen=True, slots=True)
class CellLayoutVoid:
    """Void value."""

    kind: typing.Literal["void"] = "void"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_cell_layout(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_cell_layout(self)


@dataclass(frozen=True, slots=True)
class CellLayoutBoolean:
    """Boolean value."""

    kind: typing.Literal["boolean"] = "boolean"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_cell_layout(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_cell_layout(self)


@dataclass(frozen=True, slots=True)
class CellLayoutInt:
    """Signed integer value."""

    width: int
    kind: typing.Literal["int"] = "int"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_cell_layout(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_cell_layout(self)


@dataclass(frozen=True, slots=True)
class CellLayoutUint:
    """Unsigned integer value."""

    width: int
    kind: typing.Literal["uint"] = "uint"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_cell_layout(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_cell_layout(self)


@dataclass(frozen=True, slots=True)
class CellLayoutFloat16:
    """Float16 value."""

    kind: typing.Literal["float16"] = "float16"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_cell_layout(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_cell_layout(self)


@dataclass(frozen=True, slots=True)
class CellLayoutBfloat16:
    """BF16 value."""

    kind: typing.Literal["bfloat16"] = "bfloat16"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_cell_layout(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_cell_layout(self)


@dataclass(frozen=True, slots=True)
class CellLayoutFloat32:
    """Float32 value."""

    kind: typing.Literal["float32"] = "float32"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_cell_layout(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_cell_layout(self)


@dataclass(frozen=True, slots=True)
class CellLayoutFloat64:
    """Float64 value."""

    kind: typing.Literal["float64"] = "float64"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_cell_layout(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_cell_layout(self)


@dataclass(frozen=True, slots=True)
class CellLayoutHeapReference:
    """Local heap reference."""

    kind: typing.Literal["heapReference"] = "heapReference"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_cell_layout(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_cell_layout(self)


@dataclass(frozen=True, slots=True)
class CellLayoutSharedHeapReference:
    """Shared heap reference."""

    kind: typing.Literal["sharedHeapReference"] = "sharedHeapReference"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_cell_layout(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_cell_layout(self)


@dataclass(frozen=True, slots=True)
class CellLayoutAddress:
    """Native address."""

    kind: typing.Literal["address"] = "address"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_cell_layout(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_cell_layout(self)


@dataclass(frozen=True, slots=True)
class CellLayoutStackPointer:
    """Stack pointer."""

    kind: typing.Literal["stackPointer"] = "stackPointer"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_cell_layout(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_cell_layout(self)


@dataclass(frozen=True, slots=True)
class CellLayoutFramePointer:
    """Frame pointer."""

    kind: typing.Literal["framePointer"] = "framePointer"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_cell_layout(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_cell_layout(self)


@dataclass(frozen=True, slots=True)
class CellLayoutGlobalAddress:
    """Global address."""

    kind: typing.Literal["globalAddress"] = "globalAddress"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_cell_layout(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_cell_layout(self)


@dataclass(frozen=True, slots=True)
class CellLayoutFunctionPointer:
    """Function pointer."""

    kind: typing.Literal["functionPointer"] = "functionPointer"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_cell_layout(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_cell_layout(self)


"""One-cell storage layout for a scalar or pointer value."""
CellLayout: typing.TypeAlias = (
    CellLayoutVoid
    | CellLayoutBoolean
    | CellLayoutInt
    | CellLayoutUint
    | CellLayoutFloat16
    | CellLayoutBfloat16
    | CellLayoutFloat32
    | CellLayoutFloat64
    | CellLayoutHeapReference
    | CellLayoutSharedHeapReference
    | CellLayoutAddress
    | CellLayoutStackPointer
    | CellLayoutFramePointer
    | CellLayoutGlobalAddress
    | CellLayoutFunctionPointer
)


def encode_cell_layout(writer: BinaryWriter, value: CellLayout) -> None:
    """Encode one CellLayout."""
    if value.kind == "void":
        writer.write_unsigned(0)
    elif value.kind == "boolean":
        writer.write_unsigned(1)
    elif value.kind == "int":
        writer.write_unsigned(2)
        writer.write_byte(value.width)
    elif value.kind == "uint":
        writer.write_unsigned(3)
        writer.write_byte(value.width)
    elif value.kind == "float16":
        writer.write_unsigned(4)
    elif value.kind == "bfloat16":
        writer.write_unsigned(5)
    elif value.kind == "float32":
        writer.write_unsigned(6)
    elif value.kind == "float64":
        writer.write_unsigned(7)
    elif value.kind == "heapReference":
        writer.write_unsigned(8)
    elif value.kind == "sharedHeapReference":
        writer.write_unsigned(9)
    elif value.kind == "address":
        writer.write_unsigned(10)
    elif value.kind == "stackPointer":
        writer.write_unsigned(11)
    elif value.kind == "framePointer":
        writer.write_unsigned(12)
    elif value.kind == "globalAddress":
        writer.write_unsigned(13)
    elif value.kind == "functionPointer":
        writer.write_unsigned(14)
    else:
        raise SerdeError("unknown enum variant")


def decode_cell_layout(reader: BinaryReader) -> CellLayout:
    """Decode one CellLayout."""
    variant = reader.read_number()

    if variant == 0:
        return CellLayoutVoid()
    elif variant == 1:
        return CellLayoutBoolean()
    elif variant == 2:
        width = reader.read_byte()

        return CellLayoutInt(
            width=width,
        )
    elif variant == 3:
        width = reader.read_byte()

        return CellLayoutUint(
            width=width,
        )
    elif variant == 4:
        return CellLayoutFloat16()
    elif variant == 5:
        return CellLayoutBfloat16()
    elif variant == 6:
        return CellLayoutFloat32()
    elif variant == 7:
        return CellLayoutFloat64()
    elif variant == 8:
        return CellLayoutHeapReference()
    elif variant == 9:
        return CellLayoutSharedHeapReference()
    elif variant == 10:
        return CellLayoutAddress()
    elif variant == 11:
        return CellLayoutStackPointer()
    elif variant == 12:
        return CellLayoutFramePointer()
    elif variant == 13:
        return CellLayoutGlobalAddress()
    elif variant == 14:
        return CellLayoutFunctionPointer()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_cell_layout(value: CellLayout) -> Json:
    """Return one JSON value for one CellLayout."""
    if value.kind == "void":
        return {
            "kind": "void",
        }
    elif value.kind == "boolean":
        return {
            "kind": "boolean",
        }
    elif value.kind == "int":
        return {
            "kind": "int",
            "width": value.width,
        }
    elif value.kind == "uint":
        return {
            "kind": "uint",
            "width": value.width,
        }
    elif value.kind == "float16":
        return {
            "kind": "float16",
        }
    elif value.kind == "bfloat16":
        return {
            "kind": "bfloat16",
        }
    elif value.kind == "float32":
        return {
            "kind": "float32",
        }
    elif value.kind == "float64":
        return {
            "kind": "float64",
        }
    elif value.kind == "heapReference":
        return {
            "kind": "heapReference",
        }
    elif value.kind == "sharedHeapReference":
        return {
            "kind": "sharedHeapReference",
        }
    elif value.kind == "address":
        return {
            "kind": "address",
        }
    elif value.kind == "stackPointer":
        return {
            "kind": "stackPointer",
        }
    elif value.kind == "framePointer":
        return {
            "kind": "framePointer",
        }
    elif value.kind == "globalAddress":
        return {
            "kind": "globalAddress",
        }
    elif value.kind == "functionPointer":
        return {
            "kind": "functionPointer",
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_cell_layout(value: Json) -> CellLayout:
    """Return one CellLayout from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "void":
        return CellLayoutVoid()
    elif kind == "boolean":
        return CellLayoutBoolean()
    elif kind == "int":
        return CellLayoutInt(
            width=json_int(json_field(object_, "width")),
        )
    elif kind == "uint":
        return CellLayoutUint(
            width=json_int(json_field(object_, "width")),
        )
    elif kind == "float16":
        return CellLayoutFloat16()
    elif kind == "bfloat16":
        return CellLayoutBfloat16()
    elif kind == "float32":
        return CellLayoutFloat32()
    elif kind == "float64":
        return CellLayoutFloat64()
    elif kind == "heapReference":
        return CellLayoutHeapReference()
    elif kind == "sharedHeapReference":
        return CellLayoutSharedHeapReference()
    elif kind == "address":
        return CellLayoutAddress()
    elif kind == "stackPointer":
        return CellLayoutStackPointer()
    elif kind == "framePointer":
        return CellLayoutFramePointer()
    elif kind == "globalAddress":
        return CellLayoutGlobalAddress()
    elif kind == "functionPointer":
        return CellLayoutFunctionPointer()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


"""Runtime address space for addressable values."""
AddressSpace: typing.TypeAlias = (
    typing.Literal["local"]
    | typing.Literal["shared"]
    | typing.Literal["raw"]
    | typing.Literal["stack"]
    | typing.Literal["frame"]
    | typing.Literal["static"]
)


def encode_address_space(writer: BinaryWriter, value: AddressSpace) -> None:
    """Encode one AddressSpace."""
    if value == "local":
        writer.write_unsigned(0)
    elif value == "shared":
        writer.write_unsigned(1)
    elif value == "raw":
        writer.write_unsigned(2)
    elif value == "stack":
        writer.write_unsigned(3)
    elif value == "frame":
        writer.write_unsigned(4)
    elif value == "static":
        writer.write_unsigned(5)
    else:
        raise SerdeError("unknown enum variant")


def decode_address_space(reader: BinaryReader) -> AddressSpace:
    """Decode one AddressSpace."""
    variant = reader.read_number()

    if variant == 0:
        return "local"
    elif variant == 1:
        return "shared"
    elif variant == 2:
        return "raw"
    elif variant == 3:
        return "stack"
    elif variant == 4:
        return "frame"
    elif variant == 5:
        return "static"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_address_space(value: AddressSpace) -> Json:
    """Return one JSON value for one AddressSpace."""
    return value


def from_json_address_space(value: Json) -> AddressSpace:
    """Return one AddressSpace from one JSON value."""
    variant = json_string(value)

    if variant == "local":
        return "local"
    elif variant == "shared":
        return "shared"
    elif variant == "raw":
        return "raw"
    elif variant == "stack":
        return "stack"
    elif variant == "frame":
        return "frame"
    elif variant == "static":
        return "static"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "LayoutId",
    "encode_layout_id",
    "decode_layout_id",
    "to_json_layout_id",
    "from_json_layout_id",
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
    "LayoutShapeReference",
    "LayoutShapeFunctionPointer",
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
    "ScalarFormat",
    "encode_scalar_format",
    "decode_scalar_format",
    "to_json_scalar_format",
    "from_json_scalar_format",
    "ScalarFormatInt",
    "ScalarFormatFloat",
    "ScalarFormatBoolean",
    "ReferenceLayout",
    "encode_reference_layout",
    "decode_reference_layout",
    "to_json_reference_layout",
    "from_json_reference_layout",
    "ReferenceFlags",
    "encode_reference_flags",
    "decode_reference_flags",
    "to_json_reference_flags",
    "from_json_reference_flags",
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
    "SliceLayout",
    "encode_slice_layout",
    "decode_slice_layout",
    "to_json_slice_layout",
    "from_json_slice_layout",
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
    "ObjectLayout",
    "encode_object_layout",
    "decode_object_layout",
    "to_json_object_layout",
    "from_json_object_layout",
    "FunctionLayout",
    "encode_function_layout",
    "decode_function_layout",
    "to_json_function_layout",
    "from_json_function_layout",
    "NewtypeLayout",
    "encode_newtype_layout",
    "decode_newtype_layout",
    "to_json_newtype_layout",
    "from_json_newtype_layout",
    "CellLayout",
    "encode_cell_layout",
    "decode_cell_layout",
    "to_json_cell_layout",
    "from_json_cell_layout",
    "CellLayoutVoid",
    "CellLayoutBoolean",
    "CellLayoutInt",
    "CellLayoutUint",
    "CellLayoutFloat16",
    "CellLayoutBfloat16",
    "CellLayoutFloat32",
    "CellLayoutFloat64",
    "CellLayoutHeapReference",
    "CellLayoutSharedHeapReference",
    "CellLayoutAddress",
    "CellLayoutStackPointer",
    "CellLayoutFramePointer",
    "CellLayoutGlobalAddress",
    "CellLayoutFunctionPointer",
    "AddressSpace",
    "encode_address_space",
    "decode_address_space",
    "to_json_address_space",
    "from_json_address_space",
]
