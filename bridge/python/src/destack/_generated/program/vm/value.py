# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_bool,
    json_field,
    json_int,
    json_object,
    json_string,
)

import destack._generated.mir.tree.type
import destack._generated.program.type
import destack._generated.program.vm.meta


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
class CellLayoutBool:
    """Boolean value."""

    kind: typing.Literal["bool"] = "bool"

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
class CellLayoutStaticAddress:
    """Static address."""

    kind: typing.Literal["staticAddress"] = "staticAddress"

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


"""Native cell layout for one load or store."""
CellLayout: typing.TypeAlias = (
    CellLayoutVoid
    | CellLayoutBool
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
    | CellLayoutStaticAddress
    | CellLayoutFunctionPointer
)


def encode_cell_layout(writer: BinaryWriter, value: CellLayout) -> None:
    """Encode one CellLayout."""
    if value.kind == "void":
        writer.write_unsigned(0)
    elif value.kind == "bool":
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
    elif value.kind == "staticAddress":
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
        return CellLayoutBool()
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
        return CellLayoutStaticAddress()
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
    elif value.kind == "bool":
        return {
            "kind": "bool",
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
    elif value.kind == "staticAddress":
        return {
            "kind": "staticAddress",
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
    elif kind == "bool":
        return CellLayoutBool()
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
    elif kind == "staticAddress":
        return CellLayoutStaticAddress()
    elif kind == "functionPointer":
        return CellLayoutFunctionPointer()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class ScalarLayoutInt:
    """Signed or unsigned integers with a bit width."""

    # the bit width
    width: int
    # whether the integer is signed
    is_signed: bool
    kind: typing.Literal["int"] = "int"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_scalar_layout(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_scalar_layout(self)


@dataclass(frozen=True, slots=True)
class ScalarLayoutFloat:
    """Floating-point values with a concrete format."""

    # the concrete float format
    format: destack._generated.mir.tree.type.FloatType
    kind: typing.Literal["float"] = "float"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_scalar_layout(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_scalar_layout(self)


@dataclass(frozen=True, slots=True)
class ScalarLayoutBool:
    """Boolean values."""

    kind: typing.Literal["bool"] = "bool"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_scalar_layout(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_scalar_layout(self)


"""Scalar value shape for typed vector and tensor operations."""
ScalarLayout: typing.TypeAlias = ScalarLayoutInt | ScalarLayoutFloat | ScalarLayoutBool


def encode_scalar_layout(writer: BinaryWriter, value: ScalarLayout) -> None:
    """Encode one ScalarLayout."""
    if value.kind == "int":
        writer.write_unsigned(0)
        writer.write_unsigned(value.width)
        writer.write_bool(value.is_signed)
    elif value.kind == "float":
        writer.write_unsigned(1)
        destack._generated.mir.tree.type.encode_float_type(writer, value.format)
    elif value.kind == "bool":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_scalar_layout(reader: BinaryReader) -> ScalarLayout:
    """Decode one ScalarLayout."""
    variant = reader.read_number()

    if variant == 0:
        width = reader.read_number()
        is_signed = reader.read_bool()

        return ScalarLayoutInt(
            width=width,
            is_signed=is_signed,
        )
    elif variant == 1:
        format = destack._generated.mir.tree.type.decode_float_type(reader)

        return ScalarLayoutFloat(
            format=format,
        )
    elif variant == 2:
        return ScalarLayoutBool()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_scalar_layout(value: ScalarLayout) -> Json:
    """Return one JSON value for one ScalarLayout."""
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
    elif value.kind == "bool":
        return {
            "kind": "bool",
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_scalar_layout(value: Json) -> ScalarLayout:
    """Return one ScalarLayout from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "int":
        return ScalarLayoutInt(
            width=json_int(json_field(object_, "width")),
            is_signed=json_bool(json_field(object_, "isSigned")),
        )
    elif kind == "float":
        return ScalarLayoutFloat(
            format=destack._generated.mir.tree.type.from_json_float_type(
                json_field(object_, "format")
            ),
        )
    elif kind == "bool":
        return ScalarLayoutBool()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class ValueShapeVoid:
    """Void value."""

    kind: typing.Literal["void"] = "void"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_value_shape(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_value_shape(self)


@dataclass(frozen=True, slots=True)
class ValueShapeBool:
    """Boolean value."""

    kind: typing.Literal["bool"] = "bool"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_value_shape(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_value_shape(self)


@dataclass(frozen=True, slots=True)
class ValueShapeInt:
    """Signed or unsigned integer with width."""

    width: int
    signed: bool
    kind: typing.Literal["int"] = "int"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_value_shape(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_value_shape(self)


@dataclass(frozen=True, slots=True)
class ValueShapeFloat:
    """Floating point value with format."""

    format: destack._generated.mir.tree.type.FloatType
    kind: typing.Literal["float"] = "float"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_value_shape(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_value_shape(self)


@dataclass(frozen=True, slots=True)
class ValueShapeChar:
    """Unicode character value."""

    kind: typing.Literal["char"] = "char"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_value_shape(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_value_shape(self)


@dataclass(frozen=True, slots=True)
class ValueShapePointer:
    """Addressable value with pointee type."""

    pointee: destack._generated.program.type.TypeId
    address_space: AddressSpace
    reference: destack._generated.program.vm.meta.ReferenceMeta
    kind: typing.Literal["pointer"] = "pointer"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_value_shape(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_value_shape(self)


@dataclass(frozen=True, slots=True)
class ValueShapeFunctionPointer:
    """Function pointer value with result type."""

    result: destack._generated.program.type.TypeId
    kind: typing.Literal["functionPointer"] = "functionPointer"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_value_shape(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_value_shape(self)


@dataclass(frozen=True, slots=True)
class ValueShapeAggregate:
    """Aggregate value materialized in frame storage."""

    ty: destack._generated.program.type.TypeId
    kind: typing.Literal["aggregate"] = "aggregate"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_value_shape(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_value_shape(self)


@dataclass(frozen=True, slots=True)
class ValueShapeArray:
    """Fixed-size array value with element type."""

    element: destack._generated.program.type.TypeId
    length: int
    kind: typing.Literal["array"] = "array"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_value_shape(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_value_shape(self)


"""Runtime value shape used for op selection."""
ValueShape: typing.TypeAlias = (
    ValueShapeVoid
    | ValueShapeBool
    | ValueShapeInt
    | ValueShapeFloat
    | ValueShapeChar
    | ValueShapePointer
    | ValueShapeFunctionPointer
    | ValueShapeAggregate
    | ValueShapeArray
)


def encode_value_shape(writer: BinaryWriter, value: ValueShape) -> None:
    """Encode one ValueShape."""
    if value.kind == "void":
        writer.write_unsigned(0)
    elif value.kind == "bool":
        writer.write_unsigned(1)
    elif value.kind == "int":
        writer.write_unsigned(2)
        writer.write_unsigned(value.width)
        writer.write_bool(value.signed)
    elif value.kind == "float":
        writer.write_unsigned(3)
        destack._generated.mir.tree.type.encode_float_type(writer, value.format)
    elif value.kind == "char":
        writer.write_unsigned(4)
    elif value.kind == "pointer":
        writer.write_unsigned(5)
        destack._generated.program.type.encode_type_id(writer, value.pointee)
        encode_address_space(writer, value.address_space)
        destack._generated.program.vm.meta.encode_reference_meta(
            writer, value.reference
        )
    elif value.kind == "functionPointer":
        writer.write_unsigned(6)
        destack._generated.program.type.encode_type_id(writer, value.result)
    elif value.kind == "aggregate":
        writer.write_unsigned(7)
        destack._generated.program.type.encode_type_id(writer, value.ty)
    elif value.kind == "array":
        writer.write_unsigned(8)
        destack._generated.program.type.encode_type_id(writer, value.element)
        writer.write_unsigned(value.length)
    else:
        raise SerdeError("unknown enum variant")


def decode_value_shape(reader: BinaryReader) -> ValueShape:
    """Decode one ValueShape."""
    variant = reader.read_number()

    if variant == 0:
        return ValueShapeVoid()
    elif variant == 1:
        return ValueShapeBool()
    elif variant == 2:
        width = reader.read_number()
        signed = reader.read_bool()

        return ValueShapeInt(
            width=width,
            signed=signed,
        )
    elif variant == 3:
        format = destack._generated.mir.tree.type.decode_float_type(reader)

        return ValueShapeFloat(
            format=format,
        )
    elif variant == 4:
        return ValueShapeChar()
    elif variant == 5:
        pointee = destack._generated.program.type.decode_type_id(reader)
        address_space = decode_address_space(reader)
        reference = destack._generated.program.vm.meta.decode_reference_meta(reader)

        return ValueShapePointer(
            pointee=pointee,
            address_space=address_space,
            reference=reference,
        )
    elif variant == 6:
        result = destack._generated.program.type.decode_type_id(reader)

        return ValueShapeFunctionPointer(
            result=result,
        )
    elif variant == 7:
        ty = destack._generated.program.type.decode_type_id(reader)

        return ValueShapeAggregate(
            ty=ty,
        )
    elif variant == 8:
        element = destack._generated.program.type.decode_type_id(reader)
        length = reader.read_number()

        return ValueShapeArray(
            element=element,
            length=length,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_value_shape(value: ValueShape) -> Json:
    """Return one JSON value for one ValueShape."""
    if value.kind == "void":
        return {
            "kind": "void",
        }
    elif value.kind == "bool":
        return {
            "kind": "bool",
        }
    elif value.kind == "int":
        return {
            "kind": "int",
            "width": value.width,
            "signed": value.signed,
        }
    elif value.kind == "float":
        return {
            "kind": "float",
            "format": destack._generated.mir.tree.type.to_json_float_type(value.format),
        }
    elif value.kind == "char":
        return {
            "kind": "char",
        }
    elif value.kind == "pointer":
        return {
            "kind": "pointer",
            "pointee": destack._generated.program.type.to_json_type_id(value.pointee),
            "addressSpace": to_json_address_space(value.address_space),
            "reference": destack._generated.program.vm.meta.to_json_reference_meta(
                value.reference
            ),
        }
    elif value.kind == "functionPointer":
        return {
            "kind": "functionPointer",
            "result": destack._generated.program.type.to_json_type_id(value.result),
        }
    elif value.kind == "aggregate":
        return {
            "kind": "aggregate",
            "ty": destack._generated.program.type.to_json_type_id(value.ty),
        }
    elif value.kind == "array":
        return {
            "kind": "array",
            "element": destack._generated.program.type.to_json_type_id(value.element),
            "length": value.length,
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_value_shape(value: Json) -> ValueShape:
    """Return one ValueShape from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "void":
        return ValueShapeVoid()
    elif kind == "bool":
        return ValueShapeBool()
    elif kind == "int":
        return ValueShapeInt(
            width=json_int(json_field(object_, "width")),
            signed=json_bool(json_field(object_, "signed")),
        )
    elif kind == "float":
        return ValueShapeFloat(
            format=destack._generated.mir.tree.type.from_json_float_type(
                json_field(object_, "format")
            ),
        )
    elif kind == "char":
        return ValueShapeChar()
    elif kind == "pointer":
        return ValueShapePointer(
            pointee=destack._generated.program.type.from_json_type_id(
                json_field(object_, "pointee")
            ),
            address_space=from_json_address_space(json_field(object_, "addressSpace")),
            reference=destack._generated.program.vm.meta.from_json_reference_meta(
                json_field(object_, "reference")
            ),
        )
    elif kind == "functionPointer":
        return ValueShapeFunctionPointer(
            result=destack._generated.program.type.from_json_type_id(
                json_field(object_, "result")
            ),
        )
    elif kind == "aggregate":
        return ValueShapeAggregate(
            ty=destack._generated.program.type.from_json_type_id(
                json_field(object_, "ty")
            ),
        )
    elif kind == "array":
        return ValueShapeArray(
            element=destack._generated.program.type.from_json_type_id(
                json_field(object_, "element")
            ),
            length=json_int(json_field(object_, "length")),
        )
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
    "CellLayout",
    "encode_cell_layout",
    "decode_cell_layout",
    "to_json_cell_layout",
    "from_json_cell_layout",
    "CellLayoutVoid",
    "CellLayoutBool",
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
    "CellLayoutStaticAddress",
    "CellLayoutFunctionPointer",
    "ScalarLayout",
    "encode_scalar_layout",
    "decode_scalar_layout",
    "to_json_scalar_layout",
    "from_json_scalar_layout",
    "ScalarLayoutInt",
    "ScalarLayoutFloat",
    "ScalarLayoutBool",
    "ValueShape",
    "encode_value_shape",
    "decode_value_shape",
    "to_json_value_shape",
    "from_json_value_shape",
    "ValueShapeVoid",
    "ValueShapeBool",
    "ValueShapeInt",
    "ValueShapeFloat",
    "ValueShapeChar",
    "ValueShapePointer",
    "ValueShapeFunctionPointer",
    "ValueShapeAggregate",
    "ValueShapeArray",
    "AddressSpace",
    "encode_address_space",
    "decode_address_space",
    "to_json_address_space",
    "from_json_address_space",
]
