# generated bridge target, do not edit

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
    json_object,
    json_optional,
    json_string,
)

from destack._impl.dir.tree.property import (
    PropertyImpl,
)
from destack._impl.dir.tree.property import (
    MemberImpl,
)

import destack._generated.core.string
import destack._generated.dir.symbol.key
import destack._generated.dir.tree.function
import destack._generated.dir.tree.key
import destack._generated.dir.tree.node

"""The special role of a function."""
FunctionRole: typing.TypeAlias = (
    typing.Literal["getter"]
    | typing.Literal["setter"]
    | typing.Literal["constructor"]
    | typing.Literal["new"]
    | typing.Literal["call"]
)


def encode_function_role(writer: BinaryWriter, value: FunctionRole) -> None:
    """Encode one FunctionRole."""
    if value == "getter":
        writer.write_unsigned(0)
    elif value == "setter":
        writer.write_unsigned(1)
    elif value == "constructor":
        writer.write_unsigned(2)
    elif value == "new":
        writer.write_unsigned(3)
    elif value == "call":
        writer.write_unsigned(4)
    else:
        raise SerdeError("unknown enum variant")


def decode_function_role(reader: BinaryReader) -> FunctionRole:
    """Decode one FunctionRole."""
    variant = reader.read_number()

    if variant == 0:
        return "getter"
    elif variant == 1:
        return "setter"
    elif variant == 2:
        return "constructor"
    elif variant == 3:
        return "new"
    elif variant == 4:
        return "call"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_function_role(value: FunctionRole) -> Json:
    """Return one JSON value for one FunctionRole."""
    return value


