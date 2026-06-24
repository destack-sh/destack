# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> LayoutSegment: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> LayoutSegment: ...

def encode_layout_segment(writer: BinaryWriter, value: LayoutSegment) -> None: ...
def decode_layout_segment(reader: BinaryReader) -> LayoutSegment: ...
def to_json_layout_segment(value: LayoutSegment) -> Json: ...
def from_json_layout_segment(value: Json) -> LayoutSegment: ...

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

    kind: typing.Literal["scalar"] = "scalar"

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
    """Object storage with a dispatch table header."""

    object: ObjectLayout
    kind: typing.Literal["object"] = "object"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class LayoutShapeDynamic:
    """Runtime dynamic value storage."""

    kind: typing.Literal["dynamic"] = "dynamic"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class LayoutShapeFunction:
    """Runtime function value storage."""

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

@dataclass(frozen=True, slots=True)
class LayoutShapePointer:
    """Pointer storage slot for an indirectly stored value."""

    pointer: PointerLayout
    kind: typing.Literal["pointer"] = "pointer"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

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

def encode_layout_shape(writer: BinaryWriter, value: LayoutShape) -> None: ...
def decode_layout_shape(reader: BinaryReader) -> LayoutShape: ...
def to_json_layout_shape(value: LayoutShape) -> Json: ...
def from_json_layout_shape(value: Json) -> LayoutShape: ...

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

"""Unique identifier for a concrete layout."""
LocalLayoutId: typing.TypeAlias = int

def encode_local_layout_id(writer: BinaryWriter, value: LocalLayoutId) -> None: ...
def decode_local_layout_id(reader: BinaryReader) -> LocalLayoutId: ...
def to_json_local_layout_id(value: LocalLayoutId) -> Json: ...
def from_json_local_layout_id(value: Json) -> LocalLayoutId: ...

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
class ElementLayout:
    """Layout for inline indexed element storage."""

    # the stored element type
    element: destack._generated.dir.type.type.GlobalTypeId
    # the byte stride between elements
    stride: int
    # the fixed element count when known
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
    element: destack._generated.dir.type.type.GlobalTypeId
    # the tensor storage format
    format: TensorFormat
    # the tensor placement
    sharding: TensorSharding
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
class VariantLayout:
    """Concrete layout for a variant value."""

    # the tag layout
    tag: VariantTagLayout
    # the variant payload byte offset
    payload_offset: int | None
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
    ty: destack._generated.dir.type.type.GlobalTypeId | None
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
    ty: destack._generated.dir.type.type.GlobalTypeId
    # the case layout
    layout: LocalLayoutId

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
class NewtypeLayout:
    """Concrete layout for a nominal newtype."""

    # the backing type
    backing_type: destack._generated.dir.type.type.GlobalTypeId
    # the backing type layout
    backing_layout: LocalLayoutId

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
class PointerLayout:
    """Concrete layout for a pointer storage slot."""

    # the pointed-to value type
    pointee: destack._generated.dir.type.type.GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> PointerLayout: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> PointerLayout: ...

def encode_pointer_layout(writer: BinaryWriter, value: PointerLayout) -> None: ...
def decode_pointer_layout(reader: BinaryReader) -> PointerLayout: ...
def to_json_pointer_layout(value: PointerLayout) -> Json: ...
def from_json_pointer_layout(value: Json) -> PointerLayout: ...

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Niche: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Niche: ...

def encode_niche(writer: BinaryWriter, value: Niche) -> None: ...
def decode_niche(reader: BinaryReader) -> Niche: ...
def to_json_niche(value: Niche) -> Json: ...
def from_json_niche(value: Json) -> Niche: ...

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
