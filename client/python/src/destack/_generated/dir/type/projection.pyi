# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.symbol.key
import destack._generated.dir.symbol.symbol
import destack._generated.dir.tree.literal
import destack._generated.dir.tree.node
import destack._generated.dir.type.generic
import destack._generated.dir.type.resolution
import destack._generated.dir.type.type

@dataclass(frozen=True, slots=True)
class ProjectionFieldGet:
    """Extract one static layout field from an aggregate value."""

    # the selected field
    field: ProjectionField
    # the projected value type
    ty: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["fieldGet"] = "fieldGet"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProjectionPropertyGet:
    """Read one accessor-backed property value."""

    # the selected getter member
    read: destack._generated.dir.type.resolution.MemberResolution
    # the projected value type
    ty: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["propertyGet"] = "propertyGet"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProjectionSubscriptGet:
    """Read one dynamically selected subscript value."""

    # the source node providing the subscript key
    index: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the selected subscript operation
    read: SubscriptOperation
    # the projected value type
    ty: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["subscriptGet"] = "subscriptGet"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProjectionCall:
    """Read one value through a selected call."""

    # the selected call operation
    call: destack._generated.dir.type.resolution.CallResolution
    # the returned value type
    ty: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["call"] = "call"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProjectionObjectRest:
    """Materialize one object rest value from selected fields."""

    # the selected source field projections
    fields: Sequence[ObjectRestField]
    # the materialized rest value type
    ty: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["objectRest"] = "objectRest"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProjectionSliceLength:
    """Read the runtime length from a slice descriptor."""

    # the projected length type
    ty: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["sliceLength"] = "sliceLength"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProjectionDynamicPayload:
    """Read the erased payload from a dynamic value."""

    # the projected payload type
    ty: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["dynamicPayload"] = "dynamicPayload"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProjectionDynamicType:
    """Read the concrete type id from a dynamic value."""

    # the projected type descriptor type
    ty: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["dynamicType"] = "dynamicType"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProjectionVariantTag:
    """Read the active tag from a physical tagged sum value."""

    # the projected tag type
    ty: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["variantTag"] = "variantTag"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProjectionVariantPayload:
    """Extract the payload selected by one concrete variant tag."""

    # the selected tagged case
    case: VariantCase
    # the selected generic argument bindings for the selected owner
    generic_arguments: Sequence[
        destack._generated.dir.type.generic.GenericArgumentBinding
    ]
    # the discriminant value tested at runtime
    discriminant: destack._generated.dir.tree.literal.ScalarLiteral
    # the projected payload type
    ty: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["variantPayload"] = "variantPayload"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProjectionNewtypePayload:
    """Unwrap one newtype payload."""

    # the selected newtype symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the selected generic argument bindings for the selected newtype
    generic_arguments: Sequence[
        destack._generated.dir.type.generic.GenericArgumentBinding
    ]
    # the projected payload type
    ty: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["newtypePayload"] = "newtypePayload"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProjectionBorrow:
    """Borrow the input before matching it."""

    # the requested borrow access, if source explicit
    access: destack._generated.dir.type.type.Access | None
    # the projected borrow type
    ty: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["borrow"] = "borrow"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProjectionMove:
    """Move the input before matching it."""

    # the requested move access, if source explicit
    access: destack._generated.dir.type.type.Access | None
    # the projected moved type
    ty: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["move"] = "move"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProjectionDereference:
    """Dereference the input before matching it."""

    # the selected dereference operation
    read: DereferenceOperation
    # the projected pointee type
    ty: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["dereference"] = "dereference"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProjectionCopy:
    """Duplicate one copyable value out of a place or view."""

    # the duplicated value type
    ty: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["copy"] = "copy"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Value projection selected during checking."""
Projection: typing.TypeAlias = (
    ProjectionFieldGet
    | ProjectionPropertyGet
    | ProjectionSubscriptGet
    | ProjectionCall
    | ProjectionObjectRest
    | ProjectionSliceLength
    | ProjectionDynamicPayload
    | ProjectionDynamicType
    | ProjectionVariantTag
    | ProjectionVariantPayload
    | ProjectionNewtypePayload
    | ProjectionBorrow
    | ProjectionMove
    | ProjectionDereference
    | ProjectionCopy
)

def encode_projection(writer: BinaryWriter, value: Projection) -> None: ...
def decode_projection(reader: BinaryReader) -> Projection: ...
def to_json_projection(value: Projection) -> Json: ...
def from_json_projection(value: Json) -> Projection: ...

@dataclass(frozen=True, slots=True)
class ProjectionFieldKey:
    """Structural field key."""

    key: destack._generated.dir.symbol.key.StaticKey
    kind: typing.Literal["key"] = "key"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProjectionFieldMember:
    """Declaration-backed nominal stored member."""

    member: destack._generated.dir.symbol.symbol.GlobalSymbolId
    kind: typing.Literal["member"] = "member"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Static field selected by one projection."""
