# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.mir.tree.type
import destack._generated.program.type
import destack._generated.program.vm.meta

@dataclass(frozen=True, slots=True)
class CellLayoutVoid:
    """Void value."""

    kind: typing.Literal["void"] = "void"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CellLayoutBool:
    """Boolean value."""

    kind: typing.Literal["bool"] = "bool"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CellLayoutInt:
    """Signed integer value."""

    width: int
    kind: typing.Literal["int"] = "int"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CellLayoutUint:
    """Unsigned integer value."""

    width: int
    kind: typing.Literal["uint"] = "uint"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CellLayoutFloat16:
    """Float16 value."""

    kind: typing.Literal["float16"] = "float16"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CellLayoutBfloat16:
    """BF16 value."""

    kind: typing.Literal["bfloat16"] = "bfloat16"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CellLayoutFloat32:
    """Float32 value."""

    kind: typing.Literal["float32"] = "float32"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CellLayoutFloat64:
    """Float64 value."""

    kind: typing.Literal["float64"] = "float64"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CellLayoutHeapReference:
    """Local heap reference."""

    kind: typing.Literal["heapReference"] = "heapReference"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CellLayoutSharedHeapReference:
    """Shared heap reference."""

    kind: typing.Literal["sharedHeapReference"] = "sharedHeapReference"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CellLayoutAddress:
    """Native address."""

    kind: typing.Literal["address"] = "address"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CellLayoutStackPointer:
    """Stack pointer."""

    kind: typing.Literal["stackPointer"] = "stackPointer"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CellLayoutFramePointer:
    """Frame pointer."""

    kind: typing.Literal["framePointer"] = "framePointer"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CellLayoutStaticAddress:
    """Static address."""

    kind: typing.Literal["staticAddress"] = "staticAddress"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CellLayoutFunctionPointer:
    """Function pointer."""

    kind: typing.Literal["functionPointer"] = "functionPointer"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

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

def encode_cell_layout(writer: BinaryWriter, value: CellLayout) -> None: ...
def decode_cell_layout(reader: BinaryReader) -> CellLayout: ...
def to_json_cell_layout(value: CellLayout) -> Json: ...
def from_json_cell_layout(value: Json) -> CellLayout: ...

@dataclass(frozen=True, slots=True)
class ScalarLayoutInt:
    """Signed or unsigned integers with a bit width."""

    # the bit width
    width: int
    # whether the integer is signed
    is_signed: bool
    kind: typing.Literal["int"] = "int"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ScalarLayoutFloat:
    """Floating-point values with a concrete format."""

    # the concrete float format
    format: destack._generated.mir.tree.type.FloatType
    kind: typing.Literal["float"] = "float"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ScalarLayoutBool:
    """Boolean values."""

    kind: typing.Literal["bool"] = "bool"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Scalar value shape for typed vector and tensor operations."""
ScalarLayout: typing.TypeAlias = ScalarLayoutInt | ScalarLayoutFloat | ScalarLayoutBool

def encode_scalar_layout(writer: BinaryWriter, value: ScalarLayout) -> None: ...
def decode_scalar_layout(reader: BinaryReader) -> ScalarLayout: ...
def to_json_scalar_layout(value: ScalarLayout) -> Json: ...
def from_json_scalar_layout(value: Json) -> ScalarLayout: ...

@dataclass(frozen=True, slots=True)
class ValueShapeVoid:
    """Void value."""

    kind: typing.Literal["void"] = "void"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ValueShapeBool:
    """Boolean value."""

    kind: typing.Literal["bool"] = "bool"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ValueShapeInt:
    """Signed or unsigned integer with width."""

    width: int
    signed: bool
    kind: typing.Literal["int"] = "int"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ValueShapeFloat:
    """Floating point value with format."""

    format: destack._generated.mir.tree.type.FloatType
    kind: typing.Literal["float"] = "float"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ValueShapeChar:
    """Unicode character value."""

    kind: typing.Literal["char"] = "char"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ValueShapePointer:
    """Addressable value with pointee type."""

    pointee: destack._generated.program.type.TypeId
    address_space: AddressSpace
    reference: destack._generated.program.vm.meta.ReferenceMeta
    kind: typing.Literal["pointer"] = "pointer"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ValueShapeFunctionPointer:
    """Function pointer value with result type."""

    result: destack._generated.program.type.TypeId
    kind: typing.Literal["functionPointer"] = "functionPointer"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ValueShapeAggregate:
    """Aggregate value materialized in frame storage."""

    ty: destack._generated.program.type.TypeId
    kind: typing.Literal["aggregate"] = "aggregate"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ValueShapeArray:
    """Fixed-size array value with element type."""

    element: destack._generated.program.type.TypeId
    length: int
    kind: typing.Literal["array"] = "array"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

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

def encode_value_shape(writer: BinaryWriter, value: ValueShape) -> None: ...
def decode_value_shape(reader: BinaryReader) -> ValueShape: ...
def to_json_value_shape(value: ValueShape) -> Json: ...
def from_json_value_shape(value: Json) -> ValueShape: ...

"""Runtime address space for addressable values."""
AddressSpace: typing.TypeAlias = (
    typing.Literal["local"]
    | typing.Literal["shared"]
    | typing.Literal["raw"]
    | typing.Literal["stack"]
    | typing.Literal["frame"]
    | typing.Literal["static"]
)

def encode_address_space(writer: BinaryWriter, value: AddressSpace) -> None: ...
def decode_address_space(reader: BinaryReader) -> AddressSpace: ...
def to_json_address_space(value: AddressSpace) -> Json: ...
def from_json_address_space(value: Json) -> AddressSpace: ...

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
