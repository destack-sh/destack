# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

from destack._impl.mir.metadata.layout import (
    LayoutMetadataImpl,
)
from destack._impl.mir.metadata.layout import (
    LayoutTableImpl,
)
from destack._impl.mir.metadata.layout import (
    LayoutImpl,
)
from destack._impl.mir.metadata.layout import (
    LayoutShapeImpl,
)

import destack._generated.core.string
import destack._generated.mir.metadata.trace
import destack._generated.mir.tree.node
import destack._generated.mir.tree.type

@dataclass(frozen=True, slots=True)
class LayoutMetadata(LayoutMetadataImpl):
    """Canonical layout metadata for one MIR module."""

    # layout metadata table for aggregate types
    layout_table: LayoutTable
    # concrete layout ids keyed by type id
    layout_by_type: Mapping[destack._generated.mir.tree.node.LocalNodeId, LayoutId]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> LayoutMetadata: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> LayoutMetadata: ...

def encode_layout_metadata(writer: BinaryWriter, value: LayoutMetadata) -> None: ...
def decode_layout_metadata(reader: BinaryReader) -> LayoutMetadata: ...
def to_json_layout_metadata(value: LayoutMetadata) -> Json: ...
def from_json_layout_metadata(value: Json) -> LayoutMetadata: ...

@dataclass(frozen=True, slots=True)
class LayoutTable(LayoutTableImpl):
    """Shared layout table for all aggregate types."""

    # layout entries indexed by LayoutId
    layouts: Sequence[Layout]

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
class Layout(LayoutImpl):
    """Concrete memory layout for an aggregate type."""

    # the layout shape
    shape: LayoutShape
    # total size in bytes, including trailing padding
    size: int
    # alignment requirement in bytes
    alignment: int
    # managed-reference metadata for this layout
    trace_map: destack._generated.mir.metadata.trace.TraceMap

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
class LayoutShapeNone(LayoutShapeImpl):
    """No runtime storage."""

    kind: typing.Literal["none"] = "none"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class LayoutShapeScalar(LayoutShapeImpl):
    """Builtin scalar storage."""

    kind: typing.Literal["scalar"] = "scalar"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class LayoutShapeStruct(LayoutShapeImpl):
    """Struct storage."""

    struct: StructLayout
    kind: typing.Literal["struct"] = "struct"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class LayoutShapeTuple(LayoutShapeImpl):
    """Tuple storage."""

    tuple: TupleLayout
    kind: typing.Literal["tuple"] = "tuple"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class LayoutShapeSlice(LayoutShapeImpl):
    """Slice header storage."""

    kind: typing.Literal["slice"] = "slice"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class LayoutShapeArray(LayoutShapeImpl):
    """Fixed array storage."""

    array: ElementLayout
    kind: typing.Literal["array"] = "array"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class LayoutShapeVector(LayoutShapeImpl):
    """Vector value storage."""

    vector: ElementLayout
    kind: typing.Literal["vector"] = "vector"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class LayoutShapeTensor(LayoutShapeImpl):
    """Tensor handle storage."""

    tensor: TensorLayout
    kind: typing.Literal["tensor"] = "tensor"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class LayoutShapeTensorView(LayoutShapeImpl):
    """Tensor view descriptor storage."""

    tensor_view: TensorViewLayout
    kind: typing.Literal["tensorView"] = "tensorView"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class LayoutShapeVariant(LayoutShapeImpl):
    """Variant value storage."""

    variant: VariantLayout
    kind: typing.Literal["variant"] = "variant"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class LayoutShapeObject(LayoutShapeImpl):
    """Object storage with a dispatch table header."""

    object: ObjectLayout
    kind: typing.Literal["object"] = "object"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class LayoutShapeDynamic(LayoutShapeImpl):
    """Runtime dynamic value layout."""

    kind: typing.Literal["dynamic"] = "dynamic"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class LayoutShapeFunction(LayoutShapeImpl):
    """Runtime function value storage."""

    kind: typing.Literal["function"] = "function"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class LayoutShapeNewtype(LayoutShapeImpl):
    """Transparent nominal storage."""

    newtype: NewtypeLayout
    kind: typing.Literal["newtype"] = "newtype"

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
class ElementLayout:
    """Layout for inline indexed element storage."""

    # the stored element type
    element: destack._generated.mir.tree.node.LocalNodeId
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
    element: destack._generated.mir.tree.node.LocalNodeId
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
    element: destack._generated.mir.tree.node.LocalNodeId
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
    ty: destack._generated.mir.tree.node.LocalNodeId | None
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
    ty: destack._generated.mir.tree.node.LocalNodeId
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

"""Opaque identifier for a concrete memory layout."""
LayoutId: typing.TypeAlias = int

def encode_layout_id(writer: BinaryWriter, value: LayoutId) -> None: ...
def decode_layout_id(reader: BinaryReader) -> LayoutId: ...
def to_json_layout_id(value: LayoutId) -> Json: ...
def from_json_layout_id(value: Json) -> LayoutId: ...

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
    backing_type: destack._generated.mir.tree.node.LocalNodeId
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

__all__ = [
    "LayoutMetadata",
    "encode_layout_metadata",
    "decode_layout_metadata",
    "to_json_layout_metadata",
    "from_json_layout_metadata",
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
