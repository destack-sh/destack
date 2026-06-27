# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.string
import destack._generated.mir.table.trace
import destack._generated.mir.tree.type
import destack._generated.program.function
import destack._generated.program.type

"""Opaque identifier for one executable memory layout."""
LayoutId: typing.TypeAlias = int

def encode_layout_id(writer: BinaryWriter, value: LayoutId) -> None: ...
def decode_layout_id(reader: BinaryReader) -> LayoutId: ...
def to_json_layout_id(value: LayoutId) -> Json: ...
def from_json_layout_id(value: Json) -> LayoutId: ...

@dataclass(frozen=True, slots=True)
class LayoutTable:
    """Shared executable layout table for all runtime value layouts."""

    # layout entries indexed by LayoutId
    entries: Sequence[Layout]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> LayoutTable: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> LayoutTable: ...

def encode_layout_table(writer: BinaryWriter, value: LayoutTable) -> None: ...
def decode_layout_table(reader: BinaryReader) -> LayoutTable: ...
def to_json_layout_table(value: LayoutTable) -> Json: ...
def from_json_layout_table(value: Json) -> LayoutTable: ...

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Layout: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Layout: ...

def encode_layout(writer: BinaryWriter, value: Layout) -> None: ...
def decode_layout(reader: BinaryReader) -> Layout: ...
def to_json_layout(value: Layout) -> Json: ...
def from_json_layout(value: Json) -> Layout: ...

@dataclass(frozen=True, slots=True)
class LayoutShapeNone:
    """No runtime storage."""

    kind: typing.Literal["none"] = "none"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class LayoutShapeScalar:
    """Builtin scalar storage."""

    scalar: ScalarFormat
    kind: typing.Literal["scalar"] = "scalar"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class LayoutShapeReference:
    """Reference storage."""

    reference: ReferenceLayout
    kind: typing.Literal["reference"] = "reference"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class LayoutShapeFunctionPointer:
    """Function pointer storage."""

    function_pointer: destack._generated.program.function.Signature
    kind: typing.Literal["functionPointer"] = "functionPointer"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class LayoutShapeStruct:
    """Struct storage."""

    struct: StructLayout
    kind: typing.Literal["struct"] = "struct"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class LayoutShapeTuple:
    """Tuple storage."""

    tuple: TupleLayout
    kind: typing.Literal["tuple"] = "tuple"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class LayoutShapeSlice:
    """Slice header storage."""

    slice: SliceLayout
    kind: typing.Literal["slice"] = "slice"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class LayoutShapeArray:
    """Fixed array storage."""

    array: ElementLayout
    kind: typing.Literal["array"] = "array"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class LayoutShapeVector:
    """Vector value storage."""

    vector: ElementLayout
    kind: typing.Literal["vector"] = "vector"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class LayoutShapeTensor:
    """Tensor handle storage."""

    tensor: TensorLayout
    kind: typing.Literal["tensor"] = "tensor"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class LayoutShapeTensorView:
    """Tensor view descriptor storage."""

    tensor_view: TensorViewLayout
    kind: typing.Literal["tensorView"] = "tensorView"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class LayoutShapeVariant:
    """Variant value storage."""

    variant: VariantLayout
    kind: typing.Literal["variant"] = "variant"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class LayoutShapeObject:
    """Object storage."""

    object: ObjectLayout
    kind: typing.Literal["object"] = "object"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class LayoutShapeDynamic:
    """Runtime dynamic value layout."""

    kind: typing.Literal["dynamic"] = "dynamic"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class LayoutShapeFunction:
    """Runtime function value storage."""

    function: FunctionLayout
    kind: typing.Literal["function"] = "function"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class LayoutShapeNewtype:
    """Transparent nominal storage."""

    newtype: NewtypeLayout
    kind: typing.Literal["newtype"] = "newtype"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

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

def encode_layout_shape(writer: BinaryWriter, value: LayoutShape) -> None: ...
def decode_layout_shape(reader: BinaryReader) -> LayoutShape: ...
def to_json_layout_shape(value: LayoutShape) -> Json: ...
def from_json_layout_shape(value: Json) -> LayoutShape: ...

