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
    json_int,
    json_number,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.core.string
import destack._generated.js.tree.node
import destack._generated.js.tree.path


@dataclass(frozen=True, slots=True)
class ScalarLiteralNull:
    """Null value."""

    kind: typing.Literal["null"] = "null"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_scalar_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_scalar_literal(self)


@dataclass(frozen=True, slots=True)
class ScalarLiteralUndefined:
    """Undefined value."""

    kind: typing.Literal["undefined"] = "undefined"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_scalar_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_scalar_literal(self)


@dataclass(frozen=True, slots=True)
class ScalarLiteralBoolean:
    """Boolean value."""

    boolean: bool
    kind: typing.Literal["boolean"] = "boolean"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_scalar_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_scalar_literal(self)


@dataclass(frozen=True, slots=True)
class ScalarLiteralNumber:
    """Number value."""

    number: float
    kind: typing.Literal["number"] = "number"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_scalar_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_scalar_literal(self)


@dataclass(frozen=True, slots=True)
class ScalarLiteralBigint:
    """Bigint value."""

    bigint: int
    kind: typing.Literal["bigint"] = "bigint"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_scalar_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_scalar_literal(self)


@dataclass(frozen=True, slots=True)
class ScalarLiteralString:
    """String value."""

    string: destack._generated.core.string.StringId
    kind: typing.Literal["string"] = "string"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_scalar_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_scalar_literal(self)


@dataclass(frozen=True, slots=True)
class ScalarLiteralRegexString:
    """Regex string value."""

    content: destack._generated.core.string.StringId
    flags: destack._generated.core.string.StringId | None
    kind: typing.Literal["regexString"] = "regexString"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_scalar_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_scalar_literal(self)


"""A ScalarLiteral is literal scalar value."""
ScalarLiteral: typing.TypeAlias = (
    ScalarLiteralNull
    | ScalarLiteralUndefined
    | ScalarLiteralBoolean
    | ScalarLiteralNumber
    | ScalarLiteralBigint
    | ScalarLiteralString
    | ScalarLiteralRegexString
)


def encode_scalar_literal(writer: BinaryWriter, value: ScalarLiteral) -> None:
    """Encode one ScalarLiteral."""
    if value.kind == "null":
        writer.write_unsigned(0)
    elif value.kind == "undefined":
        writer.write_unsigned(1)
    elif value.kind == "boolean":
        writer.write_unsigned(2)
        writer.write_bool(value.boolean)
    elif value.kind == "number":
        writer.write_unsigned(3)
        writer.write_f64(value.number)
    elif value.kind == "bigint":
        writer.write_unsigned(4)
        writer.write_signed(value.bigint)
    elif value.kind == "string":
        writer.write_unsigned(5)
        destack._generated.core.string.encode_string_id(writer, value.string)
    elif value.kind == "regexString":
        writer.write_unsigned(6)
        destack._generated.core.string.encode_string_id(writer, value.content)
        if value.flags is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.core.string.encode_string_id(writer, value.flags)
    else:
        raise SerdeError("unknown enum variant")


