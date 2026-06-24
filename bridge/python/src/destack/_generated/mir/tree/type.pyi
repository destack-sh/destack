# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.string
import destack._generated.mir.tree.constant
import destack._generated.mir.tree.lifetime
import destack._generated.mir.tree.node
import destack._generated.mir.tree.parameter

"""A concrete MIR floating-point type."""
FloatType: typing.TypeAlias = (
    typing.Literal["float16"]
    | typing.Literal["bfloat16"]
    | typing.Literal["float32"]
    | typing.Literal["float64"]
)

def encode_float_type(writer: BinaryWriter, value: FloatType) -> None: ...
def decode_float_type(reader: BinaryReader) -> FloatType: ...
def to_json_float_type(value: FloatType) -> Json: ...
def from_json_float_type(value: Json) -> FloatType: ...

"""Mutability of a storage binding."""
Mutability: typing.TypeAlias = typing.Literal["immutable"] | typing.Literal["mutable"]

def encode_mutability(writer: BinaryWriter, value: Mutability) -> None: ...
def decode_mutability(reader: BinaryReader) -> Mutability: ...
def to_json_mutability(value: Mutability) -> Json: ...
def from_json_mutability(value: Json) -> Mutability: ...

"""Kind of reference in MIR."""
ReferenceKind: typing.TypeAlias = (
    typing.Literal["managed"]
    | typing.Literal["unique"]
    | typing.Literal["borrowed"]
    | typing.Literal["raw"]
)

def encode_reference_kind(writer: BinaryWriter, value: ReferenceKind) -> None: ...
def decode_reference_kind(reader: BinaryReader) -> ReferenceKind: ...
def to_json_reference_kind(value: ReferenceKind) -> Json: ...
def from_json_reference_kind(value: Json) -> ReferenceKind: ...

"""Space for a reference."""
Space: typing.TypeAlias = (
    typing.Literal["local"]
    | typing.Literal["shared"]
    | typing.Literal["frame"]
    | typing.Literal["static"]
)

def encode_space(writer: BinaryWriter, value: Space) -> None: ...
def decode_space(reader: BinaryReader) -> Space: ...
def to_json_space(value: Space) -> Json: ...
def from_json_space(value: Json) -> Space: ...

"""Access exposed by a reference-like value."""
Access: typing.TypeAlias = (
    typing.Literal["readonly"] | typing.Literal["mutable"] | typing.Literal["exclusive"]
)

def encode_access(writer: BinaryWriter, value: Access) -> None: ...
def decode_access(reader: BinaryReader) -> Access: ...
def to_json_access(value: Access) -> Json: ...
def from_json_access(value: Json) -> Access: ...

"""Invalid-value niches carried by pointer-like values."""
Nullability: typing.TypeAlias = (
    typing.Literal["none"]
    | typing.Literal["null"]
    | typing.Literal["undefined"]
    | typing.Literal["nullOrUndefined"]
)

def encode_nullability(writer: BinaryWriter, value: Nullability) -> None: ...
def decode_nullability(reader: BinaryReader) -> Nullability: ...
def to_json_nullability(value: Nullability) -> Json: ...
def from_json_nullability(value: Json) -> Nullability: ...

"""Copyability of a type."""
Copy: typing.TypeAlias = typing.Literal["yes"] | typing.Literal["no"]

def encode_copy(writer: BinaryWriter, value: Copy) -> None: ...
def decode_copy(reader: BinaryReader) -> Copy: ...
def to_json_copy(value: Copy) -> Json: ...
def from_json_copy(value: Json) -> Copy: ...

@dataclass(frozen=True, slots=True)
class VariantCase:
    """One physical tagged sum case."""

    # the tag constant selecting this case
    tag: destack._generated.mir.tree.constant.Constant
    # the logical payload type
    ty: destack._generated.mir.tree.node.LocalNodeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> VariantCase: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> VariantCase: ...

def encode_variant_case(writer: BinaryWriter, value: VariantCase) -> None: ...
def decode_variant_case(reader: BinaryReader) -> VariantCase: ...
def to_json_variant_case(value: VariantCase) -> Json: ...
def from_json_variant_case(value: Json) -> VariantCase: ...

