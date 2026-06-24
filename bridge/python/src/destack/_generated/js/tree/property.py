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

import destack._generated.js.tree.argument
import destack._generated.js.tree.function
import destack._generated.js.tree.key
import destack._generated.js.tree.node


@dataclass(frozen=True, slots=True)
class PropertyField:
    """Named field (like `x: int32`)."""

    modifiers: destack._generated.js.tree.argument.BindingModifier | None
    key: destack._generated.js.tree.key.Key
    value: destack._generated.js.tree.node.LocalNodeId
    is_shorthand: bool
    kind: typing.Literal["field"] = "field"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_property(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_property(self)


@dataclass(frozen=True, slots=True)
class PropertyMethod:
    """Named member function (like `foo()` or `<T>(): T`)."""

    modifiers: destack._generated.js.tree.argument.BindingModifier | None
    key: destack._generated.js.tree.key.Key | None
    signature: destack._generated.js.tree.function.FunctionSignature
    body: destack._generated.js.tree.node.LocalNodeId | None
    kind: typing.Literal["method"] = "method"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_property(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_property(self)


@dataclass(frozen=True, slots=True)
class PropertySpread:
    """Spread property (like `...a`)."""

    modifiers: destack._generated.js.tree.argument.BindingModifier | None
    value: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["spread"] = "spread"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_property(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_property(self)


"""A Property is a property of an object literal (may be a field, method, or spread)."""
Property: typing.TypeAlias = PropertyField | PropertyMethod | PropertySpread


def encode_property(writer: BinaryWriter, value: Property) -> None:
    """Encode one Property."""
    if value.kind == "field":
        writer.write_unsigned(0)
        if value.modifiers is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.js.tree.argument.encode_binding_modifier(
                writer, value.modifiers
            )
        destack._generated.js.tree.key.encode_key(writer, value.key)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.value)
        writer.write_bool(value.is_shorthand)
    elif value.kind == "method":
        writer.write_unsigned(1)
        if value.modifiers is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.js.tree.argument.encode_binding_modifier(
                writer, value.modifiers
            )
        if value.key is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.js.tree.key.encode_key(writer, value.key)
        destack._generated.js.tree.function.encode_function_signature(
            writer, value.signature
        )
        if value.body is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.js.tree.node.encode_local_node_id(writer, value.body)
    elif value.kind == "spread":
        writer.write_unsigned(2)
        if value.modifiers is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.js.tree.argument.encode_binding_modifier(
                writer, value.modifiers
            )
        destack._generated.js.tree.node.encode_local_node_id(writer, value.value)
    else:
        raise SerdeError("unknown enum variant")


def decode_property(reader: BinaryReader) -> Property:
    """Decode one Property."""
    variant = reader.read_number()

    if variant == 0:
        modifiers = reader.read_option(
            lambda: destack._generated.js.tree.argument.decode_binding_modifier(reader)
        )
        key = destack._generated.js.tree.key.decode_key(reader)
        value_ = destack._generated.js.tree.node.decode_local_node_id(reader)
        is_shorthand = reader.read_bool()

        return PropertyField(
            modifiers=modifiers,
            key=key,
            value=value_,
            is_shorthand=is_shorthand,
        )
    elif variant == 1:
        modifiers = reader.read_option(
            lambda: destack._generated.js.tree.argument.decode_binding_modifier(reader)
        )
        key = reader.read_option(
            lambda: destack._generated.js.tree.key.decode_key(reader)
        )
        signature = destack._generated.js.tree.function.decode_function_signature(
            reader
        )
        body = reader.read_option(
            lambda: destack._generated.js.tree.node.decode_local_node_id(reader)
        )

        return PropertyMethod(
            modifiers=modifiers,
            key=key,
            signature=signature,
            body=body,
        )
    elif variant == 2:
        modifiers = reader.read_option(
            lambda: destack._generated.js.tree.argument.decode_binding_modifier(reader)
        )
        value_ = destack._generated.js.tree.node.decode_local_node_id(reader)

        return PropertySpread(
            modifiers=modifiers,
            value=value_,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_property(value: Property) -> Json:
    """Return one JSON value for one Property."""
    if value.kind == "field":
        return {
            "kind": "field",
            **(
                {}
                if value.modifiers is None
                else {
                    "modifiers": destack._generated.js.tree.argument.to_json_binding_modifier(
                        value.modifiers
                    )
                }
            ),
            "key": destack._generated.js.tree.key.to_json_key(value.key),
            "value": destack._generated.js.tree.node.to_json_local_node_id(value.value),
            "isShorthand": value.is_shorthand,
        }
    elif value.kind == "method":
        return {
            "kind": "method",
            **(
                {}
                if value.modifiers is None
                else {
                    "modifiers": destack._generated.js.tree.argument.to_json_binding_modifier(
                        value.modifiers
                    )
                }
            ),
            **(
                {}
                if value.key is None
                else {"key": destack._generated.js.tree.key.to_json_key(value.key)}
            ),
            "signature": destack._generated.js.tree.function.to_json_function_signature(
                value.signature
            ),
            **(
                {}
                if value.body is None
                else {
                    "body": destack._generated.js.tree.node.to_json_local_node_id(
                        value.body
                    )
                }
            ),
        }
    elif value.kind == "spread":
        return {
            "kind": "spread",
            **(
                {}
                if value.modifiers is None
                else {
                    "modifiers": destack._generated.js.tree.argument.to_json_binding_modifier(
                        value.modifiers
                    )
                }
            ),
            "value": destack._generated.js.tree.node.to_json_local_node_id(value.value),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_property(value: Json) -> Property:
    """Return one Property from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "field":
        return PropertyField(
            modifiers=json_optional(
                object_,
                "modifiers",
                lambda value: (
                    destack._generated.js.tree.argument.from_json_binding_modifier(
                        value
                    )
                ),
            ),
            key=destack._generated.js.tree.key.from_json_key(
                json_field(object_, "key")
            ),
            value=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "value")
            ),
            is_shorthand=json_bool(json_field(object_, "isShorthand")),
        )
    elif kind == "method":
        return PropertyMethod(
            modifiers=json_optional(
                object_,
                "modifiers",
                lambda value: (
                    destack._generated.js.tree.argument.from_json_binding_modifier(
                        value
                    )
                ),
            ),
            key=json_optional(
                object_,
                "key",
                lambda value: destack._generated.js.tree.key.from_json_key(value),
            ),
            signature=destack._generated.js.tree.function.from_json_function_signature(
                json_field(object_, "signature")
            ),
            body=json_optional(
                object_,
                "body",
                lambda value: destack._generated.js.tree.node.from_json_local_node_id(
                    value
                ),
            ),
        )
    elif kind == "spread":
        return PropertySpread(
            modifiers=json_optional(
                object_,
                "modifiers",
                lambda value: (
                    destack._generated.js.tree.argument.from_json_binding_modifier(
                        value
                    )
                ),
            ),
            value=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "value")
            ),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class MemberField:
    """Named field (like `x: int32`)."""

    modifiers: destack._generated.js.tree.argument.BindingModifier | None
    key: destack._generated.js.tree.key.Key
    value: destack._generated.js.tree.node.LocalNodeId | None
    default: destack._generated.js.tree.node.LocalNodeId | None
    kind: typing.Literal["field"] = "field"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_member(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_member(self)


@dataclass(frozen=True, slots=True)
class MemberMethod:
    """Named member function (like `foo()` or `<T>(): T`)."""

    modifiers: destack._generated.js.tree.argument.BindingModifier | None
    key: destack._generated.js.tree.key.Key | None
    signature: destack._generated.js.tree.function.FunctionSignature
    body: destack._generated.js.tree.node.LocalNodeId | None
    kind: typing.Literal["method"] = "method"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_member(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_member(self)


@dataclass(frozen=True, slots=True)
class MemberStaticBlock:
    """Static initialization block (like `static { ... }`)."""

    body: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["staticBlock"] = "staticBlock"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_member(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_member(self)


"""A Member is a member of a object-like declaration."""
Member: typing.TypeAlias = MemberField | MemberMethod | MemberStaticBlock


def encode_member(writer: BinaryWriter, value: Member) -> None:
    """Encode one Member."""
    if value.kind == "field":
        writer.write_unsigned(0)
        if value.modifiers is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.js.tree.argument.encode_binding_modifier(
                writer, value.modifiers
            )
        destack._generated.js.tree.key.encode_key(writer, value.key)
        if value.value is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.js.tree.node.encode_local_node_id(writer, value.value)
        if value.default is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.js.tree.node.encode_local_node_id(writer, value.default)
    elif value.kind == "method":
        writer.write_unsigned(1)
        if value.modifiers is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.js.tree.argument.encode_binding_modifier(
                writer, value.modifiers
            )
        if value.key is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.js.tree.key.encode_key(writer, value.key)
        destack._generated.js.tree.function.encode_function_signature(
            writer, value.signature
        )
        if value.body is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.js.tree.node.encode_local_node_id(writer, value.body)
    elif value.kind == "staticBlock":
        writer.write_unsigned(2)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.body)
    else:
        raise SerdeError("unknown enum variant")


def decode_member(reader: BinaryReader) -> Member:
    """Decode one Member."""
    variant = reader.read_number()

    if variant == 0:
        modifiers = reader.read_option(
            lambda: destack._generated.js.tree.argument.decode_binding_modifier(reader)
        )
        key = destack._generated.js.tree.key.decode_key(reader)
        value_ = reader.read_option(
            lambda: destack._generated.js.tree.node.decode_local_node_id(reader)
        )
        default = reader.read_option(
            lambda: destack._generated.js.tree.node.decode_local_node_id(reader)
        )

        return MemberField(
            modifiers=modifiers,
            key=key,
            value=value_,
            default=default,
        )
    elif variant == 1:
        modifiers = reader.read_option(
            lambda: destack._generated.js.tree.argument.decode_binding_modifier(reader)
        )
        key = reader.read_option(
            lambda: destack._generated.js.tree.key.decode_key(reader)
        )
        signature = destack._generated.js.tree.function.decode_function_signature(
            reader
        )
        body = reader.read_option(
            lambda: destack._generated.js.tree.node.decode_local_node_id(reader)
        )

        return MemberMethod(
            modifiers=modifiers,
            key=key,
            signature=signature,
            body=body,
        )
    elif variant == 2:
        body = destack._generated.js.tree.node.decode_local_node_id(reader)

        return MemberStaticBlock(
            body=body,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_member(value: Member) -> Json:
    """Return one JSON value for one Member."""
    if value.kind == "field":
        return {
            "kind": "field",
            **(
                {}
                if value.modifiers is None
                else {
                    "modifiers": destack._generated.js.tree.argument.to_json_binding_modifier(
                        value.modifiers
                    )
                }
            ),
            "key": destack._generated.js.tree.key.to_json_key(value.key),
            **(
                {}
                if value.value is None
                else {
                    "value": destack._generated.js.tree.node.to_json_local_node_id(
                        value.value
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
    elif value.kind == "method":
        return {
            "kind": "method",
            **(
                {}
                if value.modifiers is None
                else {
                    "modifiers": destack._generated.js.tree.argument.to_json_binding_modifier(
                        value.modifiers
                    )
                }
            ),
            **(
                {}
                if value.key is None
                else {"key": destack._generated.js.tree.key.to_json_key(value.key)}
            ),
            "signature": destack._generated.js.tree.function.to_json_function_signature(
                value.signature
            ),
            **(
                {}
                if value.body is None
                else {
                    "body": destack._generated.js.tree.node.to_json_local_node_id(
                        value.body
                    )
                }
            ),
        }
    elif value.kind == "staticBlock":
        return {
            "kind": "staticBlock",
            "body": destack._generated.js.tree.node.to_json_local_node_id(value.body),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_member(value: Json) -> Member:
    """Return one Member from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "field":
        return MemberField(
            modifiers=json_optional(
                object_,
                "modifiers",
                lambda value: (
                    destack._generated.js.tree.argument.from_json_binding_modifier(
                        value
                    )
                ),
            ),
            key=destack._generated.js.tree.key.from_json_key(
                json_field(object_, "key")
            ),
            value=json_optional(
                object_,
                "value",
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
    elif kind == "method":
        return MemberMethod(
            modifiers=json_optional(
                object_,
                "modifiers",
                lambda value: (
                    destack._generated.js.tree.argument.from_json_binding_modifier(
                        value
                    )
                ),
            ),
            key=json_optional(
                object_,
                "key",
                lambda value: destack._generated.js.tree.key.from_json_key(value),
            ),
            signature=destack._generated.js.tree.function.from_json_function_signature(
                json_field(object_, "signature")
            ),
            body=json_optional(
                object_,
                "body",
                lambda value: destack._generated.js.tree.node.from_json_local_node_id(
                    value
                ),
            ),
        )
    elif kind == "staticBlock":
        return MemberStaticBlock(
            body=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "body")
            ),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


__all__ = [
    "Property",
    "encode_property",
    "decode_property",
    "to_json_property",
    "from_json_property",
    "PropertyField",
    "PropertyMethod",
    "PropertySpread",
    "Member",
    "encode_member",
    "decode_member",
    "to_json_member",
    "from_json_member",
    "MemberField",
    "MemberMethod",
    "MemberStaticBlock",
]