def from_json_function_role(value: Json) -> FunctionRole:
    """Return one FunctionRole from one JSON value."""
    variant = json_string(value)

    if variant == "getter":
        return "getter"
    elif variant == "setter":
        return "setter"
    elif variant == "constructor":
        return "constructor"
    elif variant == "new":
        return "new"
    elif variant == "call":
        return "call"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class PropertyField(PropertyImpl):
    """Named field."""

    key: destack._generated.dir.tree.key.Key
    value: destack._generated.dir.tree.node.LocalNodeId
    is_shorthand: bool
    kind: typing.Literal["field"] = "field"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_property(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_property(self)


@dataclass(frozen=True, slots=True)
class PropertyMethod(PropertyImpl):
    """Object-like member function."""

    key: destack._generated.dir.tree.key.Key | None
    signature: destack._generated.dir.tree.function.FunctionSignature
    body: destack._generated.dir.tree.node.LocalNodeId | None
    kind: typing.Literal["method"] = "method"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_property(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_property(self)


@dataclass(frozen=True, slots=True)
class PropertySpread(PropertyImpl):
    """Spread property."""

    value: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["spread"] = "spread"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_property(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_property(self)


@dataclass(frozen=True, slots=True)
class PropertyError(PropertyImpl):
    """Malformed property slot."""

    kind: typing.Literal["error"] = "error"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_property(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_property(self)


"""A property of an object-like literal."""
Property: typing.TypeAlias = (
    PropertyField | PropertyMethod | PropertySpread | PropertyError
)


def encode_property(writer: BinaryWriter, value: Property) -> None:
    """Encode one Property."""
    if value.kind == "field":
        writer.write_unsigned(0)
        destack._generated.dir.tree.key.encode_key(writer, value.key)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.value)
        writer.write_bool(value.is_shorthand)
    elif value.kind == "method":
        writer.write_unsigned(1)
        if value.key is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.key.encode_key(writer, value.key)
        destack._generated.dir.tree.function.encode_function_signature(
            writer, value.signature
        )
        if value.body is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_local_node_id(writer, value.body)
    elif value.kind == "spread":
        writer.write_unsigned(2)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.value)
    elif value.kind == "error":
        writer.write_unsigned(3)
    else:
        raise SerdeError("unknown enum variant")


def decode_property(reader: BinaryReader) -> Property:
    """Decode one Property."""
    variant = reader.read_number()

    if variant == 0:
        key = destack._generated.dir.tree.key.decode_key(reader)
        value_ = destack._generated.dir.tree.node.decode_local_node_id(reader)
        is_shorthand = reader.read_bool()

        return PropertyField(
            key=key,
            value=value_,
            is_shorthand=is_shorthand,
        )
    elif variant == 1:
        key = reader.read_option(
            lambda: destack._generated.dir.tree.key.decode_key(reader)
        )
        signature = destack._generated.dir.tree.function.decode_function_signature(
            reader
        )
        body = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )

        return PropertyMethod(
            key=key,
            signature=signature,
            body=body,
        )
    elif variant == 2:
        value_ = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return PropertySpread(
            value=value_,
        )
    elif variant == 3:
        return PropertyError()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_property(value: Property) -> Json:
    """Return one JSON value for one Property."""
    if value.kind == "field":
        return {
            "kind": "field",
            "key": destack._generated.dir.tree.key.to_json_key(value.key),
            "value": destack._generated.dir.tree.node.to_json_local_node_id(
                value.value
            ),
            "isShorthand": value.is_shorthand,
        }
    elif value.kind == "method":
        return {
            "kind": "method",
            **(
                {}
                if value.key is None
                else {"key": destack._generated.dir.tree.key.to_json_key(value.key)}
            ),
            "signature": destack._generated.dir.tree.function.to_json_function_signature(
                value.signature
            ),
            **(
                {}
                if value.body is None
                else {
                    "body": destack._generated.dir.tree.node.to_json_local_node_id(
                        value.body
                    )
                }
            ),
        }
    elif value.kind == "spread":
        return {
            "kind": "spread",
            "value": destack._generated.dir.tree.node.to_json_local_node_id(
                value.value
            ),
        }
    elif value.kind == "error":
        return {
            "kind": "error",
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_property(value: Json) -> Property:
    """Return one Property from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "field":
        return PropertyField(
            key=destack._generated.dir.tree.key.from_json_key(
                json_field(object_, "key")
            ),
            value=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "value")
            ),
            is_shorthand=json_bool(json_field(object_, "isShorthand")),
        )
    elif kind == "method":
        return PropertyMethod(
            key=json_optional(
                object_,
                "key",
                lambda value: destack._generated.dir.tree.key.from_json_key(value),
            ),
            signature=destack._generated.dir.tree.function.from_json_function_signature(
                json_field(object_, "signature")
            ),
            body=json_optional(
                object_,
                "body",
                lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                    value
                ),
            ),
        )
    elif kind == "spread":
        return PropertySpread(
            value=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "value")
            ),
        )
    elif kind == "error":
        return PropertyError()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class MemberAssociatedType(MemberImpl):
    """Associated type alias."""

    name: destack._generated.core.string.StringId
    generic_parameters: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    where_clauses: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    constraint: destack._generated.dir.tree.node.LocalNodeId | None
    value: destack._generated.dir.tree.node.LocalNodeId | None
    visibility: destack._generated.dir.tree.node.Visibility | None
    is_ambient: bool
    is_abstract: bool
    is_override: bool
    kind: typing.Literal["associatedType"] = "associatedType"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_member(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_member(self)


@dataclass(frozen=True, slots=True)
class MemberAssociatedConst(MemberImpl):
    """Associated compile-time constant."""

    name: destack._generated.core.string.StringId
    declared_type: destack._generated.dir.tree.node.LocalNodeId | None
    value: destack._generated.dir.tree.node.LocalNodeId | None
    visibility: destack._generated.dir.tree.node.Visibility | None
    is_ambient: bool
    is_abstract: bool
    is_override: bool
    kind: typing.Literal["associatedConst"] = "associatedConst"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_member(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_member(self)


@dataclass(frozen=True, slots=True)
class MemberField(MemberImpl):
    """Named field."""

    key: destack._generated.dir.tree.key.Key
    declared_type: destack._generated.dir.tree.node.LocalNodeId | None
    default: destack._generated.dir.tree.node.LocalNodeId | None
    mutability: destack._generated.dir.tree.node.Mutability | None
    visibility: destack._generated.dir.tree.node.Visibility | None
    is_optional: bool
    is_definite: bool
    is_readonly: bool
    is_ambient: bool
    is_abstract: bool
    is_override: bool
    is_static: bool
    is_accessor: bool
    kind: typing.Literal["field"] = "field"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_member(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_member(self)


@dataclass(frozen=True, slots=True)
class MemberMethod(MemberImpl):
    """Named member function."""

    key: destack._generated.dir.tree.key.Key | None
    signature: destack._generated.dir.tree.function.FunctionSignature
    abstraction: MethodAbstraction
    body: destack._generated.dir.tree.node.LocalNodeId | None
    visibility: destack._generated.dir.tree.node.Visibility | None
    is_optional: bool
    is_ambient: bool
    is_override: bool
    is_static: bool
    is_accessor: bool
    kind: typing.Literal["method"] = "method"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_member(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_member(self)


@dataclass(frozen=True, slots=True)
class MemberStaticBlock(MemberImpl):
    """Static initialization block."""

    body: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["staticBlock"] = "staticBlock"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_member(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_member(self)


@dataclass(frozen=True, slots=True)
class MemberComptimeBlock(MemberImpl):
    """Comptime block."""

    body: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["comptimeBlock"] = "comptimeBlock"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_member(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_member(self)


@dataclass(frozen=True, slots=True)
class MemberError(MemberImpl):
    """Malformed member slot."""

    kind: typing.Literal["error"] = "error"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_member(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_member(self)


"""A member of a declaration body."""
Member: typing.TypeAlias = (
    MemberAssociatedType
    | MemberAssociatedConst
    | MemberField
    | MemberMethod
    | MemberStaticBlock
    | MemberComptimeBlock
    | MemberError
)


def encode_member(writer: BinaryWriter, value: Member) -> None:
    """Encode one Member."""
    if value.kind == "associatedType":
        writer.write_unsigned(0)
        destack._generated.core.string.encode_string_id(writer, value.name)
        writer.write_unsigned(len(value.generic_parameters))
        for item_value_generic_parameters_0 in value.generic_parameters:
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, item_value_generic_parameters_0
            )
        writer.write_unsigned(len(value.where_clauses))
        for item_value_where_clauses_0 in value.where_clauses:
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, item_value_where_clauses_0
            )
        if value.constraint is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, value.constraint
            )
        if value.value is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_local_node_id(writer, value.value)
        if value.visibility is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_visibility(writer, value.visibility)
        writer.write_bool(value.is_ambient)
        writer.write_bool(value.is_abstract)
        writer.write_bool(value.is_override)
    elif value.kind == "associatedConst":
        writer.write_unsigned(1)
        destack._generated.core.string.encode_string_id(writer, value.name)
        if value.declared_type is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, value.declared_type
            )
        if value.value is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_local_node_id(writer, value.value)
        if value.visibility is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_visibility(writer, value.visibility)
        writer.write_bool(value.is_ambient)
        writer.write_bool(value.is_abstract)
        writer.write_bool(value.is_override)
    elif value.kind == "field":
        writer.write_unsigned(2)
        destack._generated.dir.tree.key.encode_key(writer, value.key)
        if value.declared_type is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, value.declared_type
            )
        if value.default is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_local_node_id(writer, value.default)
        if value.mutability is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_mutability(writer, value.mutability)
        if value.visibility is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_visibility(writer, value.visibility)
        writer.write_bool(value.is_optional)
        writer.write_bool(value.is_definite)
        writer.write_bool(value.is_readonly)
        writer.write_bool(value.is_ambient)
        writer.write_bool(value.is_abstract)
        writer.write_bool(value.is_override)
        writer.write_bool(value.is_static)
        writer.write_bool(value.is_accessor)
    elif value.kind == "method":
        writer.write_unsigned(3)
        if value.key is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.key.encode_key(writer, value.key)
        destack._generated.dir.tree.function.encode_function_signature(
            writer, value.signature
        )
        encode_method_abstraction(writer, value.abstraction)
        if value.body is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_local_node_id(writer, value.body)
        if value.visibility is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_visibility(writer, value.visibility)
        writer.write_bool(value.is_optional)
        writer.write_bool(value.is_ambient)
        writer.write_bool(value.is_override)
        writer.write_bool(value.is_static)
        writer.write_bool(value.is_accessor)
    elif value.kind == "staticBlock":
        writer.write_unsigned(4)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.body)
    elif value.kind == "comptimeBlock":
        writer.write_unsigned(5)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.body)
    elif value.kind == "error":
        writer.write_unsigned(6)
    else:
        raise SerdeError("unknown enum variant")


def decode_member(reader: BinaryReader) -> Member:
    """Decode one Member."""
    variant = reader.read_number()

    if variant == 0:
        name = destack._generated.core.string.decode_string_id(reader)
        generic_parameters = [
            destack._generated.dir.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]
        where_clauses = [
            destack._generated.dir.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]
        constraint = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )
        value_ = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )
        visibility = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_visibility(reader)
        )
        is_ambient = reader.read_bool()
        is_abstract = reader.read_bool()
        is_override = reader.read_bool()

        return MemberAssociatedType(
            name=name,
            generic_parameters=generic_parameters,
            where_clauses=where_clauses,
            constraint=constraint,
            value=value_,
            visibility=visibility,
            is_ambient=is_ambient,
            is_abstract=is_abstract,
            is_override=is_override,
        )
    elif variant == 1:
        name = destack._generated.core.string.decode_string_id(reader)
        declared_type = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )
        value_ = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )
        visibility = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_visibility(reader)
        )
        is_ambient = reader.read_bool()
        is_abstract = reader.read_bool()
        is_override = reader.read_bool()

        return MemberAssociatedConst(
            name=name,
            declared_type=declared_type,
            value=value_,
            visibility=visibility,
            is_ambient=is_ambient,
            is_abstract=is_abstract,
            is_override=is_override,
        )
    elif variant == 2:
        key = destack._generated.dir.tree.key.decode_key(reader)
        declared_type = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )
        default = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )
        mutability = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_mutability(reader)
        )
        visibility = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_visibility(reader)
        )
        is_optional = reader.read_bool()
        is_definite = reader.read_bool()
        is_readonly = reader.read_bool()
        is_ambient = reader.read_bool()
        is_abstract = reader.read_bool()
        is_override = reader.read_bool()
        is_static = reader.read_bool()
        is_accessor = reader.read_bool()

        return MemberField(
            key=key,
            declared_type=declared_type,
            default=default,
            mutability=mutability,
            visibility=visibility,
            is_optional=is_optional,
            is_definite=is_definite,
            is_readonly=is_readonly,
            is_ambient=is_ambient,
            is_abstract=is_abstract,
            is_override=is_override,
            is_static=is_static,
            is_accessor=is_accessor,
        )
    elif variant == 3:
        key = reader.read_option(
            lambda: destack._generated.dir.tree.key.decode_key(reader)
        )
        signature = destack._generated.dir.tree.function.decode_function_signature(
            reader
        )
        abstraction = decode_method_abstraction(reader)
        body = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )
        visibility = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_visibility(reader)
        )
        is_optional = reader.read_bool()
        is_ambient = reader.read_bool()
        is_override = reader.read_bool()
        is_static = reader.read_bool()
        is_accessor = reader.read_bool()

        return MemberMethod(
            key=key,
            signature=signature,
            abstraction=abstraction,
            body=body,
            visibility=visibility,
            is_optional=is_optional,
            is_ambient=is_ambient,
            is_override=is_override,
            is_static=is_static,
            is_accessor=is_accessor,
        )
    elif variant == 4:
        body = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return MemberStaticBlock(
            body=body,
        )
    elif variant == 5:
        body = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return MemberComptimeBlock(
            body=body,
        )
    elif variant == 6:
        return MemberError()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_member(value: Member) -> Json:
    """Return one JSON value for one Member."""
    if value.kind == "associatedType":
        return {
            "kind": "associatedType",
            "name": destack._generated.core.string.to_json_string_id(value.name),
            "genericParameters": [
                destack._generated.dir.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.generic_parameters
            ],
            "whereClauses": [
                destack._generated.dir.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.where_clauses
            ],
            **(
                {}
                if value.constraint is None
                else {
                    "constraint": destack._generated.dir.tree.node.to_json_local_node_id(
                        value.constraint
                    )
                }
            ),
            **(
                {}
                if value.value is None
                else {
                    "value": destack._generated.dir.tree.node.to_json_local_node_id(
                        value.value
                    )
                }
            ),
            **(
                {}
                if value.visibility is None
                else {
                    "visibility": destack._generated.dir.tree.node.to_json_visibility(
                        value.visibility
                    )
                }
            ),
            "isAmbient": value.is_ambient,
            "isAbstract": value.is_abstract,
            "isOverride": value.is_override,
        }
    elif value.kind == "associatedConst":
        return {
            "kind": "associatedConst",
            "name": destack._generated.core.string.to_json_string_id(value.name),
            **(
                {}
                if value.declared_type is None
                else {
                    "declaredType": destack._generated.dir.tree.node.to_json_local_node_id(
                        value.declared_type
                    )
                }
            ),
            **(
                {}
                if value.value is None
                else {
                    "value": destack._generated.dir.tree.node.to_json_local_node_id(
                        value.value
                    )
                }
            ),
            **(
                {}
                if value.visibility is None
                else {
                    "visibility": destack._generated.dir.tree.node.to_json_visibility(
                        value.visibility
                    )
                }
            ),
            "isAmbient": value.is_ambient,
            "isAbstract": value.is_abstract,
            "isOverride": value.is_override,
        }
    elif value.kind == "field":
        return {
            "kind": "field",
            "key": destack._generated.dir.tree.key.to_json_key(value.key),
            **(
                {}
                if value.declared_type is None
                else {
                    "declaredType": destack._generated.dir.tree.node.to_json_local_node_id(
                        value.declared_type
                    )
                }
            ),
            **(
                {}
                if value.default is None
                else {
                    "default": destack._generated.dir.tree.node.to_json_local_node_id(
                        value.default
                    )
                }
            ),
            **(
                {}
                if value.mutability is None
                else {
                    "mutability": destack._generated.dir.tree.node.to_json_mutability(
                        value.mutability
                    )
                }
            ),
            **(
                {}
                if value.visibility is None
                else {
                    "visibility": destack._generated.dir.tree.node.to_json_visibility(
                        value.visibility
                    )
                }
            ),
            "isOptional": value.is_optional,
            "isDefinite": value.is_definite,
            "isReadonly": value.is_readonly,
            "isAmbient": value.is_ambient,
            "isAbstract": value.is_abstract,
            "isOverride": value.is_override,
            "isStatic": value.is_static,
            "isAccessor": value.is_accessor,
        }
    elif value.kind == "method":
        return {
            "kind": "method",
            **(
                {}
                if value.key is None
                else {"key": destack._generated.dir.tree.key.to_json_key(value.key)}
            ),
            "signature": destack._generated.dir.tree.function.to_json_function_signature(
                value.signature
            ),
            "abstraction": to_json_method_abstraction(value.abstraction),
            **(
                {}
                if value.body is None
                else {
                    "body": destack._generated.dir.tree.node.to_json_local_node_id(
                        value.body
                    )
                }
            ),
            **(
                {}
                if value.visibility is None
                else {
                    "visibility": destack._generated.dir.tree.node.to_json_visibility(
                        value.visibility
                    )
                }
            ),
            "isOptional": value.is_optional,
            "isAmbient": value.is_ambient,
            "isOverride": value.is_override,
            "isStatic": value.is_static,
            "isAccessor": value.is_accessor,
        }
    elif value.kind == "staticBlock":
        return {
            "kind": "staticBlock",
            "body": destack._generated.dir.tree.node.to_json_local_node_id(value.body),
        }
    elif value.kind == "comptimeBlock":
        return {
            "kind": "comptimeBlock",
            "body": destack._generated.dir.tree.node.to_json_local_node_id(value.body),
        }
    elif value.kind == "error":
        return {
            "kind": "error",
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_member(value: Json) -> Member:
    """Return one Member from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "associatedType":
        return MemberAssociatedType(
            name=destack._generated.core.string.from_json_string_id(
                json_field(object_, "name")
            ),
            generic_parameters=[
                destack._generated.dir.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "genericParameters"))
            ],
            where_clauses=[
                destack._generated.dir.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "whereClauses"))
            ],
            constraint=json_optional(
                object_,
                "constraint",
                lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                    value
                ),
            ),
            value=json_optional(
                object_,
                "value",
                lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                    value
                ),
            ),
            visibility=json_optional(
                object_,
                "visibility",
                lambda value: destack._generated.dir.tree.node.from_json_visibility(
                    value
                ),
            ),
            is_ambient=json_bool(json_field(object_, "isAmbient")),
            is_abstract=json_bool(json_field(object_, "isAbstract")),
            is_override=json_bool(json_field(object_, "isOverride")),
        )
    elif kind == "associatedConst":
        return MemberAssociatedConst(
            name=destack._generated.core.string.from_json_string_id(
                json_field(object_, "name")
            ),
            declared_type=json_optional(
                object_,
                "declaredType",
                lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                    value
                ),
            ),
            value=json_optional(
                object_,
                "value",
                lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                    value
                ),
            ),
            visibility=json_optional(
                object_,
                "visibility",
                lambda value: destack._generated.dir.tree.node.from_json_visibility(
                    value
                ),
            ),
            is_ambient=json_bool(json_field(object_, "isAmbient")),
            is_abstract=json_bool(json_field(object_, "isAbstract")),
            is_override=json_bool(json_field(object_, "isOverride")),
        )
    elif kind == "field":
        return MemberField(
            key=destack._generated.dir.tree.key.from_json_key(
                json_field(object_, "key")
            ),
            declared_type=json_optional(
                object_,
                "declaredType",
                lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                    value
                ),
            ),
            default=json_optional(
                object_,
                "default",
                lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                    value
                ),
            ),
            mutability=json_optional(
                object_,
                "mutability",
                lambda value: destack._generated.dir.tree.node.from_json_mutability(
                    value
                ),
            ),
            visibility=json_optional(
                object_,
                "visibility",
                lambda value: destack._generated.dir.tree.node.from_json_visibility(
                    value
                ),
            ),
            is_optional=json_bool(json_field(object_, "isOptional")),
            is_definite=json_bool(json_field(object_, "isDefinite")),
            is_readonly=json_bool(json_field(object_, "isReadonly")),
            is_ambient=json_bool(json_field(object_, "isAmbient")),
            is_abstract=json_bool(json_field(object_, "isAbstract")),
            is_override=json_bool(json_field(object_, "isOverride")),
            is_static=json_bool(json_field(object_, "isStatic")),
            is_accessor=json_bool(json_field(object_, "isAccessor")),
        )
    elif kind == "method":
        return MemberMethod(
            key=json_optional(
                object_,
                "key",
                lambda value: destack._generated.dir.tree.key.from_json_key(value),
            ),
            signature=destack._generated.dir.tree.function.from_json_function_signature(
                json_field(object_, "signature")
            ),
            abstraction=from_json_method_abstraction(
                json_field(object_, "abstraction")
            ),
            body=json_optional(
                object_,
                "body",
                lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                    value
                ),
            ),
            visibility=json_optional(
                object_,
                "visibility",
                lambda value: destack._generated.dir.tree.node.from_json_visibility(
                    value
                ),
            ),
            is_optional=json_bool(json_field(object_, "isOptional")),
            is_ambient=json_bool(json_field(object_, "isAmbient")),
            is_override=json_bool(json_field(object_, "isOverride")),
            is_static=json_bool(json_field(object_, "isStatic")),
            is_accessor=json_bool(json_field(object_, "isAccessor")),
        )
    elif kind == "staticBlock":
        return MemberStaticBlock(
            body=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "body")
            ),
        )
    elif kind == "comptimeBlock":
        return MemberComptimeBlock(
            body=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "body")
            ),
        )
    elif kind == "error":
        return MemberError()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