@dataclass(frozen=True, slots=True)
class TensorDimensionStatic:
    """Compile time static dimension size."""

    static: int
    kind: typing.Literal["static"] = "static"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TensorDimensionSymbol:
    """Symbolic runtime dimension shared across tensors."""

    symbol: str
    kind: typing.Literal["symbol"] = "symbol"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TensorDimensionDynamic:
    """Runtime dynamic dimension size."""

    kind: typing.Literal["dynamic"] = "dynamic"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Dimension size for tensor shapes and formats."""
TensorDimension: typing.TypeAlias = (
    TensorDimensionStatic | TensorDimensionSymbol | TensorDimensionDynamic
)

def encode_tensor_dimension(writer: BinaryWriter, value: TensorDimension) -> None: ...
def decode_tensor_dimension(reader: BinaryReader) -> TensorDimension: ...
def to_json_tensor_dimension(value: TensorDimension) -> Json: ...
def from_json_tensor_dimension(value: Json) -> TensorDimension: ...

@dataclass(frozen=True, slots=True)
class TensorFormatDense:
    """Dense contiguous format."""

    # the dimension order
    order: TensorDimensionOrder
    kind: typing.Literal["dense"] = "dense"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Format for an owning tensor value."""
TensorFormat: typing.TypeAlias = TensorFormatDense

def encode_tensor_format(writer: BinaryWriter, value: TensorFormat) -> None: ...
def decode_tensor_format(reader: BinaryReader) -> TensorFormat: ...
def to_json_tensor_format(value: TensorFormat) -> Json: ...
def from_json_tensor_format(value: Json) -> TensorFormat: ...

"""Dimension order for dense tensor storage."""
TensorDimensionOrder: typing.TypeAlias = (
    typing.Literal["rowMajor"] | typing.Literal["columnMajor"]
)

def encode_tensor_dimension_order(
    writer: BinaryWriter, value: TensorDimensionOrder
) -> None: ...
def decode_tensor_dimension_order(reader: BinaryReader) -> TensorDimensionOrder: ...
def to_json_tensor_dimension_order(value: TensorDimensionOrder) -> Json: ...
def from_json_tensor_dimension_order(value: Json) -> TensorDimensionOrder: ...

@dataclass(frozen=True, slots=True)
class TensorShardingUnsharded:
    """Tensor storage is not partitioned across a mesh."""

    kind: typing.Literal["unsharded"] = "unsharded"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TensorShardingSharding:
    """Tensor storage is mapped across a mesh axis by axis."""

    # the per-axis placement descriptors
    axes: Sequence[TensorShardingAxis]
    kind: typing.Literal["sharding"] = "sharding"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Placement descriptor for tensor storage."""
TensorSharding: typing.TypeAlias = TensorShardingUnsharded | TensorShardingSharding

def encode_tensor_sharding(writer: BinaryWriter, value: TensorSharding) -> None: ...
def decode_tensor_sharding(reader: BinaryReader) -> TensorSharding: ...
def to_json_tensor_sharding(value: TensorSharding) -> Json: ...
def from_json_tensor_sharding(value: Json) -> TensorSharding: ...

@dataclass(frozen=True, slots=True)
class TensorShardingAxisShard:
    """Split one tensor axis across one mesh axis."""

    # the tensor axis being split
    axis: int
    kind: typing.Literal["shard"] = "shard"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TensorShardingAxisReplicate:
    """Replicate values across one mesh axis."""

    kind: typing.Literal["replicate"] = "replicate"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TensorShardingAxisPartial:
    """Store partial results across one mesh axis."""

    # the reduction used to combine partial values
    reduction: TensorReduction
    kind: typing.Literal["partial"] = "partial"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Per-axis placement descriptor for a sharded tensor."""
TensorShardingAxis: typing.TypeAlias = (
    TensorShardingAxisShard | TensorShardingAxisReplicate | TensorShardingAxisPartial
)

def encode_tensor_sharding_axis(
    writer: BinaryWriter, value: TensorShardingAxis
) -> None: ...
def decode_tensor_sharding_axis(reader: BinaryReader) -> TensorShardingAxis: ...
def to_json_tensor_sharding_axis(value: TensorShardingAxis) -> Json: ...
def from_json_tensor_sharding_axis(value: Json) -> TensorShardingAxis: ...

