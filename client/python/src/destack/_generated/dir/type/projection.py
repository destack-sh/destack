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
    json_field,
    json_object,
    json_optional,
    json_string,
)

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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_projection(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_projection(self)


@dataclass(frozen=True, slots=True)
class ProjectionPropertyGet:
    """Read one accessor-backed property value."""

    # the selected getter member
    read: destack._generated.dir.type.resolution.MemberResolution
    # the projected value type
    ty: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["propertyGet"] = "propertyGet"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_projection(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_projection(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_projection(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_projection(self)


@dataclass(frozen=True, slots=True)
class ProjectionCall:
    """Read one value through a selected call."""

    # the selected call operation
    call: destack._generated.dir.type.resolution.CallResolution
    # the returned value type
    ty: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["call"] = "call"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_projection(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_projection(self)


@dataclass(frozen=True, slots=True)
class ProjectionObjectRest:
    """Materialize one object rest value from selected fields."""

    # the selected source field projections
    fields: Sequence[ObjectRestField]
    # the materialized rest value type
    ty: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["objectRest"] = "objectRest"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_projection(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_projection(self)


@dataclass(frozen=True, slots=True)
class ProjectionSliceLength:
    """Read the runtime length from a slice descriptor."""

    # the projected length type
    ty: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["sliceLength"] = "sliceLength"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_projection(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_projection(self)


@dataclass(frozen=True, slots=True)
class ProjectionDynamicPayload:
    """Read the erased payload from a dynamic value."""

    # the projected payload type
    ty: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["dynamicPayload"] = "dynamicPayload"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_projection(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_projection(self)


@dataclass(frozen=True, slots=True)
class ProjectionDynamicType:
    """Read the concrete type id from a dynamic value."""

    # the projected type descriptor type
    ty: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["dynamicType"] = "dynamicType"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_projection(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_projection(self)


@dataclass(frozen=True, slots=True)
class ProjectionVariantTag:
    """Read the active tag from a physical tagged sum value."""

    # the projected tag type
    ty: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["variantTag"] = "variantTag"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_projection(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_projection(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_projection(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_projection(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_projection(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_projection(self)


@dataclass(frozen=True, slots=True)
class ProjectionBorrow:
    """Borrow the input before matching it."""

    # the requested borrow access, if source explicit
    access: destack._generated.dir.type.type.Access | None
    # the projected borrow type
    ty: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["borrow"] = "borrow"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_projection(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_projection(self)


@dataclass(frozen=True, slots=True)
class ProjectionMove:
    """Move the input before matching it."""

    # the requested move access, if source explicit
    access: destack._generated.dir.type.type.Access | None
    # the projected moved type
    ty: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["move"] = "move"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_projection(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_projection(self)


@dataclass(frozen=True, slots=True)
class ProjectionDereference:
    """Dereference the input before matching it."""

    # the selected dereference operation
    read: DereferenceOperation
    # the projected pointee type
    ty: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["dereference"] = "dereference"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_projection(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_projection(self)


@dataclass(frozen=True, slots=True)
class ProjectionCopy:
    """Duplicate one copyable value out of a place or view."""

    # the duplicated value type
    ty: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["copy"] = "copy"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_projection(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_projection(self)


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


def encode_projection(writer: BinaryWriter, value: Projection) -> None:
    """Encode one Projection."""
    if value.kind == "fieldGet":
        writer.write_unsigned(0)
        encode_projection_field(writer, value.field)
        destack._generated.dir.type.type.encode_global_type_id(writer, value.ty)
    elif value.kind == "propertyGet":
        writer.write_unsigned(1)
        destack._generated.dir.type.resolution.encode_member_resolution(
            writer, value.read
        )
        destack._generated.dir.type.type.encode_global_type_id(writer, value.ty)
    elif value.kind == "subscriptGet":
        writer.write_unsigned(2)
        destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.index)
        encode_subscript_operation(writer, value.read)
        destack._generated.dir.type.type.encode_global_type_id(writer, value.ty)
    elif value.kind == "call":
        writer.write_unsigned(3)
        destack._generated.dir.type.resolution.encode_call_resolution(
            writer, value.call
        )
        destack._generated.dir.type.type.encode_global_type_id(writer, value.ty)
    elif value.kind == "objectRest":
        writer.write_unsigned(4)
        writer.write_unsigned(len(value.fields))
        for item_value_fields_0 in value.fields:
            encode_object_rest_field(writer, item_value_fields_0)
        destack._generated.dir.type.type.encode_global_type_id(writer, value.ty)
    elif value.kind == "sliceLength":
        writer.write_unsigned(5)
        destack._generated.dir.type.type.encode_global_type_id(writer, value.ty)
    elif value.kind == "dynamicPayload":
        writer.write_unsigned(6)
        destack._generated.dir.type.type.encode_global_type_id(writer, value.ty)
    elif value.kind == "dynamicType":
        writer.write_unsigned(7)
        destack._generated.dir.type.type.encode_global_type_id(writer, value.ty)
    elif value.kind == "variantTag":
        writer.write_unsigned(8)
        destack._generated.dir.type.type.encode_global_type_id(writer, value.ty)
    elif value.kind == "variantPayload":
        writer.write_unsigned(9)
        encode_variant_case(writer, value.case)
        writer.write_unsigned(len(value.generic_arguments))
        for item_value_generic_arguments_0 in value.generic_arguments:
            destack._generated.dir.type.generic.encode_generic_argument_binding(
                writer, item_value_generic_arguments_0
            )
        destack._generated.dir.tree.literal.encode_scalar_literal(
            writer, value.discriminant
        )
        destack._generated.dir.type.type.encode_global_type_id(writer, value.ty)
    elif value.kind == "newtypePayload":
        writer.write_unsigned(10)
        destack._generated.dir.symbol.symbol.encode_global_symbol_id(
            writer, value.symbol
        )
        writer.write_unsigned(len(value.generic_arguments))
        for item_value_generic_arguments_0 in value.generic_arguments:
            destack._generated.dir.type.generic.encode_generic_argument_binding(
                writer, item_value_generic_arguments_0
            )
        destack._generated.dir.type.type.encode_global_type_id(writer, value.ty)
    elif value.kind == "borrow":
        writer.write_unsigned(11)
        if value.access is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.type.type.encode_access(writer, value.access)
        destack._generated.dir.type.type.encode_global_type_id(writer, value.ty)
    elif value.kind == "move":
        writer.write_unsigned(12)
        if value.access is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.type.type.encode_access(writer, value.access)
        destack._generated.dir.type.type.encode_global_type_id(writer, value.ty)
    elif value.kind == "dereference":
        writer.write_unsigned(13)
        encode_dereference_operation(writer, value.read)
        destack._generated.dir.type.type.encode_global_type_id(writer, value.ty)
    elif value.kind == "copy":
        writer.write_unsigned(14)
        destack._generated.dir.type.type.encode_global_type_id(writer, value.ty)
    else:
        raise SerdeError("unknown enum variant")


def decode_projection(reader: BinaryReader) -> Projection:
    """Decode one Projection."""
    variant = reader.read_number()

    if variant == 0:
        field = decode_projection_field(reader)
        ty = destack._generated.dir.type.type.decode_global_type_id(reader)

        return ProjectionFieldGet(
            field=field,
            ty=ty,
        )
    elif variant == 1:
        read = destack._generated.dir.type.resolution.decode_member_resolution(reader)
        ty = destack._generated.dir.type.type.decode_global_type_id(reader)

        return ProjectionPropertyGet(
            read=read,
            ty=ty,
        )
    elif variant == 2:
        index = destack._generated.dir.tree.node.decode_global_node_id_any(reader)
        read = decode_subscript_operation(reader)
        ty = destack._generated.dir.type.type.decode_global_type_id(reader)

        return ProjectionSubscriptGet(
            index=index,
            read=read,
            ty=ty,
        )
    elif variant == 3:
        call = destack._generated.dir.type.resolution.decode_call_resolution(reader)
        ty = destack._generated.dir.type.type.decode_global_type_id(reader)

        return ProjectionCall(
            call=call,
            ty=ty,
        )
    elif variant == 4:
        fields = [decode_object_rest_field(reader) for _ in range(reader.read_number())]
        ty = destack._generated.dir.type.type.decode_global_type_id(reader)

        return ProjectionObjectRest(
            fields=fields,
            ty=ty,
        )
    elif variant == 5:
        ty = destack._generated.dir.type.type.decode_global_type_id(reader)

        return ProjectionSliceLength(
            ty=ty,
        )
    elif variant == 6:
        ty = destack._generated.dir.type.type.decode_global_type_id(reader)

        return ProjectionDynamicPayload(
            ty=ty,
        )
    elif variant == 7:
        ty = destack._generated.dir.type.type.decode_global_type_id(reader)

        return ProjectionDynamicType(
            ty=ty,
        )
    elif variant == 8:
        ty = destack._generated.dir.type.type.decode_global_type_id(reader)

        return ProjectionVariantTag(
            ty=ty,
        )
    elif variant == 9:
        case = decode_variant_case(reader)
        generic_arguments = [
            destack._generated.dir.type.generic.decode_generic_argument_binding(reader)
            for _ in range(reader.read_number())
        ]
        discriminant = destack._generated.dir.tree.literal.decode_scalar_literal(reader)
        ty = destack._generated.dir.type.type.decode_global_type_id(reader)

        return ProjectionVariantPayload(
            case=case,
            generic_arguments=generic_arguments,
            discriminant=discriminant,
            ty=ty,
        )
    elif variant == 10:
        symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
        generic_arguments = [
            destack._generated.dir.type.generic.decode_generic_argument_binding(reader)
            for _ in range(reader.read_number())
        ]
        ty = destack._generated.dir.type.type.decode_global_type_id(reader)

        return ProjectionNewtypePayload(
            symbol=symbol,
            generic_arguments=generic_arguments,
            ty=ty,
        )
    elif variant == 11:
        access = reader.read_option(
            lambda: destack._generated.dir.type.type.decode_access(reader)
        )
        ty = destack._generated.dir.type.type.decode_global_type_id(reader)

        return ProjectionBorrow(
            access=access,
            ty=ty,
        )
    elif variant == 12:
        access = reader.read_option(
            lambda: destack._generated.dir.type.type.decode_access(reader)
        )
        ty = destack._generated.dir.type.type.decode_global_type_id(reader)

        return ProjectionMove(
            access=access,
            ty=ty,
        )
    elif variant == 13:
        read = decode_dereference_operation(reader)
        ty = destack._generated.dir.type.type.decode_global_type_id(reader)

        return ProjectionDereference(
            read=read,
            ty=ty,
        )
    elif variant == 14:
        ty = destack._generated.dir.type.type.decode_global_type_id(reader)

        return ProjectionCopy(
            ty=ty,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_projection(value: Projection) -> Json:
    """Return one JSON value for one Projection."""
    if value.kind == "fieldGet":
        return {
            "kind": "fieldGet",
            "field": to_json_projection_field(value.field),
            "ty": destack._generated.dir.type.type.to_json_global_type_id(value.ty),
        }
    elif value.kind == "propertyGet":
        return {
            "kind": "propertyGet",
            "read": destack._generated.dir.type.resolution.to_json_member_resolution(
                value.read
            ),
            "ty": destack._generated.dir.type.type.to_json_global_type_id(value.ty),
        }
    elif value.kind == "subscriptGet":
        return {
            "kind": "subscriptGet",
            "index": destack._generated.dir.tree.node.to_json_global_node_id_any(
                value.index
            ),
            "read": to_json_subscript_operation(value.read),
            "ty": destack._generated.dir.type.type.to_json_global_type_id(value.ty),
        }
    elif value.kind == "call":
        return {
            "kind": "call",
            "call": destack._generated.dir.type.resolution.to_json_call_resolution(
                value.call
            ),
            "ty": destack._generated.dir.type.type.to_json_global_type_id(value.ty),
        }
    elif value.kind == "objectRest":
        return {
            "kind": "objectRest",
            "fields": [to_json_object_rest_field(item_0) for item_0 in value.fields],
            "ty": destack._generated.dir.type.type.to_json_global_type_id(value.ty),
        }
    elif value.kind == "sliceLength":
        return {
            "kind": "sliceLength",
            "ty": destack._generated.dir.type.type.to_json_global_type_id(value.ty),
        }
    elif value.kind == "dynamicPayload":
        return {
            "kind": "dynamicPayload",
            "ty": destack._generated.dir.type.type.to_json_global_type_id(value.ty),
        }
    elif value.kind == "dynamicType":
        return {
            "kind": "dynamicType",
            "ty": destack._generated.dir.type.type.to_json_global_type_id(value.ty),
        }
    elif value.kind == "variantTag":
        return {
            "kind": "variantTag",
            "ty": destack._generated.dir.type.type.to_json_global_type_id(value.ty),
        }
    elif value.kind == "variantPayload":
        return {
            "kind": "variantPayload",
            "case": to_json_variant_case(value.case),
            "genericArguments": [
                destack._generated.dir.type.generic.to_json_generic_argument_binding(
                    item_0
                )
                for item_0 in value.generic_arguments
            ],
            "discriminant": destack._generated.dir.tree.literal.to_json_scalar_literal(
                value.discriminant
            ),
            "ty": destack._generated.dir.type.type.to_json_global_type_id(value.ty),
        }
    elif value.kind == "newtypePayload":
        return {
            "kind": "newtypePayload",
            "symbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
                value.symbol
            ),
            "genericArguments": [
                destack._generated.dir.type.generic.to_json_generic_argument_binding(
                    item_0
                )
                for item_0 in value.generic_arguments
            ],
            "ty": destack._generated.dir.type.type.to_json_global_type_id(value.ty),
        }
    elif value.kind == "borrow":
        return {
            "kind": "borrow",
            **(
                {}
                if value.access is None
                else {
                    "access": destack._generated.dir.type.type.to_json_access(
                        value.access
                    )
                }
            ),
            "ty": destack._generated.dir.type.type.to_json_global_type_id(value.ty),
        }
    elif value.kind == "move":
        return {
            "kind": "move",
            **(
                {}
                if value.access is None
                else {
                    "access": destack._generated.dir.type.type.to_json_access(
                        value.access
                    )
                }
            ),
            "ty": destack._generated.dir.type.type.to_json_global_type_id(value.ty),
        }
    elif value.kind == "dereference":
        return {
            "kind": "dereference",
            "read": to_json_dereference_operation(value.read),
            "ty": destack._generated.dir.type.type.to_json_global_type_id(value.ty),
        }
    elif value.kind == "copy":
        return {
            "kind": "copy",
            "ty": destack._generated.dir.type.type.to_json_global_type_id(value.ty),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_projection(value: Json) -> Projection:
    """Return one Projection from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "fieldGet":
        return ProjectionFieldGet(
            field=from_json_projection_field(json_field(object_, "field")),
            ty=destack._generated.dir.type.type.from_json_global_type_id(
                json_field(object_, "ty")
            ),
        )
    elif kind == "propertyGet":
        return ProjectionPropertyGet(
            read=destack._generated.dir.type.resolution.from_json_member_resolution(
                json_field(object_, "read")
            ),
            ty=destack._generated.dir.type.type.from_json_global_type_id(
                json_field(object_, "ty")
            ),
        )
    elif kind == "subscriptGet":
        return ProjectionSubscriptGet(
            index=destack._generated.dir.tree.node.from_json_global_node_id_any(
                json_field(object_, "index")
            ),
            read=from_json_subscript_operation(json_field(object_, "read")),
            ty=destack._generated.dir.type.type.from_json_global_type_id(
                json_field(object_, "ty")
            ),
        )
    elif kind == "call":
        return ProjectionCall(
            call=destack._generated.dir.type.resolution.from_json_call_resolution(
                json_field(object_, "call")
            ),
            ty=destack._generated.dir.type.type.from_json_global_type_id(
                json_field(object_, "ty")
            ),
        )
    elif kind == "objectRest":
        return ProjectionObjectRest(
            fields=[
                from_json_object_rest_field(item_0)
                for item_0 in json_array(json_field(object_, "fields"))
            ],
            ty=destack._generated.dir.type.type.from_json_global_type_id(
                json_field(object_, "ty")
            ),
        )
    elif kind == "sliceLength":
        return ProjectionSliceLength(
            ty=destack._generated.dir.type.type.from_json_global_type_id(
                json_field(object_, "ty")
            ),
        )
    elif kind == "dynamicPayload":
        return ProjectionDynamicPayload(
            ty=destack._generated.dir.type.type.from_json_global_type_id(
                json_field(object_, "ty")
            ),
        )
    elif kind == "dynamicType":
        return ProjectionDynamicType(
            ty=destack._generated.dir.type.type.from_json_global_type_id(
                json_field(object_, "ty")
            ),
        )
    elif kind == "variantTag":
        return ProjectionVariantTag(
            ty=destack._generated.dir.type.type.from_json_global_type_id(
                json_field(object_, "ty")
            ),
        )
    elif kind == "variantPayload":
        return ProjectionVariantPayload(
            case=from_json_variant_case(json_field(object_, "case")),
            generic_arguments=[
                destack._generated.dir.type.generic.from_json_generic_argument_binding(
                    item_0
                )
                for item_0 in json_array(json_field(object_, "genericArguments"))
            ],
            discriminant=destack._generated.dir.tree.literal.from_json_scalar_literal(
                json_field(object_, "discriminant")
            ),
            ty=destack._generated.dir.type.type.from_json_global_type_id(
                json_field(object_, "ty")
            ),
        )
    elif kind == "newtypePayload":
        return ProjectionNewtypePayload(
            symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
                json_field(object_, "symbol")
            ),
            generic_arguments=[
                destack._generated.dir.type.generic.from_json_generic_argument_binding(
                    item_0
                )
                for item_0 in json_array(json_field(object_, "genericArguments"))
            ],
            ty=destack._generated.dir.type.type.from_json_global_type_id(
                json_field(object_, "ty")
            ),
        )
    elif kind == "borrow":
        return ProjectionBorrow(
            access=json_optional(
                object_,
                "access",
                lambda value: destack._generated.dir.type.type.from_json_access(value),
            ),
            ty=destack._generated.dir.type.type.from_json_global_type_id(
                json_field(object_, "ty")
            ),
        )
    elif kind == "move":
        return ProjectionMove(
            access=json_optional(
                object_,
                "access",
                lambda value: destack._generated.dir.type.type.from_json_access(value),
            ),
            ty=destack._generated.dir.type.type.from_json_global_type_id(
                json_field(object_, "ty")
            ),
        )
    elif kind == "dereference":
        return ProjectionDereference(
            read=from_json_dereference_operation(json_field(object_, "read")),
            ty=destack._generated.dir.type.type.from_json_global_type_id(
                json_field(object_, "ty")
            ),
        )
    elif kind == "copy":
        return ProjectionCopy(
            ty=destack._generated.dir.type.type.from_json_global_type_id(
                json_field(object_, "ty")
            ),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class ProjectionFieldKey:
    """Structural field key."""

    key: destack._generated.dir.symbol.key.StaticKey
    kind: typing.Literal["key"] = "key"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_projection_field(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_projection_field(self)


@dataclass(frozen=True, slots=True)
class ProjectionFieldMember:
    """Declaration-backed nominal stored member."""

    member: destack._generated.dir.symbol.symbol.GlobalSymbolId
    kind: typing.Literal["member"] = "member"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_projection_field(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_projection_field(self)


"""Static field selected by one projection."""
ProjectionField: typing.TypeAlias = ProjectionFieldKey | ProjectionFieldMember


def encode_projection_field(writer: BinaryWriter, value: ProjectionField) -> None:
    """Encode one ProjectionField."""
    if value.kind == "key":
        writer.write_unsigned(0)
        destack._generated.dir.symbol.key.encode_static_key(writer, value.key)
    elif value.kind == "member":
        writer.write_unsigned(1)
        destack._generated.dir.symbol.symbol.encode_global_symbol_id(
            writer, value.member
        )
    else:
        raise SerdeError("unknown enum variant")


def decode_projection_field(reader: BinaryReader) -> ProjectionField:
    """Decode one ProjectionField."""
    variant = reader.read_number()

    if variant == 0:
        key = destack._generated.dir.symbol.key.decode_static_key(reader)

        return ProjectionFieldKey(key=key)
    elif variant == 1:
        member = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)

        return ProjectionFieldMember(member=member)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_projection_field(value: ProjectionField) -> Json:
    """Return one JSON value for one ProjectionField."""
    if value.kind == "key":
        return {
            "kind": "key",
            "key": destack._generated.dir.symbol.key.to_json_static_key(value.key),
        }
    elif value.kind == "member":
        return {
            "kind": "member",
            "member": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
                value.member
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_projection_field(value: Json) -> ProjectionField:
    """Return one ProjectionField from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "key":
        return ProjectionFieldKey(
            key=destack._generated.dir.symbol.key.from_json_static_key(
                json_field(object_, "key")
            )
        )
    elif kind == "member":
        return ProjectionFieldMember(
            member=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
                json_field(object_, "member")
            )
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class SubscriptOperationMember:
    """Structural tuple, field, or index-signature selection."""

    member: destack._generated.dir.type.resolution.MemberResolution
    kind: typing.Literal["member"] = "member"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_subscript_operation(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_subscript_operation(self)


@dataclass(frozen=True, slots=True)
class SubscriptOperationCall:
    """Protocol-backed subscript call."""

    call: destack._generated.dir.type.resolution.CallResolution
    kind: typing.Literal["call"] = "call"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_subscript_operation(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_subscript_operation(self)


"""Subscript operation selected by one projection or place."""
SubscriptOperation: typing.TypeAlias = SubscriptOperationMember | SubscriptOperationCall


def encode_subscript_operation(writer: BinaryWriter, value: SubscriptOperation) -> None:
    """Encode one SubscriptOperation."""
    if value.kind == "member":
        writer.write_unsigned(0)
        destack._generated.dir.type.resolution.encode_member_resolution(
            writer, value.member
        )
    elif value.kind == "call":
        writer.write_unsigned(1)
        destack._generated.dir.type.resolution.encode_call_resolution(
            writer, value.call
        )
    else:
        raise SerdeError("unknown enum variant")


def decode_subscript_operation(reader: BinaryReader) -> SubscriptOperation:
    """Decode one SubscriptOperation."""
    variant = reader.read_number()

    if variant == 0:
        member = destack._generated.dir.type.resolution.decode_member_resolution(reader)

        return SubscriptOperationMember(member=member)
    elif variant == 1:
        call = destack._generated.dir.type.resolution.decode_call_resolution(reader)

        return SubscriptOperationCall(call=call)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_subscript_operation(value: SubscriptOperation) -> Json:
    """Return one JSON value for one SubscriptOperation."""
    if value.kind == "member":
        return {
            "kind": "member",
            "member": destack._generated.dir.type.resolution.to_json_member_resolution(
                value.member
            ),
        }
    elif value.kind == "call":
        return {
            "kind": "call",
            "call": destack._generated.dir.type.resolution.to_json_call_resolution(
                value.call
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_subscript_operation(value: Json) -> SubscriptOperation:
    """Return one SubscriptOperation from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "member":
        return SubscriptOperationMember(
            member=destack._generated.dir.type.resolution.from_json_member_resolution(
                json_field(object_, "member")
            )
        )
    elif kind == "call":
        return SubscriptOperationCall(
            call=destack._generated.dir.type.resolution.from_json_call_resolution(
                json_field(object_, "call")
            )
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class ObjectRestField:
    """One source field used to materialize an object rest value."""

    # the materialized field key
    key: destack._generated.dir.symbol.key.StaticKey
    # the selected source projection
    projection: Projection

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_object_rest_field(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ObjectRestField:
        """Decode one ObjectRestField."""
        return decode_object_rest_field(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_object_rest_field(self)

    @classmethod
    def from_json(cls, value: Json) -> ObjectRestField:
        """Return one ObjectRestField from one JSON value."""
        return from_json_object_rest_field(value)


def encode_object_rest_field(writer: BinaryWriter, value: ObjectRestField) -> None:
    """Encode one ObjectRestField."""
    destack._generated.dir.symbol.key.encode_static_key(writer, value.key)
    encode_projection(writer, value.projection)


def decode_object_rest_field(reader: BinaryReader) -> ObjectRestField:
    """Decode one ObjectRestField."""
    key = destack._generated.dir.symbol.key.decode_static_key(reader)
    projection = decode_projection(reader)

    return ObjectRestField(
        key=key,
        projection=projection,
    )


def to_json_object_rest_field(value: ObjectRestField) -> Json:
    """Return one JSON value for one ObjectRestField."""
    return {
        "key": destack._generated.dir.symbol.key.to_json_static_key(value.key),
        "projection": to_json_projection(value.projection),
    }


def from_json_object_rest_field(value: Json) -> ObjectRestField:
    """Return one ObjectRestField from one JSON value."""
    object_ = json_object(value)

    return ObjectRestField(
        key=destack._generated.dir.symbol.key.from_json_static_key(
            json_field(object_, "key")
        ),
        projection=from_json_projection(json_field(object_, "projection")),
    )


@dataclass(frozen=True, slots=True)
class VariantCase:
    """One tagged union case selected during checking."""

    # the selected variant family symbol
    owner: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the source-level case key
    key: destack._generated.dir.symbol.key.StaticKey
    # the selected variant declaration
    member: destack._generated.dir.symbol.symbol.GlobalSymbolId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_variant_case(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> VariantCase:
        """Decode one VariantCase."""
        return decode_variant_case(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_variant_case(self)

    @classmethod
    def from_json(cls, value: Json) -> VariantCase:
        """Return one VariantCase from one JSON value."""
        return from_json_variant_case(value)


def encode_variant_case(writer: BinaryWriter, value: VariantCase) -> None:
    """Encode one VariantCase."""
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(writer, value.owner)
    destack._generated.dir.symbol.key.encode_static_key(writer, value.key)
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(writer, value.member)


def decode_variant_case(reader: BinaryReader) -> VariantCase:
    """Decode one VariantCase."""
    owner = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    key = destack._generated.dir.symbol.key.decode_static_key(reader)
    member = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)

    return VariantCase(
        owner=owner,
        key=key,
        member=member,
    )


def to_json_variant_case(value: VariantCase) -> Json:
    """Return one JSON value for one VariantCase."""
    return {
        "owner": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.owner
        ),
        "key": destack._generated.dir.symbol.key.to_json_static_key(value.key),
        "member": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.member
        ),
    }


def from_json_variant_case(value: Json) -> VariantCase:
    """Return one VariantCase from one JSON value."""
    object_ = json_object(value)

    return VariantCase(
        owner=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "owner")
        ),
        key=destack._generated.dir.symbol.key.from_json_static_key(
            json_field(object_, "key")
        ),
        member=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "member")
        ),
    )


@dataclass(frozen=True, slots=True)
class DereferenceOperationDirect:
    """Direct dereference of a physical reference or pointer form."""

    kind: typing.Literal["direct"] = "direct"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_dereference_operation(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_dereference_operation(self)


@dataclass(frozen=True, slots=True)
class DereferenceOperationCall:
    """Protocol-backed dereference call."""

    call: destack._generated.dir.type.resolution.CallResolution
    kind: typing.Literal["call"] = "call"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_dereference_operation(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_dereference_operation(self)


"""Dereference operation selected by one projection or place."""
DereferenceOperation: typing.TypeAlias = (
    DereferenceOperationDirect | DereferenceOperationCall
)


def encode_dereference_operation(
    writer: BinaryWriter, value: DereferenceOperation
) -> None:
    """Encode one DereferenceOperation."""
    if value.kind == "direct":
        writer.write_unsigned(0)
    elif value.kind == "call":
        writer.write_unsigned(1)
        destack._generated.dir.type.resolution.encode_call_resolution(
            writer, value.call
        )
    else:
        raise SerdeError("unknown enum variant")


def decode_dereference_operation(reader: BinaryReader) -> DereferenceOperation:
    """Decode one DereferenceOperation."""
    variant = reader.read_number()

    if variant == 0:
        return DereferenceOperationDirect()
    elif variant == 1:
        call = destack._generated.dir.type.resolution.decode_call_resolution(reader)

        return DereferenceOperationCall(call=call)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_dereference_operation(value: DereferenceOperation) -> Json:
    """Return one JSON value for one DereferenceOperation."""
    if value.kind == "direct":
        return {
            "kind": "direct",
        }
    elif value.kind == "call":
        return {
            "kind": "call",
            "call": destack._generated.dir.type.resolution.to_json_call_resolution(
                value.call
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_dereference_operation(value: Json) -> DereferenceOperation:
    """Return one DereferenceOperation from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "direct":
        return DereferenceOperationDirect()
    elif kind == "call":
        return DereferenceOperationCall(
            call=destack._generated.dir.type.resolution.from_json_call_resolution(
                json_field(object_, "call")
            )
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


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
