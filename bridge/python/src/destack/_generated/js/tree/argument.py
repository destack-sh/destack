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
    json_object,
    json_optional,
    json_string,
)

import destack._generated.core.string
import destack._generated.js.tree.node


@dataclass(frozen=True, slots=True)
class BindingModifier:
    """The modifiers of a field-like item."""

    # the kind of the binding
    kind: BindingKind | None
    # the variance of a type parameter
    variance: VarianceModifier | None
    # the scope of the binding
    anchor: BindingAnchor | None
    # the mutability of the field
    mutability: destack._generated.js.tree.node.Mutability | None
    # the visibility of the field
    visibility: destack._generated.js.tree.node.Visibility | None
    # the operator to apply to the binding
    operator: BindingOperator | None
    # whether the binding uses a definite assignment assertion
    definite: bool
    # the accessor kind of the binding
    accessor: AccessorKind | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_binding_modifier(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> BindingModifier:
        """Decode one BindingModifier."""
        return decode_binding_modifier(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_binding_modifier(self)

    @classmethod
    def from_json(cls, value: Json) -> BindingModifier:
        """Return one BindingModifier from one JSON value."""
        return from_json_binding_modifier(value)


def encode_binding_modifier(writer: BinaryWriter, value: BindingModifier) -> None:
    """Encode one BindingModifier."""
    if value.kind is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_binding_kind(writer, value.kind)
    if value.variance is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_variance_modifier(writer, value.variance)
    if value.anchor is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_binding_anchor(writer, value.anchor)
    if value.mutability is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.js.tree.node.encode_mutability(writer, value.mutability)
    if value.visibility is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.js.tree.node.encode_visibility(writer, value.visibility)
    if value.operator is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_binding_operator(writer, value.operator)
    writer.write_bool(value.definite)
    if value.accessor is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_accessor_kind(writer, value.accessor)


def decode_binding_modifier(reader: BinaryReader) -> BindingModifier:
    """Decode one BindingModifier."""
    kind = reader.read_option(lambda: decode_binding_kind(reader))
    variance = reader.read_option(lambda: decode_variance_modifier(reader))
    anchor = reader.read_option(lambda: decode_binding_anchor(reader))
    mutability = reader.read_option(
        lambda: destack._generated.js.tree.node.decode_mutability(reader)
    )
    visibility = reader.read_option(
        lambda: destack._generated.js.tree.node.decode_visibility(reader)
    )
    operator = reader.read_option(lambda: decode_binding_operator(reader))
    definite = reader.read_bool()
    accessor = reader.read_option(lambda: decode_accessor_kind(reader))

    return BindingModifier(
        kind=kind,
        variance=variance,
        anchor=anchor,
        mutability=mutability,
        visibility=visibility,
        operator=operator,
        definite=definite,
        accessor=accessor,
    )


def to_json_binding_modifier(value: BindingModifier) -> Json:
    """Return one JSON value for one BindingModifier."""
    return {
        **({} if value.kind is None else {"kind": to_json_binding_kind(value.kind)}),
        **(
            {}
            if value.variance is None
            else {"variance": to_json_variance_modifier(value.variance)}
        ),
        **(
            {}
            if value.anchor is None
            else {"anchor": to_json_binding_anchor(value.anchor)}
        ),
        **(
            {}
            if value.mutability is None
            else {
                "mutability": destack._generated.js.tree.node.to_json_mutability(
                    value.mutability
                )
            }
        ),
        **(
            {}
            if value.visibility is None
            else {
                "visibility": destack._generated.js.tree.node.to_json_visibility(
                    value.visibility
                )
            }
        ),
        **(
            {}
            if value.operator is None
            else {"operator": to_json_binding_operator(value.operator)}
        ),
        "definite": value.definite,
        **(
            {}
            if value.accessor is None
            else {"accessor": to_json_accessor_kind(value.accessor)}
        ),
    }


def from_json_binding_modifier(value: Json) -> BindingModifier:
    """Return one BindingModifier from one JSON value."""
    object_ = json_object(value)

    return BindingModifier(
        kind=json_optional(
            object_, "kind", lambda value: from_json_binding_kind(value)
        ),
        variance=json_optional(
            object_, "variance", lambda value: from_json_variance_modifier(value)
        ),
        anchor=json_optional(
            object_, "anchor", lambda value: from_json_binding_anchor(value)
        ),
        mutability=json_optional(
            object_,
            "mutability",
            lambda value: destack._generated.js.tree.node.from_json_mutability(value),
        ),
        visibility=json_optional(
            object_,
            "visibility",
            lambda value: destack._generated.js.tree.node.from_json_visibility(value),
        ),
        operator=json_optional(
            object_, "operator", lambda value: from_json_binding_operator(value)
        ),
        definite=json_bool(json_field(object_, "definite")),
        accessor=json_optional(
            object_, "accessor", lambda value: from_json_accessor_kind(value)
        ),
    )


"""The type of a binding."""
BindingKind: typing.TypeAlias = typing.Literal["must"] | typing.Literal["maybe"]


def encode_binding_kind(writer: BinaryWriter, value: BindingKind) -> None:
    """Encode one BindingKind."""
    if value == "must":
        writer.write_unsigned(0)
    elif value == "maybe":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_binding_kind(reader: BinaryReader) -> BindingKind:
    """Decode one BindingKind."""
    variant = reader.read_number()

    if variant == 0:
        return "must"
    elif variant == 1:
        return "maybe"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_binding_kind(value: BindingKind) -> Json:
    """Return one JSON value for one BindingKind."""
    return value


def from_json_binding_kind(value: Json) -> BindingKind:
    """Return one BindingKind from one JSON value."""
    variant = json_string(value)

    if variant == "must":
        return "must"
    elif variant == "maybe":
        return "maybe"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Variance annotation for type parameters."""
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


"""The scope of a binding (dynamic or static)."""
BindingAnchor: typing.TypeAlias = typing.Literal["instance"] | typing.Literal["static"]


def encode_binding_anchor(writer: BinaryWriter, value: BindingAnchor) -> None:
    """Encode one BindingAnchor."""
    if value == "instance":
        writer.write_unsigned(0)
    elif value == "static":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_binding_anchor(reader: BinaryReader) -> BindingAnchor:
    """Decode one BindingAnchor."""
    variant = reader.read_number()

    if variant == 0:
        return "instance"
    elif variant == 1:
        return "static"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_binding_anchor(value: BindingAnchor) -> Json:
    """Return one JSON value for one BindingAnchor."""
    return value


def from_json_binding_anchor(value: Json) -> BindingAnchor:
    """Return one BindingAnchor from one JSON value."""
    variant = json_string(value)

    if variant == "instance":
        return "instance"
    elif variant == "static":
        return "static"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""The operator to apply to the binding."""
BindingOperator: typing.TypeAlias = typing.Literal["asConst"]


def encode_binding_operator(writer: BinaryWriter, value: BindingOperator) -> None:
    """Encode one BindingOperator."""
    if value == "asConst":
        writer.write_unsigned(0)
    else:
        raise SerdeError("unknown enum variant")


def decode_binding_operator(reader: BinaryReader) -> BindingOperator:
    """Decode one BindingOperator."""
    variant = reader.read_number()

    if variant == 0:
        return "asConst"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_binding_operator(value: BindingOperator) -> Json:
    """Return one JSON value for one BindingOperator."""
    return value


def from_json_binding_operator(value: Json) -> BindingOperator:
    """Return one BindingOperator from one JSON value."""
    variant = json_string(value)

    if variant == "asConst":
        return "asConst"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""The accessor kind of a binding."""
AccessorKind: typing.TypeAlias = typing.Literal["accessor"]


def encode_accessor_kind(writer: BinaryWriter, value: AccessorKind) -> None:
    """Encode one AccessorKind."""
    if value == "accessor":
        writer.write_unsigned(0)
    else:
        raise SerdeError("unknown enum variant")


def decode_accessor_kind(reader: BinaryReader) -> AccessorKind:
    """Decode one AccessorKind."""
    variant = reader.read_number()

    if variant == 0:
        return "accessor"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_accessor_kind(value: AccessorKind) -> Json:
    """Return one JSON value for one AccessorKind."""
    return value


def from_json_accessor_kind(value: Json) -> AccessorKind:
    """Return one AccessorKind from one JSON value."""
    variant = json_string(value)

    if variant == "accessor":
        return "accessor"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class GenericParameterType:
    """Type parameter."""

    modifiers: BindingModifier | None
    name: destack._generated.core.string.StringId
    constraint: destack._generated.js.tree.node.LocalNodeId | None
    default: destack._generated.js.tree.node.LocalNodeId | None
    kind: typing.Literal["type"] = "type"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_generic_parameter(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_generic_parameter(self)


"""One generic parameter."""
GenericParameter: typing.TypeAlias = GenericParameterType


def encode_generic_parameter(writer: BinaryWriter, value: GenericParameter) -> None:
    """Encode one GenericParameter."""
    if value.kind == "type":
        writer.write_unsigned(0)
        if value.modifiers is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            encode_binding_modifier(writer, value.modifiers)
        destack._generated.core.string.encode_string_id(writer, value.name)
        if value.constraint is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.js.tree.node.encode_local_node_id(
                writer, value.constraint
            )
        if value.default is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.js.tree.node.encode_local_node_id(writer, value.default)
    else:
        raise SerdeError("unknown enum variant")


def decode_generic_parameter(reader: BinaryReader) -> GenericParameter:
    """Decode one GenericParameter."""
    variant = reader.read_number()

    if variant == 0:
        modifiers = reader.read_option(lambda: decode_binding_modifier(reader))
        name = destack._generated.core.string.decode_string_id(reader)
        constraint = reader.read_option(
            lambda: destack._generated.js.tree.node.decode_local_node_id(reader)
        )
        default = reader.read_option(
            lambda: destack._generated.js.tree.node.decode_local_node_id(reader)
        )

        return GenericParameterType(
            modifiers=modifiers,
            name=name,
            constraint=constraint,
            default=default,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_generic_parameter(value: GenericParameter) -> Json:
    """Return one JSON value for one GenericParameter."""
    if value.kind == "type":
        return {
            "kind": "type",
            **(
                {}
                if value.modifiers is None
                else {"modifiers": to_json_binding_modifier(value.modifiers)}
            ),
            "name": destack._generated.core.string.to_json_string_id(value.name),
            **(
                {}
                if value.constraint is None
                else {
                    "constraint": destack._generated.js.tree.node.to_json_local_node_id(
                        value.constraint
                    )
                }
            ),
            **(
                {}
                if value.default is None
                else {
                    "default": destack._generated.js.tree.node.to_json_local_node_id(
                        value.default
                    )
                }
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_generic_parameter(value: Json) -> GenericParameter:
    """Return one GenericParameter from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "type":
        return GenericParameterType(
            modifiers=json_optional(
                object_, "modifiers", lambda value: from_json_binding_modifier(value)
            ),
            name=destack._generated.core.string.from_json_string_id(
                json_field(object_, "name")
            ),
            constraint=json_optional(
                object_,
                "constraint",
                lambda value: destack._generated.js.tree.node.from_json_local_node_id(
                    value
                ),
            ),
            default=json_optional(
                object_,
                "default",
                lambda value: destack._generated.js.tree.node.from_json_local_node_id(
                    value
                ),
            ),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class ParameterNamed:
    """Named parameter (like `x: int32` or `Validate: boolean = true`)."""

    modifiers: BindingModifier | None
    name: destack._generated.core.string.StringId
    ty: destack._generated.js.tree.node.LocalNodeId | None
    default: destack._generated.js.tree.node.LocalNodeId | None
    kind: typing.Literal["named"] = "named"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_parameter(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_parameter(self)


@dataclass(frozen=True, slots=True)
class ParameterPattern:
    """Pattern parameter (like `_` or `{ x }` or `{ x, ..rest }: MyType = Foo`)."""

    modifiers: BindingModifier | None
    pattern: destack._generated.js.tree.node.LocalNodeId
    ty: destack._generated.js.tree.node.LocalNodeId | None
    default: destack._generated.js.tree.node.LocalNodeId | None
    kind: typing.Literal["pattern"] = "pattern"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_parameter(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_parameter(self)


@dataclass(frozen=True, slots=True)
class ParameterVariadicNamed:
    """Variadic parameter with a named binding (like `...args: int32[]`)."""

    modifiers: BindingModifier | None
    name: destack._generated.core.string.StringId
    ty: destack._generated.js.tree.node.LocalNodeId | None
    kind: typing.Literal["variadicNamed"] = "variadicNamed"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_parameter(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_parameter(self)


@dataclass(frozen=True, slots=True)
class ParameterVariadicPattern:
    """Variadic parameter with a pattern binding (like `...[a, b]`)."""

    modifiers: BindingModifier | None
    pattern: destack._generated.js.tree.node.LocalNodeId
    ty: destack._generated.js.tree.node.LocalNodeId | None
    kind: typing.Literal["variadicPattern"] = "variadicPattern"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_parameter(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_parameter(self)


"""Named or positional parameter to some construct."""
Parameter: typing.TypeAlias = (
    ParameterNamed
    | ParameterPattern
    | ParameterVariadicNamed
    | ParameterVariadicPattern
)


def encode_parameter(writer: BinaryWriter, value: Parameter) -> None:
    """Encode one Parameter."""
    if value.kind == "named":
        writer.write_unsigned(0)
        if value.modifiers is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            encode_binding_modifier(writer, value.modifiers)
        destack._generated.core.string.encode_string_id(writer, value.name)
        if value.ty is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.js.tree.node.encode_local_node_id(writer, value.ty)
        if value.default is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.js.tree.node.encode_local_node_id(writer, value.default)
    elif value.kind == "pattern":
        writer.write_unsigned(1)
        if value.modifiers is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            encode_binding_modifier(writer, value.modifiers)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.pattern)
        if value.ty is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.js.tree.node.encode_local_node_id(writer, value.ty)
        if value.default is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.js.tree.node.encode_local_node_id(writer, value.default)
    elif value.kind == "variadicNamed":
        writer.write_unsigned(2)
        if value.modifiers is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            encode_binding_modifier(writer, value.modifiers)
        destack._generated.core.string.encode_string_id(writer, value.name)
        if value.ty is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.js.tree.node.encode_local_node_id(writer, value.ty)
    elif value.kind == "variadicPattern":
        writer.write_unsigned(3)
        if value.modifiers is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            encode_binding_modifier(writer, value.modifiers)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.pattern)
        if value.ty is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.js.tree.node.encode_local_node_id(writer, value.ty)
    else:
        raise SerdeError("unknown enum variant")


def decode_parameter(reader: BinaryReader) -> Parameter:
    """Decode one Parameter."""
    variant = reader.read_number()

    if variant == 0:
        modifiers = reader.read_option(lambda: decode_binding_modifier(reader))
        name = destack._generated.core.string.decode_string_id(reader)
        ty = reader.read_option(
            lambda: destack._generated.js.tree.node.decode_local_node_id(reader)
        )
        default = reader.read_option(
            lambda: destack._generated.js.tree.node.decode_local_node_id(reader)
        )

        return ParameterNamed(
            modifiers=modifiers,
            name=name,
            ty=ty,
            default=default,
        )
    elif variant == 1:
        modifiers = reader.read_option(lambda: decode_binding_modifier(reader))
        pattern = destack._generated.js.tree.node.decode_local_node_id(reader)
        ty = reader.read_option(
            lambda: destack._generated.js.tree.node.decode_local_node_id(reader)
        )
        default = reader.read_option(
            lambda: destack._generated.js.tree.node.decode_local_node_id(reader)
        )

        return ParameterPattern(
            modifiers=modifiers,
            pattern=pattern,
            ty=ty,
            default=default,
        )
    elif variant == 2:
        modifiers = reader.read_option(lambda: decode_binding_modifier(reader))
        name = destack._generated.core.string.decode_string_id(reader)
        ty = reader.read_option(
            lambda: destack._generated.js.tree.node.decode_local_node_id(reader)
        )

        return ParameterVariadicNamed(
            modifiers=modifiers,
            name=name,
            ty=ty,
        )
    elif variant == 3:
        modifiers = reader.read_option(lambda: decode_binding_modifier(reader))
        pattern = destack._generated.js.tree.node.decode_local_node_id(reader)
        ty = reader.read_option(
            lambda: destack._generated.js.tree.node.decode_local_node_id(reader)
        )

        return ParameterVariadicPattern(
            modifiers=modifiers,
            pattern=pattern,
            ty=ty,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_parameter(value: Parameter) -> Json:
    """Return one JSON value for one Parameter."""
    if value.kind == "named":
        return {
            "kind": "named",
            **(
                {}
                if value.modifiers is None
                else {"modifiers": to_json_binding_modifier(value.modifiers)}
            ),
            "name": destack._generated.core.string.to_json_string_id(value.name),
            **(
                {}
                if value.ty is None
                else {
                    "ty": destack._generated.js.tree.node.to_json_local_node_id(
                        value.ty
                    )
                }
            ),
            **(
                {}
                if value.default is None
                else {
                    "default": destack._generated.js.tree.node.to_json_local_node_id(
                        value.default
                    )
                }
            ),
        }
    elif value.kind == "pattern":
        return {
            "kind": "pattern",
            **(
                {}
                if value.modifiers is None
                else {"modifiers": to_json_binding_modifier(value.modifiers)}
            ),
            "pattern": destack._generated.js.tree.node.to_json_local_node_id(
                value.pattern
            ),
            **(
                {}
                if value.ty is None
                else {
                    "ty": destack._generated.js.tree.node.to_json_local_node_id(
                        value.ty
                    )
                }
            ),
            **(
                {}
                if value.default is None
                else {
                    "default": destack._generated.js.tree.node.to_json_local_node_id(
                        value.default
                    )
                }
            ),
        }
    elif value.kind == "variadicNamed":
        return {
            "kind": "variadicNamed",
            **(
                {}
                if value.modifiers is None
                else {"modifiers": to_json_binding_modifier(value.modifiers)}
            ),
            "name": destack._generated.core.string.to_json_string_id(value.name),
            **(
                {}
                if value.ty is None
                else {
                    "ty": destack._generated.js.tree.node.to_json_local_node_id(
                        value.ty
                    )
                }
            ),
        }
    elif value.kind == "variadicPattern":
        return {
            "kind": "variadicPattern",
            **(
                {}
                if value.modifiers is None
                else {"modifiers": to_json_binding_modifier(value.modifiers)}
            ),
            "pattern": destack._generated.js.tree.node.to_json_local_node_id(
                value.pattern
            ),
            **(
                {}
                if value.ty is None
                else {
                    "ty": destack._generated.js.tree.node.to_json_local_node_id(
                        value.ty
                    )
                }
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_parameter(value: Json) -> Parameter:
    """Return one Parameter from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "named":
        return ParameterNamed(
            modifiers=json_optional(
                object_, "modifiers", lambda value: from_json_binding_modifier(value)
            ),
            name=destack._generated.core.string.from_json_string_id(
                json_field(object_, "name")
            ),
            ty=json_optional(
                object_,
                "ty",
                lambda value: destack._generated.js.tree.node.from_json_local_node_id(
                    value
                ),
            ),
            default=json_optional(
                object_,
                "default",
                lambda value: destack._generated.js.tree.node.from_json_local_node_id(
                    value
                ),
            ),
        )
    elif kind == "pattern":
        return ParameterPattern(
            modifiers=json_optional(
                object_, "modifiers", lambda value: from_json_binding_modifier(value)
            ),
            pattern=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "pattern")
            ),
            ty=json_optional(
                object_,
                "ty",
                lambda value: destack._generated.js.tree.node.from_json_local_node_id(
                    value
                ),
            ),
            default=json_optional(
                object_,
                "default",
                lambda value: destack._generated.js.tree.node.from_json_local_node_id(
                    value
                ),
            ),
        )
    elif kind == "variadicNamed":
        return ParameterVariadicNamed(
            modifiers=json_optional(
                object_, "modifiers", lambda value: from_json_binding_modifier(value)
            ),
            name=destack._generated.core.string.from_json_string_id(
                json_field(object_, "name")
            ),
            ty=json_optional(
                object_,
                "ty",
                lambda value: destack._generated.js.tree.node.from_json_local_node_id(
                    value
                ),
            ),
        )
    elif kind == "variadicPattern":
        return ParameterVariadicPattern(
            modifiers=json_optional(
                object_, "modifiers", lambda value: from_json_binding_modifier(value)
            ),
            pattern=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "pattern")
            ),
            ty=json_optional(
                object_,
                "ty",
                lambda value: destack._generated.js.tree.node.from_json_local_node_id(
                    value
                ),
            ),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class ArgumentPositional:
    """Positional argument (like `1` or `foo()`)."""

    value: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["positional"] = "positional"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_argument(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_argument(self)


@dataclass(frozen=True, slots=True)
class ArgumentSpread:
    """Spread argument (like `...args`)."""

    value: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["spread"] = "spread"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_argument(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_argument(self)


"""Positional argument to some construct."""
Argument: typing.TypeAlias = ArgumentPositional | ArgumentSpread


def encode_argument(writer: BinaryWriter, value: Argument) -> None:
    """Encode one Argument."""
    if value.kind == "positional":
        writer.write_unsigned(0)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.value)
    elif value.kind == "spread":
        writer.write_unsigned(1)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.value)
    else:
        raise SerdeError("unknown enum variant")


def decode_argument(reader: BinaryReader) -> Argument:
    """Decode one Argument."""
    variant = reader.read_number()

    if variant == 0:
        value_ = destack._generated.js.tree.node.decode_local_node_id(reader)

        return ArgumentPositional(
            value=value_,
        )
    elif variant == 1:
        value_ = destack._generated.js.tree.node.decode_local_node_id(reader)

        return ArgumentSpread(
            value=value_,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_argument(value: Argument) -> Json:
    """Return one JSON value for one Argument."""
    if value.kind == "positional":
        return {
            "kind": "positional",
            "value": destack._generated.js.tree.node.to_json_local_node_id(value.value),
        }
    elif value.kind == "spread":
        return {
            "kind": "spread",
            "value": destack._generated.js.tree.node.to_json_local_node_id(value.value),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_argument(value: Json) -> Argument:
    """Return one Argument from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "positional":
        return ArgumentPositional(
            value=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "value")
            ),
        )
    elif kind == "spread":
        return ArgumentSpread(
            value=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "value")
            ),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


__all__ = [
    "BindingModifier",
    "encode_binding_modifier",
    "decode_binding_modifier",
    "to_json_binding_modifier",
    "from_json_binding_modifier",
    "BindingKind",
    "encode_binding_kind",
    "decode_binding_kind",
    "to_json_binding_kind",
    "from_json_binding_kind",
    "VarianceModifier",
    "encode_variance_modifier",
    "decode_variance_modifier",
    "to_json_variance_modifier",
    "from_json_variance_modifier",
    "BindingAnchor",
    "encode_binding_anchor",
    "decode_binding_anchor",
    "to_json_binding_anchor",
    "from_json_binding_anchor",
    "BindingOperator",
    "encode_binding_operator",
    "decode_binding_operator",
    "to_json_binding_operator",
    "from_json_binding_operator",
    "AccessorKind",
    "encode_accessor_kind",
    "decode_accessor_kind",
    "to_json_accessor_kind",
    "from_json_accessor_kind",
    "GenericParameter",
    "encode_generic_parameter",
    "decode_generic_parameter",
    "to_json_generic_parameter",
    "from_json_generic_parameter",
    "GenericParameterType",
    "Parameter",
    "encode_parameter",
    "decode_parameter",
    "to_json_parameter",
    "from_json_parameter",
    "ParameterNamed",
    "ParameterPattern",
    "ParameterVariadicNamed",
    "ParameterVariadicPattern",
    "Argument",
    "encode_argument",
    "decode_argument",
    "to_json_argument",
    "from_json_argument",
    "ArgumentPositional",
    "ArgumentSpread",
]
