# generated client target, do not edit

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
    json_object,
    json_optional,
    json_string,
)

from destack._impl.dir.tree.argument import (
    GenericParameterImpl,
)
from destack._impl.dir.tree.argument import (
    ParameterImpl,
)

import destack._generated.core.string
import destack._generated.dir.tree.key
import destack._generated.dir.tree.node
import destack._generated.dir.tree.property


@dataclass(frozen=True, slots=True)
class GenericParameterType(GenericParameterImpl):
    """Type parameter."""

    name: destack._generated.core.string.StringId
    variance: destack._generated.dir.tree.property.VarianceModifier | None
    constraint: destack._generated.dir.tree.node.LocalNodeId | None
    default: destack._generated.dir.tree.node.LocalNodeId | None
    is_const: bool
    kind: typing.Literal["type"] = "type"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_generic_parameter(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_generic_parameter(self)


@dataclass(frozen=True, slots=True)
class GenericParameterVariadicType(GenericParameterImpl):
    """Variadic type parameter."""

    name: destack._generated.core.string.StringId
    variance: destack._generated.dir.tree.property.VarianceModifier | None
    constraint: destack._generated.dir.tree.node.LocalNodeId | None
    default: destack._generated.dir.tree.node.LocalNodeId | None
    is_const: bool
    kind: typing.Literal["variadicType"] = "variadicType"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_generic_parameter(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_generic_parameter(self)


@dataclass(frozen=True, slots=True)
class GenericParameterValue(GenericParameterImpl):
    """Value parameter."""

    name: destack._generated.core.string.StringId
    declared_type: destack._generated.dir.tree.node.LocalNodeId | None
    default: destack._generated.dir.tree.node.LocalNodeId | None
    is_comptime: bool
    kind: typing.Literal["value"] = "value"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_generic_parameter(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_generic_parameter(self)


@dataclass(frozen=True, slots=True)
class GenericParameterVariadicValue(GenericParameterImpl):
    """Variadic value parameter."""

    name: destack._generated.core.string.StringId
    declared_type: destack._generated.dir.tree.node.LocalNodeId | None
    default: destack._generated.dir.tree.node.LocalNodeId | None
    is_comptime: bool
    kind: typing.Literal["variadicValue"] = "variadicValue"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_generic_parameter(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_generic_parameter(self)


@dataclass(frozen=True, slots=True)
class GenericParameterError(GenericParameterImpl):
    """Malformed generic parameter."""

    kind: typing.Literal["error"] = "error"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_generic_parameter(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_generic_parameter(self)


"""A declared generic parameter in source."""
GenericParameter: typing.TypeAlias = (
    GenericParameterType
    | GenericParameterVariadicType
    | GenericParameterValue
    | GenericParameterVariadicValue
    | GenericParameterError
)


def encode_generic_parameter(writer: BinaryWriter, value: GenericParameter) -> None:
    """Encode one GenericParameter."""
    if value.kind == "type":
        writer.write_unsigned(0)
        destack._generated.core.string.encode_string_id(writer, value.name)
        if value.variance is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.property.encode_variance_modifier(
                writer, value.variance
            )
        if value.constraint is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, value.constraint
            )
        if value.default is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_local_node_id(writer, value.default)
        writer.write_bool(value.is_const)
    elif value.kind == "variadicType":
        writer.write_unsigned(1)
        destack._generated.core.string.encode_string_id(writer, value.name)
        if value.variance is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.property.encode_variance_modifier(
                writer, value.variance
            )
        if value.constraint is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, value.constraint
            )
        if value.default is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_local_node_id(writer, value.default)
        writer.write_bool(value.is_const)
    elif value.kind == "value":
        writer.write_unsigned(2)
        destack._generated.core.string.encode_string_id(writer, value.name)
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
        writer.write_bool(value.is_comptime)
    elif value.kind == "variadicValue":
        writer.write_unsigned(3)
        destack._generated.core.string.encode_string_id(writer, value.name)
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
        writer.write_bool(value.is_comptime)
    elif value.kind == "error":
        writer.write_unsigned(4)
    else:
        raise SerdeError("unknown enum variant")


def decode_generic_parameter(reader: BinaryReader) -> GenericParameter:
    """Decode one GenericParameter."""
    variant = reader.read_number()

    if variant == 0:
        name = destack._generated.core.string.decode_string_id(reader)
        variance = reader.read_option(
            lambda: destack._generated.dir.tree.property.decode_variance_modifier(
                reader
            )
        )
        constraint = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )
        default = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )
        is_const = reader.read_bool()

        return GenericParameterType(
            name=name,
            variance=variance,
            constraint=constraint,
            default=default,
            is_const=is_const,
        )
    elif variant == 1:
        name = destack._generated.core.string.decode_string_id(reader)
        variance = reader.read_option(
            lambda: destack._generated.dir.tree.property.decode_variance_modifier(
                reader
            )
        )
        constraint = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )
        default = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )
        is_const = reader.read_bool()

        return GenericParameterVariadicType(
            name=name,
            variance=variance,
            constraint=constraint,
            default=default,
            is_const=is_const,
        )
    elif variant == 2:
        name = destack._generated.core.string.decode_string_id(reader)
        declared_type = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )
        default = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )
        is_comptime = reader.read_bool()

        return GenericParameterValue(
            name=name,
            declared_type=declared_type,
            default=default,
            is_comptime=is_comptime,
        )
    elif variant == 3:
        name = destack._generated.core.string.decode_string_id(reader)
        declared_type = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )
        default = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )
        is_comptime = reader.read_bool()

        return GenericParameterVariadicValue(
            name=name,
            declared_type=declared_type,
            default=default,
            is_comptime=is_comptime,
        )
    elif variant == 4:
        return GenericParameterError()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_generic_parameter(value: GenericParameter) -> Json:
    """Return one JSON value for one GenericParameter."""
    if value.kind == "type":
        return {
            "kind": "type",
            "name": destack._generated.core.string.to_json_string_id(value.name),
            **(
                {}
                if value.variance is None
                else {
                    "variance": destack._generated.dir.tree.property.to_json_variance_modifier(
                        value.variance
                    )
                }
            ),
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
                if value.default is None
                else {
                    "default": destack._generated.dir.tree.node.to_json_local_node_id(
                        value.default
                    )
                }
            ),
            "isConst": value.is_const,
        }
    elif value.kind == "variadicType":
        return {
            "kind": "variadicType",
            "name": destack._generated.core.string.to_json_string_id(value.name),
            **(
                {}
                if value.variance is None
                else {
                    "variance": destack._generated.dir.tree.property.to_json_variance_modifier(
                        value.variance
                    )
                }
            ),
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
                if value.default is None
                else {
                    "default": destack._generated.dir.tree.node.to_json_local_node_id(
                        value.default
                    )
                }
            ),
            "isConst": value.is_const,
        }
    elif value.kind == "value":
        return {
            "kind": "value",
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
                if value.default is None
                else {
                    "default": destack._generated.dir.tree.node.to_json_local_node_id(
                        value.default
                    )
                }
            ),
            "isComptime": value.is_comptime,
        }
    elif value.kind == "variadicValue":
        return {
            "kind": "variadicValue",
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
                if value.default is None
                else {
                    "default": destack._generated.dir.tree.node.to_json_local_node_id(
                        value.default
                    )
                }
            ),
            "isComptime": value.is_comptime,
        }
    elif value.kind == "error":
        return {
            "kind": "error",
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_generic_parameter(value: Json) -> GenericParameter:
    """Return one GenericParameter from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "type":
        return GenericParameterType(
            name=destack._generated.core.string.from_json_string_id(
                json_field(object_, "name")
            ),
            variance=json_optional(
                object_,
                "variance",
                lambda value: (
                    destack._generated.dir.tree.property.from_json_variance_modifier(
                        value
                    )
                ),
            ),
            constraint=json_optional(
                object_,
                "constraint",
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
            is_const=json_bool(json_field(object_, "isConst")),
        )
    elif kind == "variadicType":
        return GenericParameterVariadicType(
            name=destack._generated.core.string.from_json_string_id(
                json_field(object_, "name")
            ),
            variance=json_optional(
                object_,
                "variance",
                lambda value: (
                    destack._generated.dir.tree.property.from_json_variance_modifier(
                        value
                    )
                ),
            ),
            constraint=json_optional(
                object_,
                "constraint",
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
            is_const=json_bool(json_field(object_, "isConst")),
        )
    elif kind == "value":
        return GenericParameterValue(
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
            default=json_optional(
                object_,
                "default",
                lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                    value
                ),
            ),
            is_comptime=json_bool(json_field(object_, "isComptime")),
        )
    elif kind == "variadicValue":
        return GenericParameterVariadicValue(
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
            default=json_optional(
                object_,
                "default",
                lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                    value
                ),
            ),
            is_comptime=json_bool(json_field(object_, "isComptime")),
        )
    elif kind == "error":
        return GenericParameterError()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class ParameterNamed(ParameterImpl):
    """Named scalar parameter."""

    name: destack._generated.core.string.StringId
    declared_type: destack._generated.dir.tree.node.LocalNodeId | None
    default: destack._generated.dir.tree.node.LocalNodeId | None
    is_optional: bool
    is_comptime: bool
    kind: typing.Literal["named"] = "named"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_parameter(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_parameter(self)


@dataclass(frozen=True, slots=True)
class ParameterPattern(ParameterImpl):
    """Pattern parameter."""

    pattern: destack._generated.dir.tree.node.LocalNodeId
    declared_type: destack._generated.dir.tree.node.LocalNodeId | None
    default: destack._generated.dir.tree.node.LocalNodeId | None
    is_optional: bool
    is_comptime: bool
    kind: typing.Literal["pattern"] = "pattern"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_parameter(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_parameter(self)


@dataclass(frozen=True, slots=True)
class ParameterVariadicNamed(ParameterImpl):
    """Variadic named parameter."""

    name: destack._generated.core.string.StringId
    declared_type: destack._generated.dir.tree.node.LocalNodeId | None
    is_comptime: bool
    kind: typing.Literal["variadicNamed"] = "variadicNamed"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_parameter(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_parameter(self)


@dataclass(frozen=True, slots=True)
class ParameterVariadicPattern(ParameterImpl):
    """Variadic pattern parameter."""

    pattern: destack._generated.dir.tree.node.LocalNodeId
    declared_type: destack._generated.dir.tree.node.LocalNodeId | None
    is_comptime: bool
    kind: typing.Literal["variadicPattern"] = "variadicPattern"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_parameter(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_parameter(self)


@dataclass(frozen=True, slots=True)
class ParameterError(ParameterImpl):
    """Malformed parameter."""

    kind: typing.Literal["error"] = "error"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_parameter(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_parameter(self)


"""A parameter to a callable construct."""
Parameter: typing.TypeAlias = (
    ParameterNamed
    | ParameterPattern
    | ParameterVariadicNamed
    | ParameterVariadicPattern
    | ParameterError
)


def encode_parameter(writer: BinaryWriter, value: Parameter) -> None:
    """Encode one Parameter."""
    if value.kind == "named":
        writer.write_unsigned(0)
        destack._generated.core.string.encode_string_id(writer, value.name)
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
        writer.write_bool(value.is_optional)
        writer.write_bool(value.is_comptime)
    elif value.kind == "pattern":
        writer.write_unsigned(1)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.pattern)
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
        writer.write_bool(value.is_optional)
        writer.write_bool(value.is_comptime)
    elif value.kind == "variadicNamed":
        writer.write_unsigned(2)
        destack._generated.core.string.encode_string_id(writer, value.name)
        if value.declared_type is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, value.declared_type
            )
        writer.write_bool(value.is_comptime)
    elif value.kind == "variadicPattern":
        writer.write_unsigned(3)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.pattern)
        if value.declared_type is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, value.declared_type
            )
        writer.write_bool(value.is_comptime)
    elif value.kind == "error":
        writer.write_unsigned(4)
    else:
        raise SerdeError("unknown enum variant")


def decode_parameter(reader: BinaryReader) -> Parameter:
    """Decode one Parameter."""
    variant = reader.read_number()

    if variant == 0:
        name = destack._generated.core.string.decode_string_id(reader)
        declared_type = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )
        default = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )
        is_optional = reader.read_bool()
        is_comptime = reader.read_bool()

        return ParameterNamed(
            name=name,
            declared_type=declared_type,
            default=default,
            is_optional=is_optional,
            is_comptime=is_comptime,
        )
    elif variant == 1:
        pattern = destack._generated.dir.tree.node.decode_local_node_id(reader)
        declared_type = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )
        default = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )
        is_optional = reader.read_bool()
        is_comptime = reader.read_bool()

        return ParameterPattern(
            pattern=pattern,
            declared_type=declared_type,
            default=default,
            is_optional=is_optional,
            is_comptime=is_comptime,
        )
    elif variant == 2:
        name = destack._generated.core.string.decode_string_id(reader)
        declared_type = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )
        is_comptime = reader.read_bool()

        return ParameterVariadicNamed(
            name=name,
            declared_type=declared_type,
            is_comptime=is_comptime,
        )
    elif variant == 3:
        pattern = destack._generated.dir.tree.node.decode_local_node_id(reader)
        declared_type = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )
        is_comptime = reader.read_bool()

        return ParameterVariadicPattern(
            pattern=pattern,
            declared_type=declared_type,
            is_comptime=is_comptime,
        )
    elif variant == 4:
        return ParameterError()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_parameter(value: Parameter) -> Json:
    """Return one JSON value for one Parameter."""
    if value.kind == "named":
        return {
            "kind": "named",
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
                if value.default is None
                else {
                    "default": destack._generated.dir.tree.node.to_json_local_node_id(
                        value.default
                    )
                }
            ),
            "isOptional": value.is_optional,
            "isComptime": value.is_comptime,
        }
    elif value.kind == "pattern":
        return {
            "kind": "pattern",
            "pattern": destack._generated.dir.tree.node.to_json_local_node_id(
                value.pattern
            ),
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
            "isOptional": value.is_optional,
            "isComptime": value.is_comptime,
        }
    elif value.kind == "variadicNamed":
        return {
            "kind": "variadicNamed",
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
            "isComptime": value.is_comptime,
        }
    elif value.kind == "variadicPattern":
        return {
            "kind": "variadicPattern",
            "pattern": destack._generated.dir.tree.node.to_json_local_node_id(
                value.pattern
            ),
            **(
                {}
                if value.declared_type is None
                else {
                    "declaredType": destack._generated.dir.tree.node.to_json_local_node_id(
                        value.declared_type
                    )
                }
            ),
            "isComptime": value.is_comptime,
        }
    elif value.kind == "error":
        return {
            "kind": "error",
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_parameter(value: Json) -> Parameter:
    """Return one Parameter from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "named":
        return ParameterNamed(
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
            default=json_optional(
                object_,
                "default",
                lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                    value
                ),
            ),
            is_optional=json_bool(json_field(object_, "isOptional")),
            is_comptime=json_bool(json_field(object_, "isComptime")),
        )
    elif kind == "pattern":
        return ParameterPattern(
            pattern=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "pattern")
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
            is_optional=json_bool(json_field(object_, "isOptional")),
            is_comptime=json_bool(json_field(object_, "isComptime")),
        )
    elif kind == "variadicNamed":
        return ParameterVariadicNamed(
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
            is_comptime=json_bool(json_field(object_, "isComptime")),
        )
    elif kind == "variadicPattern":
        return ParameterVariadicPattern(
            pattern=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "pattern")
            ),
            declared_type=json_optional(
                object_,
                "declaredType",
                lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                    value
                ),
            ),
            is_comptime=json_bool(json_field(object_, "isComptime")),
        )
    elif kind == "error":
        return ParameterError()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class GenericArgumentType:
    """Type generic argument."""

    value: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["type"] = "type"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_generic_argument(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_generic_argument(self)


@dataclass(frozen=True, slots=True)
class GenericArgumentSpreadType:
    """Spread type generic argument."""

    value: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["spreadType"] = "spreadType"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_generic_argument(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_generic_argument(self)


@dataclass(frozen=True, slots=True)
class GenericArgumentValue:
    """Value generic argument."""

    value: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["value"] = "value"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_generic_argument(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_generic_argument(self)


@dataclass(frozen=True, slots=True)
class GenericArgumentSpreadValue:
    """Spread value generic argument."""

    value: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["spreadValue"] = "spreadValue"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_generic_argument(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_generic_argument(self)


@dataclass(frozen=True, slots=True)
class GenericArgumentAssociatedType:
    """Associated type refinement."""

    name: destack._generated.core.string.StringId
    value: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["associatedType"] = "associatedType"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_generic_argument(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_generic_argument(self)


@dataclass(frozen=True, slots=True)
class GenericArgumentAssociatedConst:
    """Associated compile-time constant refinement."""

    name: destack._generated.core.string.StringId
    value: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["associatedConst"] = "associatedConst"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_generic_argument(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_generic_argument(self)


@dataclass(frozen=True, slots=True)
class GenericArgumentError:
    """Malformed generic argument slot."""

    kind: typing.Literal["error"] = "error"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_generic_argument(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_generic_argument(self)


"""A generic argument in static argument position."""
GenericArgument: typing.TypeAlias = (
    GenericArgumentType
    | GenericArgumentSpreadType
    | GenericArgumentValue
    | GenericArgumentSpreadValue
    | GenericArgumentAssociatedType
    | GenericArgumentAssociatedConst
    | GenericArgumentError
)


def encode_generic_argument(writer: BinaryWriter, value: GenericArgument) -> None:
    """Encode one GenericArgument."""
    if value.kind == "type":
        writer.write_unsigned(0)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.value)
    elif value.kind == "spreadType":
        writer.write_unsigned(1)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.value)
    elif value.kind == "value":
        writer.write_unsigned(2)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.value)
    elif value.kind == "spreadValue":
        writer.write_unsigned(3)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.value)
    elif value.kind == "associatedType":
        writer.write_unsigned(4)
        destack._generated.core.string.encode_string_id(writer, value.name)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.value)
    elif value.kind == "associatedConst":
        writer.write_unsigned(5)
        destack._generated.core.string.encode_string_id(writer, value.name)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.value)
    elif value.kind == "error":
        writer.write_unsigned(6)
    else:
        raise SerdeError("unknown enum variant")


def decode_generic_argument(reader: BinaryReader) -> GenericArgument:
    """Decode one GenericArgument."""
    variant = reader.read_number()

    if variant == 0:
        value_ = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return GenericArgumentType(
            value=value_,
        )
    elif variant == 1:
        value_ = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return GenericArgumentSpreadType(
            value=value_,
        )
    elif variant == 2:
        value_ = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return GenericArgumentValue(
            value=value_,
        )
    elif variant == 3:
        value_ = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return GenericArgumentSpreadValue(
            value=value_,
        )
    elif variant == 4:
        name = destack._generated.core.string.decode_string_id(reader)
        value_ = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return GenericArgumentAssociatedType(
            name=name,
            value=value_,
        )
    elif variant == 5:
        name = destack._generated.core.string.decode_string_id(reader)
        value_ = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return GenericArgumentAssociatedConst(
            name=name,
            value=value_,
        )
    elif variant == 6:
        return GenericArgumentError()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_generic_argument(value: GenericArgument) -> Json:
    """Return one JSON value for one GenericArgument."""
    if value.kind == "type":
        return {
            "kind": "type",
            "value": destack._generated.dir.tree.node.to_json_local_node_id(
                value.value
            ),
        }
    elif value.kind == "spreadType":
        return {
            "kind": "spreadType",
            "value": destack._generated.dir.tree.node.to_json_local_node_id(
                value.value
            ),
        }
    elif value.kind == "value":
        return {
            "kind": "value",
            "value": destack._generated.dir.tree.node.to_json_local_node_id(
                value.value
            ),
        }
    elif value.kind == "spreadValue":
        return {
            "kind": "spreadValue",
            "value": destack._generated.dir.tree.node.to_json_local_node_id(
                value.value
            ),
        }
    elif value.kind == "associatedType":
        return {
            "kind": "associatedType",
            "name": destack._generated.core.string.to_json_string_id(value.name),
            "value": destack._generated.dir.tree.node.to_json_local_node_id(
                value.value
            ),
        }
    elif value.kind == "associatedConst":
        return {
            "kind": "associatedConst",
            "name": destack._generated.core.string.to_json_string_id(value.name),
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


def from_json_generic_argument(value: Json) -> GenericArgument:
    """Return one GenericArgument from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "type":
        return GenericArgumentType(
            value=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "value")
            ),
        )
    elif kind == "spreadType":
        return GenericArgumentSpreadType(
            value=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "value")
            ),
        )
    elif kind == "value":
        return GenericArgumentValue(
            value=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "value")
            ),
        )
    elif kind == "spreadValue":
        return GenericArgumentSpreadValue(
            value=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "value")
            ),
        )
    elif kind == "associatedType":
        return GenericArgumentAssociatedType(
            name=destack._generated.core.string.from_json_string_id(
                json_field(object_, "name")
            ),
            value=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "value")
            ),
        )
    elif kind == "associatedConst":
        return GenericArgumentAssociatedConst(
            name=destack._generated.core.string.from_json_string_id(
                json_field(object_, "name")
            ),
            value=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "value")
            ),
        )
    elif kind == "error":
        return GenericArgumentError()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class TupleElementElement:
    """One non-spread tuple element."""

    label: destack._generated.core.string.StringId | None
    value: destack._generated.dir.tree.node.LocalNodeId
    is_optional: bool
    is_readonly: bool
    kind: typing.Literal["element"] = "element"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tuple_element(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tuple_element(self)


@dataclass(frozen=True, slots=True)
class TupleElementSpread:
    """One spread tuple element."""

    label: destack._generated.core.string.StringId | None
    value: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["spread"] = "spread"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tuple_element(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tuple_element(self)


@dataclass(frozen=True, slots=True)
class TupleElementError:
    """Malformed tuple element slot."""

    kind: typing.Literal["error"] = "error"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tuple_element(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tuple_element(self)


"""One tuple type element."""
TupleElement: typing.TypeAlias = (
    TupleElementElement | TupleElementSpread | TupleElementError
)


def encode_tuple_element(writer: BinaryWriter, value: TupleElement) -> None:
    """Encode one TupleElement."""
    if value.kind == "element":
        writer.write_unsigned(0)
        if value.label is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.core.string.encode_string_id(writer, value.label)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.value)
        writer.write_bool(value.is_optional)
        writer.write_bool(value.is_readonly)
    elif value.kind == "spread":
        writer.write_unsigned(1)
        if value.label is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.core.string.encode_string_id(writer, value.label)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.value)
    elif value.kind == "error":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_tuple_element(reader: BinaryReader) -> TupleElement:
    """Decode one TupleElement."""
    variant = reader.read_number()

    if variant == 0:
        label = reader.read_option(
            lambda: destack._generated.core.string.decode_string_id(reader)
        )
        value_ = destack._generated.dir.tree.node.decode_local_node_id(reader)
        is_optional = reader.read_bool()
        is_readonly = reader.read_bool()

        return TupleElementElement(
            label=label,
            value=value_,
            is_optional=is_optional,
            is_readonly=is_readonly,
        )
    elif variant == 1:
        label = reader.read_option(
            lambda: destack._generated.core.string.decode_string_id(reader)
        )
        value_ = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return TupleElementSpread(
            label=label,
            value=value_,
        )
    elif variant == 2:
        return TupleElementError()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_tuple_element(value: TupleElement) -> Json:
    """Return one JSON value for one TupleElement."""
    if value.kind == "element":
        return {
            "kind": "element",
            **(
                {}
                if value.label is None
                else {
                    "label": destack._generated.core.string.to_json_string_id(
                        value.label
                    )
                }
            ),
            "value": destack._generated.dir.tree.node.to_json_local_node_id(
                value.value
            ),
            "isOptional": value.is_optional,
            "isReadonly": value.is_readonly,
        }
    elif value.kind == "spread":
        return {
            "kind": "spread",
            **(
                {}
                if value.label is None
                else {
                    "label": destack._generated.core.string.to_json_string_id(
                        value.label
                    )
                }
            ),
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


def from_json_tuple_element(value: Json) -> TupleElement:
    """Return one TupleElement from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "element":
        return TupleElementElement(
            label=json_optional(
                object_,
                "label",
                lambda value: destack._generated.core.string.from_json_string_id(value),
            ),
            value=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "value")
            ),
            is_optional=json_bool(json_field(object_, "isOptional")),
            is_readonly=json_bool(json_field(object_, "isReadonly")),
        )
    elif kind == "spread":
        return TupleElementSpread(
            label=json_optional(
                object_,
                "label",
                lambda value: destack._generated.core.string.from_json_string_id(value),
            ),
            value=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "value")
            ),
        )
    elif kind == "error":
        return TupleElementError()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class ArgumentNamed:
    """Named argument."""

    name: destack._generated.dir.tree.key.Name
    value: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["named"] = "named"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_argument(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_argument(self)


@dataclass(frozen=True, slots=True)
class ArgumentLabeled:
    """Labeled argument."""

    label: destack._generated.core.string.StringId
    value: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["labeled"] = "labeled"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_argument(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_argument(self)


@dataclass(frozen=True, slots=True)
class ArgumentPositional:
    """Positional argument."""

    value: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["positional"] = "positional"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_argument(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_argument(self)


@dataclass(frozen=True, slots=True)
class ArgumentSpread:
    """Spread argument."""

    label: destack._generated.core.string.StringId | None
    value: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["spread"] = "spread"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_argument(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_argument(self)


@dataclass(frozen=True, slots=True)
class ArgumentError:
    """Malformed argument slot."""

    kind: typing.Literal["error"] = "error"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_argument(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_argument(self)


"""An argument to a runtime call or tree construct."""
Argument: typing.TypeAlias = (
    ArgumentNamed
    | ArgumentLabeled
    | ArgumentPositional
    | ArgumentSpread
    | ArgumentError
)


def encode_argument(writer: BinaryWriter, value: Argument) -> None:
    """Encode one Argument."""
    if value.kind == "named":
        writer.write_unsigned(0)
        destack._generated.dir.tree.key.encode_name(writer, value.name)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.value)
    elif value.kind == "labeled":
        writer.write_unsigned(1)
        destack._generated.core.string.encode_string_id(writer, value.label)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.value)
    elif value.kind == "positional":
        writer.write_unsigned(2)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.value)
    elif value.kind == "spread":
        writer.write_unsigned(3)
        if value.label is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.core.string.encode_string_id(writer, value.label)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.value)
    elif value.kind == "error":
        writer.write_unsigned(4)
    else:
        raise SerdeError("unknown enum variant")


def decode_argument(reader: BinaryReader) -> Argument:
    """Decode one Argument."""
    variant = reader.read_number()

    if variant == 0:
        name = destack._generated.dir.tree.key.decode_name(reader)
        value_ = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return ArgumentNamed(
            name=name,
            value=value_,
        )
    elif variant == 1:
        label = destack._generated.core.string.decode_string_id(reader)
        value_ = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return ArgumentLabeled(
            label=label,
            value=value_,
        )
    elif variant == 2:
        value_ = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return ArgumentPositional(
            value=value_,
        )
    elif variant == 3:
        label = reader.read_option(
            lambda: destack._generated.core.string.decode_string_id(reader)
        )
        value_ = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return ArgumentSpread(
            label=label,
            value=value_,
        )
    elif variant == 4:
        return ArgumentError()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_argument(value: Argument) -> Json:
    """Return one JSON value for one Argument."""
    if value.kind == "named":
        return {
            "kind": "named",
            "name": destack._generated.dir.tree.key.to_json_name(value.name),
            "value": destack._generated.dir.tree.node.to_json_local_node_id(
                value.value
            ),
        }
    elif value.kind == "labeled":
        return {
            "kind": "labeled",
            "label": destack._generated.core.string.to_json_string_id(value.label),
            "value": destack._generated.dir.tree.node.to_json_local_node_id(
                value.value
            ),
        }
    elif value.kind == "positional":
        return {
            "kind": "positional",
            "value": destack._generated.dir.tree.node.to_json_local_node_id(
                value.value
            ),
        }
    elif value.kind == "spread":
        return {
            "kind": "spread",
            **(
                {}
                if value.label is None
                else {
                    "label": destack._generated.core.string.to_json_string_id(
                        value.label
                    )
                }
            ),
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


def from_json_argument(value: Json) -> Argument:
    """Return one Argument from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "named":
        return ArgumentNamed(
            name=destack._generated.dir.tree.key.from_json_name(
                json_field(object_, "name")
            ),
            value=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "value")
            ),
        )
    elif kind == "labeled":
        return ArgumentLabeled(
            label=destack._generated.core.string.from_json_string_id(
                json_field(object_, "label")
            ),
            value=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "value")
            ),
        )
    elif kind == "positional":
        return ArgumentPositional(
            value=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "value")
            ),
        )
    elif kind == "spread":
        return ArgumentSpread(
            label=json_optional(
                object_,
                "label",
                lambda value: destack._generated.core.string.from_json_string_id(value),
            ),
            value=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "value")
            ),
        )
    elif kind == "error":
        return ArgumentError()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


__all__ = [
    "GenericParameter",
    "encode_generic_parameter",
    "decode_generic_parameter",
    "to_json_generic_parameter",
    "from_json_generic_parameter",
    "GenericParameterType",
    "GenericParameterVariadicType",
    "GenericParameterValue",
    "GenericParameterVariadicValue",
    "GenericParameterError",
    "Parameter",
    "encode_parameter",
    "decode_parameter",
    "to_json_parameter",
    "from_json_parameter",
    "ParameterNamed",
    "ParameterPattern",
    "ParameterVariadicNamed",
    "ParameterVariadicPattern",
    "ParameterError",
    "GenericArgument",
    "encode_generic_argument",
    "decode_generic_argument",
    "to_json_generic_argument",
    "from_json_generic_argument",
    "GenericArgumentType",
    "GenericArgumentSpreadType",
    "GenericArgumentValue",
    "GenericArgumentSpreadValue",
    "GenericArgumentAssociatedType",
    "GenericArgumentAssociatedConst",
    "GenericArgumentError",
    "TupleElement",
    "encode_tuple_element",
    "decode_tuple_element",
    "to_json_tuple_element",
    "from_json_tuple_element",
    "TupleElementElement",
    "TupleElementSpread",
    "TupleElementError",
    "Argument",
    "encode_argument",
    "decode_argument",
    "to_json_argument",
    "from_json_argument",
    "ArgumentNamed",
    "ArgumentLabeled",
    "ArgumentPositional",
    "ArgumentSpread",
    "ArgumentError",
]