@dataclass(frozen=True, slots=True)
class ScalarFormatInt:
    """Signed or unsigned integers with a bit width."""

    # the bit width
    width: int
    # whether the integer is signed
    is_signed: bool
    kind: typing.Literal["int"] = "int"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ScalarFormatFloat:
    """Floating-point values with a concrete format."""

    # the concrete float format
    format: destack._generated.mir.tree.type.FloatType
    kind: typing.Literal["float"] = "float"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ScalarFormatBoolean:
    """Boolean values."""

    kind: typing.Literal["boolean"] = "boolean"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Scalar storage format for vector and tensor element operations."""
ScalarFormat: typing.TypeAlias = (
    ScalarFormatInt | ScalarFormatFloat | ScalarFormatBoolean
)

def encode_scalar_format(writer: BinaryWriter, value: ScalarFormat) -> None: ...
def decode_scalar_format(reader: BinaryReader) -> ScalarFormat: ...
def to_json_scalar_format(value: ScalarFormat) -> Json: ...
def from_json_scalar_format(value: Json) -> ScalarFormat: ...

@dataclass(frozen=True, slots=True)
class ReferenceLayout:
    """Concrete layout for one reference value."""

    # the referenced value type
    pointee: destack._generated.program.type.TypeId
    # the packed reference flags
    flags: ReferenceFlags

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ReferenceLayout: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ReferenceLayout: ...

def encode_reference_layout(writer: BinaryWriter, value: ReferenceLayout) -> None: ...
def decode_reference_layout(reader: BinaryReader) -> ReferenceLayout: ...
def to_json_reference_layout(value: ReferenceLayout) -> Json: ...
def from_json_reference_layout(value: Json) -> ReferenceLayout: ...

@dataclass(frozen=True, slots=True)
class ReferenceFlags:
    """Packed reference flags used by executable layouts."""

    bits: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ReferenceFlags: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ReferenceFlags: ...

def encode_reference_flags(writer: BinaryWriter, value: ReferenceFlags) -> None: ...
def decode_reference_flags(reader: BinaryReader) -> ReferenceFlags: ...
def to_json_reference_flags(value: ReferenceFlags) -> Json: ...
def from_json_reference_flags(value: Json) -> ReferenceFlags: ...

@dataclass(frozen=True, slots=True)
class StructLayout:
    """Concrete layout for a struct."""

    # the fields in layout order
    fields: Sequence[LayoutField]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> StructLayout: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> StructLayout: ...

def encode_struct_layout(writer: BinaryWriter, value: StructLayout) -> None: ...
def decode_struct_layout(reader: BinaryReader) -> StructLayout: ...
def to_json_struct_layout(value: StructLayout) -> Json: ...
def from_json_struct_layout(value: Json) -> StructLayout: ...

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> LayoutField: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> LayoutField: ...

def encode_layout_field(writer: BinaryWriter, value: LayoutField) -> None: ...
def decode_layout_field(reader: BinaryReader) -> LayoutField: ...
def to_json_layout_field(value: LayoutField) -> Json: ...
def from_json_layout_field(value: Json) -> LayoutField: ...

@dataclass(frozen=True, slots=True)
class TupleLayout:
    """Concrete layout for a tuple."""

    # the tuple elements in layout order
    elements: Sequence[LayoutField]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TupleLayout: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TupleLayout: ...

def encode_tuple_layout(writer: BinaryWriter, value: TupleLayout) -> None: ...
def decode_tuple_layout(reader: BinaryReader) -> TupleLayout: ...
def to_json_tuple_layout(value: TupleLayout) -> Json: ...
def from_json_tuple_layout(value: Json) -> TupleLayout: ...

@dataclass(frozen=True, slots=True)
class SliceLayout:
    """Concrete layout for one slice descriptor."""

    # the backing data reference
    data: ReferenceLayout

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SliceLayout: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SliceLayout: ...

def encode_slice_layout(writer: BinaryWriter, value: SliceLayout) -> None: ...
def decode_slice_layout(reader: BinaryReader) -> SliceLayout: ...
def to_json_slice_layout(value: SliceLayout) -> Json: ...
def from_json_slice_layout(value: Json) -> SliceLayout: ...

@dataclass(frozen=True, slots=True)
class ElementLayout:
    """Layout for inline indexed element storage."""

    # the stored element type
    element: destack._generated.program.type.TypeId
    # the byte stride between elements
    stride: int
    # the fixed element count
    count: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ElementLayout: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ElementLayout: ...

def encode_element_layout(writer: BinaryWriter, value: ElementLayout) -> None: ...
def decode_element_layout(reader: BinaryReader) -> ElementLayout: ...
def to_json_element_layout(value: ElementLayout) -> Json: ...
def from_json_element_layout(value: Json) -> ElementLayout: ...

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorLayout: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TensorLayout: ...

def encode_tensor_layout(writer: BinaryWriter, value: TensorLayout) -> None: ...
def decode_tensor_layout(reader: BinaryReader) -> TensorLayout: ...
def to_json_tensor_layout(value: TensorLayout) -> Json: ...
def from_json_tensor_layout(value: Json) -> TensorLayout: ...

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorViewLayout: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TensorViewLayout: ...

def encode_tensor_view_layout(
    writer: BinaryWriter, value: TensorViewLayout
) -> None: ...
def decode_tensor_view_layout(reader: BinaryReader) -> TensorViewLayout: ...
def to_json_tensor_view_layout(value: TensorViewLayout) -> Json: ...
def from_json_tensor_view_layout(value: Json) -> TensorViewLayout: ...

@dataclass(frozen=True, slots=True)
class VariantLayout:
    """Concrete layout for a variant value."""

    # the tag layout
    tag: VariantTagLayout
    # the variant payload byte offset
    payload_offset: int
    # the variant cases
    variants: Sequence[VariantCaseLayout]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> VariantLayout: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> VariantLayout: ...

def encode_variant_layout(writer: BinaryWriter, value: VariantLayout) -> None: ...
def decode_variant_layout(reader: BinaryReader) -> VariantLayout: ...
def to_json_variant_layout(value: VariantLayout) -> Json: ...
def from_json_variant_layout(value: Json) -> VariantLayout: ...

@dataclass(frozen=True, slots=True)
class VariantTagLayout:
    """Concrete layout for a variant tag."""

    # the tag type when it has been materialized
    ty: destack._generated.program.type.TypeId | None
    # the tag size in bytes
    size: int
    # the tag alignment in bytes
    alignment: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> VariantTagLayout: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> VariantTagLayout: ...

def encode_variant_tag_layout(
    writer: BinaryWriter, value: VariantTagLayout
) -> None: ...
def decode_variant_tag_layout(reader: BinaryReader) -> VariantTagLayout: ...
def to_json_variant_tag_layout(value: VariantTagLayout) -> Json: ...
def from_json_variant_tag_layout(value: Json) -> VariantTagLayout: ...

@dataclass(frozen=True, slots=True)
class VariantCaseLayout:
    """Concrete layout for one variant case."""

    # the logical case type
    ty: destack._generated.program.type.TypeId
    # the case layout
    layout: LayoutId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> VariantCaseLayout: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> VariantCaseLayout: ...

def encode_variant_case_layout(
    writer: BinaryWriter, value: VariantCaseLayout
) -> None: ...
def decode_variant_case_layout(reader: BinaryReader) -> VariantCaseLayout: ...
def to_json_variant_case_layout(value: VariantCaseLayout) -> Json: ...
def from_json_variant_case_layout(value: Json) -> VariantCaseLayout: ...

@dataclass(frozen=True, slots=True)
class ObjectLayout:
    """Concrete layout for an object."""

    # byte offset of the virtual dispatch pointer when this object carries one
    dispatch_offset: int | None
    # the fields in layout order
    fields: Sequence[LayoutField]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ObjectLayout: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ObjectLayout: ...

def encode_object_layout(writer: BinaryWriter, value: ObjectLayout) -> None: ...
def decode_object_layout(reader: BinaryReader) -> ObjectLayout: ...
def to_json_object_layout(value: ObjectLayout) -> Json: ...
def from_json_object_layout(value: Json) -> ObjectLayout: ...

@dataclass(frozen=True, slots=True)
class FunctionLayout:
    """Concrete layout for one closure value."""

    # the callable signature
    signature: destack._generated.program.function.Signature
    # the captured environment type
    environment: destack._generated.program.type.TypeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionLayout: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FunctionLayout: ...

def encode_function_layout(writer: BinaryWriter, value: FunctionLayout) -> None: ...
def decode_function_layout(reader: BinaryReader) -> FunctionLayout: ...
def to_json_function_layout(value: FunctionLayout) -> Json: ...
def from_json_function_layout(value: Json) -> FunctionLayout: ...

@dataclass(frozen=True, slots=True)
class NewtypeLayout:
    """Concrete layout for a nominal newtype."""

    # the backing type
    backing_type: destack._generated.program.type.TypeId
    # the backing type layout
    backing_layout: LayoutId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> NewtypeLayout: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> NewtypeLayout: ...

def encode_newtype_layout(writer: BinaryWriter, value: NewtypeLayout) -> None: ...
def decode_newtype_layout(reader: BinaryReader) -> NewtypeLayout: ...
def to_json_newtype_layout(value: NewtypeLayout) -> Json: ...
def from_json_newtype_layout(value: Json) -> NewtypeLayout: ...

@dataclass(frozen=True, slots=True)
class CellLayoutVoid:
    """Void value."""

    kind: typing.Literal["void"] = "void"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CellLayoutBoolean:
    """Boolean value."""

    kind: typing.Literal["boolean"] = "boolean"

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
class CellLayoutGlobalAddress:
    """Global address."""

    kind: typing.Literal["globalAddress"] = "globalAddress"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CellLayoutFunctionPointer:
    """Function pointer."""

    kind: typing.Literal["functionPointer"] = "functionPointer"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

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

def encode_cell_layout(writer: BinaryWriter, value: CellLayout) -> None: ...
def decode_cell_layout(reader: BinaryReader) -> CellLayout: ...
def to_json_cell_layout(value: CellLayout) -> Json: ...
def from_json_cell_layout(value: Json) -> CellLayout: ...

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