"""Reduction used when partial tensor shards are combined."""
TensorReduction: typing.TypeAlias = (
    typing.Literal["add"]
    | typing.Literal["multiply"]
    | typing.Literal["minimum"]
    | typing.Literal["maximum"]
    | typing.Literal["and"]
    | typing.Literal["or"]
)

def encode_tensor_reduction(writer: BinaryWriter, value: TensorReduction) -> None: ...
def decode_tensor_reduction(reader: BinaryReader) -> TensorReduction: ...
def to_json_tensor_reduction(value: TensorReduction) -> Json: ...
def from_json_tensor_reduction(value: Json) -> TensorReduction: ...

@dataclass(frozen=True, slots=True)
class TensorViewFormatDense:
    """Dense contiguous view."""

    # the dimension order
    order: TensorDimensionOrder
    kind: typing.Literal["dense"] = "dense"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TensorViewFormatStrided:
    """Explicit strided view."""

    kind: typing.Literal["strided"] = "strided"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Format descriptor for a tensor view."""
TensorViewFormat: typing.TypeAlias = TensorViewFormatDense | TensorViewFormatStrided

def encode_tensor_view_format(
    writer: BinaryWriter, value: TensorViewFormat
) -> None: ...
def decode_tensor_view_format(reader: BinaryReader) -> TensorViewFormat: ...
def to_json_tensor_view_format(value: TensorViewFormat) -> Json: ...
def from_json_tensor_view_format(value: Json) -> TensorViewFormat: ...

@dataclass(frozen=True, slots=True)
class TypeAlias:
    """A named type alias in MIR text format."""

    # alias name (without the leading `@`)
    name: destack._generated.core.string.StringId
    # lifetime parameters in type-local slot order
    lifetimes: Sequence[destack._generated.mir.tree.lifetime.LifetimeParameter]
    # the aliased type
    ty: destack._generated.mir.tree.node.LocalNodeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeAlias: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TypeAlias: ...

def encode_type_alias(writer: BinaryWriter, value: TypeAlias) -> None: ...
def decode_type_alias(reader: BinaryReader) -> TypeAlias: ...
def to_json_type_alias(value: TypeAlias) -> Json: ...
def from_json_type_alias(value: Json) -> TypeAlias: ...

@dataclass(frozen=True, slots=True)
class Field:
    """A field in a struct type."""

    # name (optional)
    name: destack._generated.core.string.StringId | None
    # type of the field
    ty: destack._generated.mir.tree.node.LocalNodeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Field: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Field: ...

def encode_field(writer: BinaryWriter, value: Field) -> None: ...
def decode_field(reader: BinaryReader) -> Field: ...
def to_json_field(value: Field) -> Json: ...
def from_json_field(value: Json) -> Field: ...

@dataclass(frozen=True, slots=True)
class TypeError:
    """Invalid type produced while recovering malformed MIR text."""

    kind: typing.Literal["error"] = "error"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeVoid:
    """Void / unit type (no value)."""

    kind: typing.Literal["void"] = "void"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeBoolean:
    """Boolean (1 bit logical, typically 1 byte)."""

    kind: typing.Literal["boolean"] = "boolean"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeInt:
    """Integer with explicit width and signedness."""

    width: int
    is_signed: bool
    kind: typing.Literal["int"] = "int"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeIsize:
    """Pointer-sized signed integer."""

    kind: typing.Literal["isize"] = "isize"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeUsize:
    """Pointer-sized unsigned integer."""

    kind: typing.Literal["usize"] = "usize"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeFloat:
    """Floating point with concrete representation."""

    float: FloatType
    kind: typing.Literal["float"] = "float"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeTypeDescriptor:
    """Runtime type descriptor handle."""

    kind: typing.Literal["typeDescriptor"] = "typeDescriptor"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeTypeId:
    """Compact runtime type identity token."""

    kind: typing.Literal["typeId"] = "typeId"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeAtomic:
    """Atomic storage cell for one value type."""

    # the stored value type
    value: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["atomic"] = "atomic"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeDynamic:
    """Runtime-erased value satisfying one dynamic constraint."""

    # the lowered dynamic constraint type
    constraint: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["dynamic"] = "dynamic"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeWithLifetimes:
    """Type use with applied lifetime arguments."""

    # the type being applied
    base: destack._generated.mir.tree.node.LocalNodeId
    # the applied lifetime arguments
    lifetimes: Sequence[destack._generated.mir.tree.lifetime.Lifetime]
    kind: typing.Literal["withLifetimes"] = "withLifetimes"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeReference:
    """Reference with explicit kind and access."""

    # the reference kind (managed, unique, borrowed, raw)
    kind_value: ReferenceKind
    # lifetime roots for borrowed references
    lifetime: destack._generated.mir.tree.lifetime.Lifetime
    # the space for this reference
    space: Space
    # the access exposed through this reference
    access: Access
    # the referenced type
    pointee: destack._generated.mir.tree.node.LocalNodeId
    # the nullish values allowed by this reference
    nullability: Nullability
    kind: typing.Literal["reference"] = "reference"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeSlice:
    """Slice into memory, a repeated element with explicit kind and access."""

    # the reference kind of the slice base
    kind_value: ReferenceKind
    # lifetime roots for borrowed slices
    lifetime: destack._generated.mir.tree.lifetime.Lifetime
    # the element type of the slice
    element: destack._generated.mir.tree.node.LocalNodeId
    # the space of the slice base
    space: Space
    # the element access exposed by the slice
    access: Access
    # the nullish values allowed by this slice descriptor
    nullability: Nullability
    kind: typing.Literal["slice"] = "slice"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeUninit:
    """Linear token for one uninitialized allocation."""

    # the value under construction
    value: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["uninit"] = "uninit"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeFixedArray:
    """Fixed array: `[T; N]`."""

    # the element type of the array
    element: destack._generated.mir.tree.node.LocalNodeId
    # the number of elements in the array
    length: int
    # copy of this fixed array type
    copy: Copy
    kind: typing.Literal["fixedArray"] = "fixedArray"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeTuple:
    """Tuple: `(T1, T2, ...)`."""

    # the element types of the tuple
    elements: Sequence[destack._generated.mir.tree.node.LocalNodeId]
    # copy of this tuple type
    copy: Copy
    kind: typing.Literal["tuple"] = "tuple"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeStruct:
    """Struct."""

    # the fields of the struct
    fields: Sequence[destack._generated.mir.tree.node.LocalNodeId]
    # copy of this struct type
    copy: Copy
    kind: typing.Literal["struct"] = "struct"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeNewtype:
    """Nominal newtype wrapping an inner type."""

    # the wrapped inner type
    inner: destack._generated.mir.tree.node.LocalNodeId
    # copy of this newtype
    copy: Copy
    kind: typing.Literal["newtype"] = "newtype"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeVariant:
    """Physical tagged sum value."""

    # the tag value type
    tag: destack._generated.mir.tree.node.LocalNodeId
    # the physical payload storage type
    storage: destack._generated.mir.tree.node.LocalNodeId
    # the cases keyed by tag value
    cases: Sequence[VariantCase]
    # copy of this variant type
    copy: Copy
    kind: typing.Literal["variant"] = "variant"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeVector:
    """Fixed-width vector value."""

    # the element type
    element: destack._generated.mir.tree.node.LocalNodeId
    # the number of lanes
    lanes: int
    # copy of this vector type
    copy: Copy
    kind: typing.Literal["vector"] = "vector"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeTensor:
    """Ranked tensor value with static or dynamic shape."""

    # the element type
    element: destack._generated.mir.tree.node.LocalNodeId
    # the static shape
    shape: Sequence[TensorDimension]
    # the tensor format
    format: TensorFormat
    # the tensor placement
    sharding: TensorSharding
    # copy of this tensor type
    copy: Copy
    kind: typing.Literal["tensor"] = "tensor"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeTensorView:
    """Reference-like view into tensor-shaped memory."""

    # the reference kind (managed, unique, borrowed, raw)
    kind_value: ReferenceKind
    # lifetime roots for borrowed tensor views
    lifetime: destack._generated.mir.tree.lifetime.Lifetime
    # the space for this view
    space: Space
    # the access exposed through this view
    access: Access
    # the element type
    element: destack._generated.mir.tree.node.LocalNodeId
    # the static shape
    shape: Sequence[TensorDimension]
    # the tensor view format
    format: TensorViewFormat
    # the tensor placement
    sharding: TensorSharding
    # the nullish values allowed by this view descriptor
    nullability: Nullability
    kind: typing.Literal["tensorView"] = "tensorView"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeFunctionSignature:
    """Bare function signature."""

    # lifetime parameters in signature-local slot order
    lifetimes: Sequence[destack._generated.mir.tree.lifetime.LifetimeParameter]
    # the parameters of the function
    parameters: Sequence[destack._generated.mir.tree.parameter.SignatureParameter]
    # the result type of the function
    result: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["functionSignature"] = "functionSignature"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeFunction:
    """Function value type."""

    # the bare function signature
    signature: destack._generated.mir.tree.node.LocalNodeId
    # the captured environment reference
    environment: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["function"] = "function"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeFunctionPointer:
    """Function pointer type."""

    # the bare function signature
    signature: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["functionPointer"] = "functionPointer"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Concrete type in MIR (post-monomorphization)."""