def decode_scalar_literal(reader: BinaryReader) -> ScalarLiteral:
    """Decode one ScalarLiteral."""
    variant = reader.read_number()

    if variant == 0:
        return ScalarLiteralNull()
    elif variant == 1:
        return ScalarLiteralUndefined()
    elif variant == 2:
        boolean = reader.read_bool()

        return ScalarLiteralBoolean(boolean=boolean)
    elif variant == 3:
        number = reader.read_f64()

        return ScalarLiteralNumber(number=number)
    elif variant == 4:
        bigint = reader.read_signed_number()

        return ScalarLiteralBigint(bigint=bigint)
    elif variant == 5:
        string = destack._generated.core.string.decode_string_id(reader)

        return ScalarLiteralString(string=string)
    elif variant == 6:
        content = destack._generated.core.string.decode_string_id(reader)
        flags = reader.read_option(
            lambda: destack._generated.core.string.decode_string_id(reader)
        )

        return ScalarLiteralRegexString(
            content=content,
            flags=flags,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_scalar_literal(value: ScalarLiteral) -> Json:
    """Return one JSON value for one ScalarLiteral."""
    if value.kind == "null":
        return {
            "kind": "null",
        }
    elif value.kind == "undefined":
        return {
            "kind": "undefined",
        }
    elif value.kind == "boolean":
        return {
            "kind": "boolean",
            "boolean": value.boolean,
        }
    elif value.kind == "number":
        return {
            "kind": "number",
            "number": value.number,
        }
    elif value.kind == "bigint":
        return {
            "kind": "bigint",
            "bigint": value.bigint,
        }
    elif value.kind == "string":
        return {
            "kind": "string",
            "string": destack._generated.core.string.to_json_string_id(value.string),
        }
    elif value.kind == "regexString":
        return {
            "kind": "regexString",
            "content": destack._generated.core.string.to_json_string_id(value.content),
            **(
                {}
                if value.flags is None
                else {
                    "flags": destack._generated.core.string.to_json_string_id(
                        value.flags
                    )
                }
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_scalar_literal(value: Json) -> ScalarLiteral:
    """Return one ScalarLiteral from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "null":
        return ScalarLiteralNull()
    elif kind == "undefined":
        return ScalarLiteralUndefined()
    elif kind == "boolean":
        return ScalarLiteralBoolean(boolean=json_bool(json_field(object_, "boolean")))
    elif kind == "number":
        return ScalarLiteralNumber(number=json_number(json_field(object_, "number")))
    elif kind == "bigint":
        return ScalarLiteralBigint(bigint=json_int(json_field(object_, "bigint")))
    elif kind == "string":
        return ScalarLiteralString(
            string=destack._generated.core.string.from_json_string_id(
                json_field(object_, "string")
            )
        )
    elif kind == "regexString":
        return ScalarLiteralRegexString(
            content=destack._generated.core.string.from_json_string_id(
                json_field(object_, "content")
            ),
            flags=json_optional(
                object_,
                "flags",
                lambda value: destack._generated.core.string.from_json_string_id(value),
            ),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class TemplateLiteralString:
    """Template string value."""

    template: destack._generated.core.string.StringId
    kind: typing.Literal["string"] = "string"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_template_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_template_literal(self)


@dataclass(frozen=True, slots=True)
class TemplateLiteralTaggedString:
    """Tagged template literal value."""

    tag: destack._generated.js.tree.path.Path
    template: destack._generated.core.string.StringId
    kind: typing.Literal["taggedString"] = "taggedString"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_template_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_template_literal(self)


@dataclass(frozen=True, slots=True)
class TemplateLiteralInterpolatedString:
    """Interpolated template literal value."""

    template: Sequence[destack._generated.core.string.StringId]
    expressions: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["interpolatedString"] = "interpolatedString"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_template_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_template_literal(self)


@dataclass(frozen=True, slots=True)
class TemplateLiteralTaggedInterpolatedString:
    """Tagged interpolated template literal value."""

    tag: destack._generated.js.tree.path.Path
    template: Sequence[destack._generated.core.string.StringId]
    expressions: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["taggedInterpolatedString"] = "taggedInterpolatedString"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_template_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_template_literal(self)


"""A TemplateLiteral is literal template value."""
TemplateLiteral: typing.TypeAlias = (
    TemplateLiteralString
    | TemplateLiteralTaggedString
    | TemplateLiteralInterpolatedString
    | TemplateLiteralTaggedInterpolatedString
)


def encode_template_literal(writer: BinaryWriter, value: TemplateLiteral) -> None:
    """Encode one TemplateLiteral."""
    if value.kind == "string":
        writer.write_unsigned(0)
        destack._generated.core.string.encode_string_id(writer, value.template)
    elif value.kind == "taggedString":
        writer.write_unsigned(1)
        destack._generated.js.tree.path.encode_path(writer, value.tag)
        destack._generated.core.string.encode_string_id(writer, value.template)
    elif value.kind == "interpolatedString":
        writer.write_unsigned(2)
        writer.write_unsigned(len(value.template))
        for item_value_template_0 in value.template:
            destack._generated.core.string.encode_string_id(
                writer, item_value_template_0
            )
        writer.write_unsigned(len(value.expressions))
        for item_value_expressions_0 in value.expressions:
            destack._generated.js.tree.node.encode_local_node_id(
                writer, item_value_expressions_0
            )
    elif value.kind == "taggedInterpolatedString":
        writer.write_unsigned(3)
        destack._generated.js.tree.path.encode_path(writer, value.tag)
        writer.write_unsigned(len(value.template))
        for item_value_template_0 in value.template:
            destack._generated.core.string.encode_string_id(
                writer, item_value_template_0
            )
        writer.write_unsigned(len(value.expressions))
        for item_value_expressions_0 in value.expressions:
            destack._generated.js.tree.node.encode_local_node_id(
                writer, item_value_expressions_0
            )
    else:
        raise SerdeError("unknown enum variant")


def decode_template_literal(reader: BinaryReader) -> TemplateLiteral:
    """Decode one TemplateLiteral."""
    variant = reader.read_number()

    if variant == 0:
        template = destack._generated.core.string.decode_string_id(reader)

        return TemplateLiteralString(
            template=template,
        )
    elif variant == 1:
        tag = destack._generated.js.tree.path.decode_path(reader)
        template = destack._generated.core.string.decode_string_id(reader)

        return TemplateLiteralTaggedString(
            tag=tag,
            template=template,
        )
    elif variant == 2:
        template = [
            destack._generated.core.string.decode_string_id(reader)
            for _ in range(reader.read_number())
        ]
        expressions = [
            destack._generated.js.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return TemplateLiteralInterpolatedString(
            template=template,
            expressions=expressions,
        )
    elif variant == 3:
        tag = destack._generated.js.tree.path.decode_path(reader)
        template = [
            destack._generated.core.string.decode_string_id(reader)
            for _ in range(reader.read_number())
        ]
        expressions = [
            destack._generated.js.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return TemplateLiteralTaggedInterpolatedString(
            tag=tag,
            template=template,
            expressions=expressions,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_template_literal(value: TemplateLiteral) -> Json:
    """Return one JSON value for one TemplateLiteral."""
    if value.kind == "string":
        return {
            "kind": "string",
            "template": destack._generated.core.string.to_json_string_id(
                value.template
            ),
        }
    elif value.kind == "taggedString":
        return {
            "kind": "taggedString",
            "tag": destack._generated.js.tree.path.to_json_path(value.tag),
            "template": destack._generated.core.string.to_json_string_id(
                value.template
            ),
        }
    elif value.kind == "interpolatedString":
        return {
            "kind": "interpolatedString",
            "template": [
                destack._generated.core.string.to_json_string_id(item_0)
                for item_0 in value.template
            ],
            "expressions": [
                destack._generated.js.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.expressions
            ],
        }
    elif value.kind == "taggedInterpolatedString":
        return {
            "kind": "taggedInterpolatedString",
            "tag": destack._generated.js.tree.path.to_json_path(value.tag),
            "template": [
                destack._generated.core.string.to_json_string_id(item_0)
                for item_0 in value.template
            ],
            "expressions": [
                destack._generated.js.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.expressions
            ],
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_template_literal(value: Json) -> TemplateLiteral:
    """Return one TemplateLiteral from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "string":
        return TemplateLiteralString(
            template=destack._generated.core.string.from_json_string_id(
                json_field(object_, "template")
            ),
        )
    elif kind == "taggedString":
        return TemplateLiteralTaggedString(
            tag=destack._generated.js.tree.path.from_json_path(
                json_field(object_, "tag")
            ),
            template=destack._generated.core.string.from_json_string_id(
                json_field(object_, "template")
            ),
        )
    elif kind == "interpolatedString":
        return TemplateLiteralInterpolatedString(
            template=[
                destack._generated.core.string.from_json_string_id(item_0)
                for item_0 in json_array(json_field(object_, "template"))
            ],
            expressions=[
                destack._generated.js.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "expressions"))
            ],
        )
    elif kind == "taggedInterpolatedString":
        return TemplateLiteralTaggedInterpolatedString(
            tag=destack._generated.js.tree.path.from_json_path(
                json_field(object_, "tag")
            ),
            template=[
                destack._generated.core.string.from_json_string_id(item_0)
                for item_0 in json_array(json_field(object_, "template"))
            ],
            expressions=[
                destack._generated.js.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "expressions"))
            ],
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


__all__ = [
    "ScalarLiteral",
    "encode_scalar_literal",
    "decode_scalar_literal",
    "to_json_scalar_literal",
    "from_json_scalar_literal",
    "ScalarLiteralNull",
    "ScalarLiteralUndefined",
    "ScalarLiteralBoolean",
    "ScalarLiteralNumber",
    "ScalarLiteralBigint",
    "ScalarLiteralString",
    "ScalarLiteralRegexString",
    "TemplateLiteral",
    "encode_template_literal",
    "decode_template_literal",
    "to_json_template_literal",
    "from_json_template_literal",
    "TemplateLiteralString",
    "TemplateLiteralTaggedString",
    "TemplateLiteralInterpolatedString",
    "TemplateLiteralTaggedInterpolatedString",
]