"""The abstraction mode of a class method."""
MethodAbstraction: typing.TypeAlias = (
    typing.Literal["concrete"] | typing.Literal["virtual"] | typing.Literal["abstract"]
)


def encode_method_abstraction(writer: BinaryWriter, value: MethodAbstraction) -> None:
    """Encode one MethodAbstraction."""
    if value == "concrete":
        writer.write_unsigned(0)
    elif value == "virtual":
        writer.write_unsigned(1)
    elif value == "abstract":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_method_abstraction(reader: BinaryReader) -> MethodAbstraction:
    """Decode one MethodAbstraction."""
    variant = reader.read_number()

    if variant == 0:
        return "concrete"
    elif variant == 1:
        return "virtual"
    elif variant == 2:
        return "abstract"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_method_abstraction(value: MethodAbstraction) -> Json:
    """Return one JSON value for one MethodAbstraction."""
    return value


def from_json_method_abstraction(value: Json) -> MethodAbstraction:
    """Return one MethodAbstraction from one JSON value."""
    variant = json_string(value)

    if variant == "concrete":
        return "concrete"
    elif variant == "virtual":
        return "virtual"
    elif variant == "abstract":
        return "abstract"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Variance annotation for generic parameters."""
VarianceModifier: typing.TypeAlias = (
    typing.Literal["in"] | typing.Literal["out"] | typing.Literal["inOut"]
)


def encode_variance_modifier(writer: BinaryWriter, value: VarianceModifier) -> None:
    """Encode one VarianceModifier."""
    if value == "in":
        writer.write_unsigned(0)
    elif value == "out":
        writer.write_unsigned(1)
    elif value == "inOut":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_variance_modifier(reader: BinaryReader) -> VarianceModifier:
    """Decode one VarianceModifier."""
    variant = reader.read_number()

    if variant == 0:
        return "in"
    elif variant == 1:
        return "out"
    elif variant == 2:
        return "inOut"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_variance_modifier(value: VarianceModifier) -> Json:
    """Return one JSON value for one VarianceModifier."""
    return value


def from_json_variance_modifier(value: Json) -> VarianceModifier:
    """Return one VarianceModifier from one JSON value."""
    variant = json_string(value)

    if variant == "in":
        return "in"
    elif variant == "out":
        return "out"
    elif variant == "inOut":
        return "inOut"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class MemberSlotKey:
    """Property keyed by a static key."""

    key: destack._generated.dir.symbol.key.StaticKey
    kind: typing.Literal["key"] = "key"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_member_slot(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_member_slot(self)


@dataclass(frozen=True, slots=True)
class MemberSlotConstructor:
    """Constructor role member."""

    kind: typing.Literal["constructor"] = "constructor"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_member_slot(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_member_slot(self)


@dataclass(frozen=True, slots=True)
class MemberSlotNew:
    """New role member."""

    kind: typing.Literal["new"] = "new"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_member_slot(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_member_slot(self)


@dataclass(frozen=True, slots=True)
class MemberSlotCall:
    """Callable role member."""

    kind: typing.Literal["call"] = "call"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_member_slot(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_member_slot(self)


"""A nominal member slot."""
MemberSlot: typing.TypeAlias = (
    MemberSlotKey | MemberSlotConstructor | MemberSlotNew | MemberSlotCall
)


def encode_member_slot(writer: BinaryWriter, value: MemberSlot) -> None:
    """Encode one MemberSlot."""
    if value.kind == "key":
        writer.write_unsigned(0)
        destack._generated.dir.symbol.key.encode_static_key(writer, value.key)
    elif value.kind == "constructor":
        writer.write_unsigned(1)
    elif value.kind == "new":
        writer.write_unsigned(2)
    elif value.kind == "call":
        writer.write_unsigned(3)
    else:
        raise SerdeError("unknown enum variant")


def decode_member_slot(reader: BinaryReader) -> MemberSlot:
    """Decode one MemberSlot."""
    variant = reader.read_number()

    if variant == 0:
        key = destack._generated.dir.symbol.key.decode_static_key(reader)

        return MemberSlotKey(key=key)
    elif variant == 1:
        return MemberSlotConstructor()
    elif variant == 2:
        return MemberSlotNew()
    elif variant == 3:
        return MemberSlotCall()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_member_slot(value: MemberSlot) -> Json:
    """Return one JSON value for one MemberSlot."""
    if value.kind == "key":
        return {
            "kind": "key",
            "key": destack._generated.dir.symbol.key.to_json_static_key(value.key),
        }
    elif value.kind == "constructor":
        return {
            "kind": "constructor",
        }
    elif value.kind == "new":
        return {
            "kind": "new",
        }
    elif value.kind == "call":
        return {
            "kind": "call",
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_member_slot(value: Json) -> MemberSlot:
    """Return one MemberSlot from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "key":
        return MemberSlotKey(
            key=destack._generated.dir.symbol.key.from_json_static_key(
                json_field(object_, "key")
            )
        )
    elif kind == "constructor":
        return MemberSlotConstructor()
    elif kind == "new":
        return MemberSlotNew()
    elif kind == "call":
        return MemberSlotCall()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


__all__ = [
    "FunctionRole",
    "encode_function_role",
    "decode_function_role",
    "to_json_function_role",
    "from_json_function_role",
    "Property",
    "encode_property",
    "decode_property",
    "to_json_property",
    "from_json_property",
    "PropertyField",
    "PropertyMethod",
    "PropertySpread",
    "PropertyError",
    "Member",
    "encode_member",
    "decode_member",
    "to_json_member",
    "from_json_member",
    "MemberAssociatedType",
    "MemberAssociatedConst",
    "MemberField",
    "MemberMethod",
    "MemberStaticBlock",
    "MemberComptimeBlock",
    "MemberError",
    "MethodAbstraction",
    "encode_method_abstraction",
    "decode_method_abstraction",
    "to_json_method_abstraction",
    "from_json_method_abstraction",
    "VarianceModifier",
    "encode_variance_modifier",
    "decode_variance_modifier",
    "to_json_variance_modifier",
    "from_json_variance_modifier",
    "MemberSlot",
    "encode_member_slot",
    "decode_member_slot",
    "to_json_member_slot",
    "from_json_member_slot",
    "MemberSlotKey",
    "MemberSlotConstructor",
    "MemberSlotNew",
    "MemberSlotCall",
]