ProjectionField: typing.TypeAlias = ProjectionFieldKey | ProjectionFieldMember

def encode_projection_field(writer: BinaryWriter, value: ProjectionField) -> None: ...
def decode_projection_field(reader: BinaryReader) -> ProjectionField: ...
def to_json_projection_field(value: ProjectionField) -> Json: ...
def from_json_projection_field(value: Json) -> ProjectionField: ...

@dataclass(frozen=True, slots=True)
class SubscriptOperationMember:
    """Structural tuple, field, or index-signature selection."""

    member: destack._generated.dir.type.resolution.MemberResolution
    kind: typing.Literal["member"] = "member"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class SubscriptOperationCall:
    """Protocol-backed subscript call."""

    call: destack._generated.dir.type.resolution.CallResolution
    kind: typing.Literal["call"] = "call"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Subscript operation selected by one projection or place."""
SubscriptOperation: typing.TypeAlias = SubscriptOperationMember | SubscriptOperationCall

def encode_subscript_operation(
    writer: BinaryWriter, value: SubscriptOperation
) -> None: ...
def decode_subscript_operation(reader: BinaryReader) -> SubscriptOperation: ...
def to_json_subscript_operation(value: SubscriptOperation) -> Json: ...
def from_json_subscript_operation(value: Json) -> SubscriptOperation: ...

@dataclass(frozen=True, slots=True)
class ObjectRestField:
    """One source field used to materialize an object rest value."""

    # the materialized field key
    key: destack._generated.dir.symbol.key.StaticKey
    # the selected source projection
    projection: Projection

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ObjectRestField: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ObjectRestField: ...

def encode_object_rest_field(writer: BinaryWriter, value: ObjectRestField) -> None: ...
def decode_object_rest_field(reader: BinaryReader) -> ObjectRestField: ...
def to_json_object_rest_field(value: ObjectRestField) -> Json: ...
def from_json_object_rest_field(value: Json) -> ObjectRestField: ...

@dataclass(frozen=True, slots=True)
class VariantCase:
    """One tagged union case selected during checking."""

    # the selected variant family symbol
    owner: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the source-level case key
    key: destack._generated.dir.symbol.key.StaticKey
    # the selected variant declaration
    member: destack._generated.dir.symbol.symbol.GlobalSymbolId

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
class DereferenceOperationDirect:
    """Direct dereference of a physical reference or pointer form."""

    kind: typing.Literal["direct"] = "direct"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DereferenceOperationCall:
    """Protocol-backed dereference call."""

    call: destack._generated.dir.type.resolution.CallResolution
    kind: typing.Literal["call"] = "call"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Dereference operation selected by one projection or place."""
DereferenceOperation: typing.TypeAlias = (
    DereferenceOperationDirect | DereferenceOperationCall
)

def encode_dereference_operation(
    writer: BinaryWriter, value: DereferenceOperation
) -> None: ...
def decode_dereference_operation(reader: BinaryReader) -> DereferenceOperation: ...
def to_json_dereference_operation(value: DereferenceOperation) -> Json: ...
def from_json_dereference_operation(value: Json) -> DereferenceOperation: ...

__all__ = [
    "Projection",
    "encode_projection",
    "decode_projection",
    "to_json_projection",
    "from_json_projection",
    "ProjectionFieldGet",
    "ProjectionPropertyGet",
    "ProjectionSubscriptGet",
    "ProjectionCall",
    "ProjectionObjectRest",
    "ProjectionSliceLength",
    "ProjectionDynamicPayload",
    "ProjectionDynamicType",
    "ProjectionVariantTag",
    "ProjectionVariantPayload",
    "ProjectionNewtypePayload",
    "ProjectionBorrow",
    "ProjectionMove",
    "ProjectionDereference",
    "ProjectionCopy",
    "ProjectionField",
    "encode_projection_field",
    "decode_projection_field",
    "to_json_projection_field",
    "from_json_projection_field",
    "ProjectionFieldKey",
    "ProjectionFieldMember",
    "SubscriptOperation",
    "encode_subscript_operation",
    "decode_subscript_operation",
    "to_json_subscript_operation",
    "from_json_subscript_operation",
    "SubscriptOperationMember",
    "SubscriptOperationCall",
    "ObjectRestField",
    "encode_object_rest_field",
    "decode_object_rest_field",
    "to_json_object_rest_field",
    "from_json_object_rest_field",
    "VariantCase",
    "encode_variant_case",
    "decode_variant_case",
    "to_json_variant_case",
    "from_json_variant_case",
    "DereferenceOperation",
    "encode_dereference_operation",
    "decode_dereference_operation",
    "to_json_dereference_operation",
    "from_json_dereference_operation",
    "DereferenceOperationDirect",
    "DereferenceOperationCall",
]