Type: typing.TypeAlias = (
    TypeError
    | TypeVoid
    | TypeBoolean
    | TypeInt
    | TypeIsize
    | TypeUsize
    | TypeFloat
    | TypeTypeDescriptor
    | TypeTypeId
    | TypeAtomic
    | TypeDynamic
    | TypeWithLifetimes
    | TypeReference
    | TypeSlice
    | TypeUninit
    | TypeFixedArray
    | TypeTuple
    | TypeStruct
    | TypeNewtype
    | TypeVariant
    | TypeVector
    | TypeTensor
    | TypeTensorView
    | TypeFunctionSignature
    | TypeFunction
    | TypeFunctionPointer
)

def encode_type(writer: BinaryWriter, value: Type) -> None: ...
def decode_type(reader: BinaryReader) -> Type: ...
def to_json_type(value: Type) -> Json: ...
def from_json_type(value: Json) -> Type: ...

__all__ = [
    "FloatType",
    "encode_float_type",
    "decode_float_type",
    "to_json_float_type",
    "from_json_float_type",
    "Mutability",
    "encode_mutability",
    "decode_mutability",
    "to_json_mutability",
    "from_json_mutability",
    "ReferenceKind",
    "encode_reference_kind",
    "decode_reference_kind",
    "to_json_reference_kind",
    "from_json_reference_kind",
    "Space",
    "encode_space",
    "decode_space",
    "to_json_space",
    "from_json_space",
    "Access",
    "encode_access",
    "decode_access",
    "to_json_access",
    "from_json_access",
    "Nullability",
    "encode_nullability",
    "decode_nullability",
    "to_json_nullability",
    "from_json_nullability",
    "Copy",
    "encode_copy",
    "decode_copy",
    "to_json_copy",
    "from_json_copy",
    "VariantCase",
    "encode_variant_case",
    "decode_variant_case",
    "to_json_variant_case",
    "from_json_variant_case",
    "TensorDimension",
    "encode_tensor_dimension",
    "decode_tensor_dimension",
    "to_json_tensor_dimension",
    "from_json_tensor_dimension",
    "TensorDimensionStatic",
    "TensorDimensionSymbol",
    "TensorDimensionDynamic",
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
    "TensorViewFormat",
    "encode_tensor_view_format",
    "decode_tensor_view_format",
    "to_json_tensor_view_format",
    "from_json_tensor_view_format",
    "TensorViewFormatDense",
    "TensorViewFormatStrided",
    "TypeAlias",
    "encode_type_alias",
    "decode_type_alias",
    "to_json_type_alias",
    "from_json_type_alias",
    "Field",
    "encode_field",
    "decode_field",
    "to_json_field",
    "from_json_field",
    "Type",
    "encode_type",
    "decode_type",
    "to_json_type",
    "from_json_type",
    "TypeError",
    "TypeVoid",
    "TypeBoolean",
    "TypeInt",
    "TypeIsize",
    "TypeUsize",
    "TypeFloat",
    "TypeTypeDescriptor",
    "TypeTypeId",
    "TypeAtomic",
    "TypeDynamic",
    "TypeWithLifetimes",
    "TypeReference",
    "TypeSlice",
    "TypeUninit",
    "TypeFixedArray",
    "TypeTuple",
    "TypeStruct",
    "TypeNewtype",
    "TypeVariant",
    "TypeVector",
    "TypeTensor",
    "TypeTensorView",
    "TypeFunctionSignature",
    "TypeFunction",
    "TypeFunctionPointer",
]
