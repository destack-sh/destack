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
    json_bool,
    json_field,
    json_int,
    json_number,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.core.string
import destack._generated.dir.tree.key
import destack._generated.dir.tree.node
import destack._generated.dir.type.primitive


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
class ScalarLiteralInteger:
    """Integer value."""

    integer: int
    kind: typing.Literal["integer"] = "integer"

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
class ScalarLiteralFloat:
    """Float value."""

    float: float
    kind: typing.Literal["float"] = "float"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_scalar_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_scalar_literal(self)


@dataclass(frozen=True, slots=True)
class ScalarLiteralCharacter:
    """Character value."""

    character: str
    kind: typing.Literal["character"] = "character"

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
    | ScalarLiteralInteger
    | ScalarLiteralBigint
    | ScalarLiteralFloat
    | ScalarLiteralCharacter
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
    elif value.kind == "integer":
        writer.write_unsigned(3)
        writer.write_signed(value.integer)
    elif value.kind == "bigint":
        writer.write_unsigned(4)
        writer.write_signed(value.bigint)
    elif value.kind == "float":
        writer.write_unsigned(5)
        writer.write_f64(value.float)
    elif value.kind == "character":
        writer.write_unsigned(6)
        writer.write_char(value.character)
    elif value.kind == "string":
        writer.write_unsigned(7)
        destack._generated.core.string.encode_string_id(writer, value.string)
    elif value.kind == "regexString":
        writer.write_unsigned(8)
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
        integer = reader.read_signed_number()

        return ScalarLiteralInteger(integer=integer)
    elif variant == 4:
        bigint = reader.read_signed_number()

        return ScalarLiteralBigint(bigint=bigint)
    elif variant == 5:
        float = reader.read_f64()

        return ScalarLiteralFloat(float=float)
    elif variant == 6:
        character = reader.read_char()

        return ScalarLiteralCharacter(character=character)
    elif variant == 7:
        string = destack._generated.core.string.decode_string_id(reader)

        return ScalarLiteralString(string=string)
    elif variant == 8:
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
    elif value.kind == "integer":
        return {
            "kind": "integer",
            "integer": value.integer,
        }
    elif value.kind == "bigint":
        return {
            "kind": "bigint",
            "bigint": value.bigint,
        }
    elif value.kind == "float":
        return {
            "kind": "float",
            "float": value.float,
        }
    elif value.kind == "character":
        return {
            "kind": "character",
            "character": value.character,
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
    elif kind == "integer":
        return ScalarLiteralInteger(integer=json_int(json_field(object_, "integer")))
    elif kind == "bigint":
        return ScalarLiteralBigint(bigint=json_int(json_field(object_, "bigint")))
    elif kind == "float":
        return ScalarLiteralFloat(float=json_number(json_field(object_, "float")))
    elif kind == "character":
        return ScalarLiteralCharacter(
            character=json_string(json_field(object_, "character"))
        )
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

    string: destack._generated.core.string.StringId
    kind: typing.Literal["string"] = "string"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_template_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_template_literal(self)


@dataclass(frozen=True, slots=True)
class TemplateLiteralInterpolatedString:
    """Interpolated template literal value."""

    strings: Sequence[destack._generated.core.string.StringId]
    arguments: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["interpolatedString"] = "interpolatedString"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_template_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_template_literal(self)


"""A TemplateLiteral is literal template value."""
TemplateLiteral: typing.TypeAlias = (
    TemplateLiteralString | TemplateLiteralInterpolatedString
)


def encode_template_literal(writer: BinaryWriter, value: TemplateLiteral) -> None:
    """Encode one TemplateLiteral."""
    if value.kind == "string":
        writer.write_unsigned(0)
        destack._generated.core.string.encode_string_id(writer, value.string)
    elif value.kind == "interpolatedString":
        writer.write_unsigned(1)
        writer.write_unsigned(len(value.strings))
        for item_value_strings_0 in value.strings:
            destack._generated.core.string.encode_string_id(
                writer, item_value_strings_0
            )
        writer.write_unsigned(len(value.arguments))
        for item_value_arguments_0 in value.arguments:
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, item_value_arguments_0
            )
    else:
        raise SerdeError("unknown enum variant")


def decode_template_literal(reader: BinaryReader) -> TemplateLiteral:
    """Decode one TemplateLiteral."""
    variant = reader.read_number()

    if variant == 0:
        string = destack._generated.core.string.decode_string_id(reader)

        return TemplateLiteralString(
            string=string,
        )
    elif variant == 1:
        strings = [
            destack._generated.core.string.decode_string_id(reader)
            for _ in range(reader.read_number())
        ]
        arguments = [
            destack._generated.dir.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return TemplateLiteralInterpolatedString(
            strings=strings,
            arguments=arguments,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_template_literal(value: TemplateLiteral) -> Json:
    """Return one JSON value for one TemplateLiteral."""
    if value.kind == "string":
        return {
            "kind": "string",
            "string": destack._generated.core.string.to_json_string_id(value.string),
        }
    elif value.kind == "interpolatedString":
        return {
            "kind": "interpolatedString",
            "strings": [
                destack._generated.core.string.to_json_string_id(item_0)
                for item_0 in value.strings
            ],
            "arguments": [
                destack._generated.dir.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.arguments
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
            string=destack._generated.core.string.from_json_string_id(
                json_field(object_, "string")
            ),
        )
    elif kind == "interpolatedString":
        return TemplateLiteralInterpolatedString(
            strings=[
                destack._generated.core.string.from_json_string_id(item_0)
                for item_0 in json_array(json_field(object_, "strings"))
            ],
            arguments=[
                destack._generated.dir.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "arguments"))
            ],
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class TypeLiteralNever:
    """Never type `never`."""

    kind: typing.Literal["never"] = "never"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_literal(self)


@dataclass(frozen=True, slots=True)
class TypeLiteralAny:
    """Any type `any`."""

    kind: typing.Literal["any"] = "any"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_literal(self)


@dataclass(frozen=True, slots=True)
class TypeLiteralUndefined:
    """Uninitialized type and value."""

    kind: typing.Literal["undefined"] = "undefined"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_literal(self)


@dataclass(frozen=True, slots=True)
class TypeLiteralUnknown:
    """Unknown type."""

    kind: typing.Literal["unknown"] = "unknown"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_literal(self)


@dataclass(frozen=True, slots=True)
class TypeLiteralObject:
    """Object type (any non-primitive)."""

    kind: typing.Literal["object"] = "object"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_literal(self)


@dataclass(frozen=True, slots=True)
class TypeLiteralVoid:
    """Void type."""

    kind: typing.Literal["void"] = "void"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_literal(self)


@dataclass(frozen=True, slots=True)
class TypeLiteralNull:
    """Null type and value."""

    kind: typing.Literal["null"] = "null"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_literal(self)


@dataclass(frozen=True, slots=True)
class TypeLiteralBoolean:
    """Boolean type."""

    kind: typing.Literal["boolean"] = "boolean"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_literal(self)


@dataclass(frozen=True, slots=True)
class TypeLiteralCharacter:
    """Character type."""

    kind: typing.Literal["character"] = "character"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_literal(self)


@dataclass(frozen=True, slots=True)
class TypeLiteralString:
    """String type (unsized)."""

    kind: typing.Literal["string"] = "string"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_literal(self)


@dataclass(frozen=True, slots=True)
class TypeLiteralBigint:
    """Bigint type (unsized)."""

    kind: typing.Literal["bigint"] = "bigint"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_literal(self)


@dataclass(frozen=True, slots=True)
class TypeLiteralNumber:
    """`number`, the `float64` source alias."""

    kind: typing.Literal["number"] = "number"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_literal(self)


@dataclass(frozen=True, slots=True)
class TypeLiteralAlias:
    """A widthless source alias for a sized scalar, like `int` for `int64`."""

    alias: destack._generated.dir.type.primitive.ScalarAlias
    kind: typing.Literal["alias"] = "alias"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_literal(self)


@dataclass(frozen=True, slots=True)
class TypeLiteralInteger:
    """Width-spelled integer type, like `int32` or `usize`."""

    integer: destack._generated.dir.type.primitive.IntegerType
    kind: typing.Literal["integer"] = "integer"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_literal(self)


@dataclass(frozen=True, slots=True)
class TypeLiteralFloat:
    """Width-spelled floating-point type, like `float32`."""

    float: destack._generated.dir.type.primitive.FloatType
    kind: typing.Literal["float"] = "float"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_literal(self)


@dataclass(frozen=True, slots=True)
class TypeLiteralSymbol:
    """Symbol type."""

    kind: typing.Literal["symbol"] = "symbol"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_literal(self)


@dataclass(frozen=True, slots=True)
class TypeLiteralUniqueSymbol:
    """Unique symbol type."""

    kind: typing.Literal["uniqueSymbol"] = "uniqueSymbol"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_literal(self)


"""A TypeLiteral is literal type."""
TypeLiteral: typing.TypeAlias = (
    TypeLiteralNever
    | TypeLiteralAny
    | TypeLiteralUndefined
    | TypeLiteralUnknown
    | TypeLiteralObject
    | TypeLiteralVoid
    | TypeLiteralNull
    | TypeLiteralBoolean
    | TypeLiteralCharacter
    | TypeLiteralString
    | TypeLiteralBigint
    | TypeLiteralNumber
    | TypeLiteralAlias
    | TypeLiteralInteger
    | TypeLiteralFloat
    | TypeLiteralSymbol
    | TypeLiteralUniqueSymbol
)


def encode_type_literal(writer: BinaryWriter, value: TypeLiteral) -> None:
    """Encode one TypeLiteral."""
    if value.kind == "never":
        writer.write_unsigned(0)
    elif value.kind == "any":
        writer.write_unsigned(1)
    elif value.kind == "undefined":
        writer.write_unsigned(2)
    elif value.kind == "unknown":
        writer.write_unsigned(3)
    elif value.kind == "object":
        writer.write_unsigned(4)
    elif value.kind == "void":
        writer.write_unsigned(5)
    elif value.kind == "null":
        writer.write_unsigned(6)
    elif value.kind == "boolean":
        writer.write_unsigned(7)
    elif value.kind == "character":
        writer.write_unsigned(8)
    elif value.kind == "string":
        writer.write_unsigned(9)
    elif value.kind == "bigint":
        writer.write_unsigned(10)
    elif value.kind == "number":
        writer.write_unsigned(11)
    elif value.kind == "alias":
        writer.write_unsigned(12)
        destack._generated.dir.type.primitive.encode_scalar_alias(writer, value.alias)
    elif value.kind == "integer":
        writer.write_unsigned(13)
        destack._generated.dir.type.primitive.encode_integer_type(writer, value.integer)
    elif value.kind == "float":
        writer.write_unsigned(14)
        destack._generated.dir.type.primitive.encode_float_type(writer, value.float)
    elif value.kind == "symbol":
        writer.write_unsigned(15)
    elif value.kind == "uniqueSymbol":
        writer.write_unsigned(16)
    else:
        raise SerdeError("unknown enum variant")


def decode_type_literal(reader: BinaryReader) -> TypeLiteral:
    """Decode one TypeLiteral."""
    variant = reader.read_number()

    if variant == 0:
        return TypeLiteralNever()
    elif variant == 1:
        return TypeLiteralAny()
    elif variant == 2:
        return TypeLiteralUndefined()
    elif variant == 3:
        return TypeLiteralUnknown()
    elif variant == 4:
        return TypeLiteralObject()
    elif variant == 5:
        return TypeLiteralVoid()
    elif variant == 6:
        return TypeLiteralNull()
    elif variant == 7:
        return TypeLiteralBoolean()
    elif variant == 8:
        return TypeLiteralCharacter()
    elif variant == 9:
        return TypeLiteralString()
    elif variant == 10:
        return TypeLiteralBigint()
    elif variant == 11:
        return TypeLiteralNumber()
    elif variant == 12:
        alias = destack._generated.dir.type.primitive.decode_scalar_alias(reader)

        return TypeLiteralAlias(alias=alias)
    elif variant == 13:
        integer = destack._generated.dir.type.primitive.decode_integer_type(reader)

        return TypeLiteralInteger(integer=integer)
    elif variant == 14:
        float = destack._generated.dir.type.primitive.decode_float_type(reader)

        return TypeLiteralFloat(float=float)
    elif variant == 15:
        return TypeLiteralSymbol()
    elif variant == 16:
        return TypeLiteralUniqueSymbol()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_type_literal(value: TypeLiteral) -> Json:
    """Return one JSON value for one TypeLiteral."""
    if value.kind == "never":
        return {
            "kind": "never",
        }
    elif value.kind == "any":
        return {
            "kind": "any",
        }
    elif value.kind == "undefined":
        return {
            "kind": "undefined",
        }
    elif value.kind == "unknown":
        return {
            "kind": "unknown",
        }
    elif value.kind == "object":
        return {
            "kind": "object",
        }
    elif value.kind == "void":
        return {
            "kind": "void",
        }
    elif value.kind == "null":
        return {
            "kind": "null",
        }
    elif value.kind == "boolean":
        return {
            "kind": "boolean",
        }
    elif value.kind == "character":
        return {
            "kind": "character",
        }
    elif value.kind == "string":
        return {
            "kind": "string",
        }
    elif value.kind == "bigint":
        return {
            "kind": "bigint",
        }
    elif value.kind == "number":
        return {
            "kind": "number",
        }
    elif value.kind == "alias":
        return {
            "kind": "alias",
            "alias": destack._generated.dir.type.primitive.to_json_scalar_alias(
                value.alias
            ),
        }
    elif value.kind == "integer":
        return {
            "kind": "integer",
            "integer": destack._generated.dir.type.primitive.to_json_integer_type(
                value.integer
            ),
        }
    elif value.kind == "float":
        return {
            "kind": "float",
            "float": destack._generated.dir.type.primitive.to_json_float_type(
                value.float
            ),
        }
    elif value.kind == "symbol":
        return {
            "kind": "symbol",
        }
    elif value.kind == "uniqueSymbol":
        return {
            "kind": "uniqueSymbol",
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_type_literal(value: Json) -> TypeLiteral:
    """Return one TypeLiteral from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "never":
        return TypeLiteralNever()
    elif kind == "any":
        return TypeLiteralAny()
    elif kind == "undefined":
        return TypeLiteralUndefined()
    elif kind == "unknown":
        return TypeLiteralUnknown()
    elif kind == "object":
        return TypeLiteralObject()
    elif kind == "void":
        return TypeLiteralVoid()
    elif kind == "null":
        return TypeLiteralNull()
    elif kind == "boolean":
        return TypeLiteralBoolean()
    elif kind == "character":
        return TypeLiteralCharacter()
    elif kind == "string":
        return TypeLiteralString()
    elif kind == "bigint":
        return TypeLiteralBigint()
    elif kind == "number":
        return TypeLiteralNumber()
    elif kind == "alias":
        return TypeLiteralAlias(
            alias=destack._generated.dir.type.primitive.from_json_scalar_alias(
                json_field(object_, "alias")
            )
        )
    elif kind == "integer":
        return TypeLiteralInteger(
            integer=destack._generated.dir.type.primitive.from_json_integer_type(
                json_field(object_, "integer")
            )
        )
    elif kind == "float":
        return TypeLiteralFloat(
            float=destack._generated.dir.type.primitive.from_json_float_type(
                json_field(object_, "float")
            )
        )
    elif kind == "symbol":
        return TypeLiteralSymbol()
    elif kind == "uniqueSymbol":
        return TypeLiteralUniqueSymbol()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class TreeAttributeNamed:
    """Named attribute with an optional value."""

    name: destack._generated.dir.tree.key.Name
    value: TreeAttributeValue | None
    kind: typing.Literal["named"] = "named"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tree_attribute(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tree_attribute(self)


@dataclass(frozen=True, slots=True)
class TreeAttributeSpread:
    """Spread attribute."""

    value: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["spread"] = "spread"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tree_attribute(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tree_attribute(self)


@dataclass(frozen=True, slots=True)
class TreeAttributeError:
    """Malformed attribute slot."""

    kind: typing.Literal["error"] = "error"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tree_attribute(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tree_attribute(self)


"""A tree tag attribute."""
TreeAttribute: typing.TypeAlias = (
    TreeAttributeNamed | TreeAttributeSpread | TreeAttributeError
)


def encode_tree_attribute(writer: BinaryWriter, value: TreeAttribute) -> None:
    """Encode one TreeAttribute."""
    if value.kind == "named":
        writer.write_unsigned(0)
        destack._generated.dir.tree.key.encode_name(writer, value.name)
        if value.value is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            encode_tree_attribute_value(writer, value.value)
    elif value.kind == "spread":
        writer.write_unsigned(1)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.value)
    elif value.kind == "error":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_tree_attribute(reader: BinaryReader) -> TreeAttribute:
    """Decode one TreeAttribute."""
    variant = reader.read_number()

    if variant == 0:
        name = destack._generated.dir.tree.key.decode_name(reader)
        value_ = reader.read_option(lambda: decode_tree_attribute_value(reader))

        return TreeAttributeNamed(
            name=name,
            value=value_,
        )
    elif variant == 1:
        value_ = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return TreeAttributeSpread(
            value=value_,
        )
    elif variant == 2:
        return TreeAttributeError()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_tree_attribute(value: TreeAttribute) -> Json:
    """Return one JSON value for one TreeAttribute."""
    if value.kind == "named":
        return {
            "kind": "named",
            "name": destack._generated.dir.tree.key.to_json_name(value.name),
            **(
                {}
                if value.value is None
                else {"value": to_json_tree_attribute_value(value.value)}
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


def from_json_tree_attribute(value: Json) -> TreeAttribute:
    """Return one TreeAttribute from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "named":
        return TreeAttributeNamed(
            name=destack._generated.dir.tree.key.from_json_name(
                json_field(object_, "name")
            ),
            value=json_optional(
                object_, "value", lambda value: from_json_tree_attribute_value(value)
            ),
        )
    elif kind == "spread":
        return TreeAttributeSpread(
            value=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "value")
            ),
        )
    elif kind == "error":
        return TreeAttributeError()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class TreeAttributeValueString:
    """Quoted string attribute value."""

    string: destack._generated.core.string.StringId
    kind: typing.Literal["string"] = "string"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tree_attribute_value(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tree_attribute_value(self)


@dataclass(frozen=True, slots=True)
class TreeAttributeValueExpression:
    """Expression container attribute value."""

    expression: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["expression"] = "expression"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tree_attribute_value(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tree_attribute_value(self)


"""The value form of a tree tag attribute."""
TreeAttributeValue: typing.TypeAlias = (
    TreeAttributeValueString | TreeAttributeValueExpression
)


def encode_tree_attribute_value(
    writer: BinaryWriter, value: TreeAttributeValue
) -> None:
    """Encode one TreeAttributeValue."""
    if value.kind == "string":
        writer.write_unsigned(0)
        destack._generated.core.string.encode_string_id(writer, value.string)
    elif value.kind == "expression":
        writer.write_unsigned(1)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.expression)
    else:
        raise SerdeError("unknown enum variant")


def decode_tree_attribute_value(reader: BinaryReader) -> TreeAttributeValue:
    """Decode one TreeAttributeValue."""
    variant = reader.read_number()

    if variant == 0:
        string = destack._generated.core.string.decode_string_id(reader)

        return TreeAttributeValueString(string=string)
    elif variant == 1:
        expression = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return TreeAttributeValueExpression(expression=expression)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_tree_attribute_value(value: TreeAttributeValue) -> Json:
    """Return one JSON value for one TreeAttributeValue."""
    if value.kind == "string":
        return {
            "kind": "string",
            "string": destack._generated.core.string.to_json_string_id(value.string),
        }
    elif value.kind == "expression":
        return {
            "kind": "expression",
            "expression": destack._generated.dir.tree.node.to_json_local_node_id(
                value.expression
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_tree_attribute_value(value: Json) -> TreeAttributeValue:
    """Return one TreeAttributeValue from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "string":
        return TreeAttributeValueString(
            string=destack._generated.core.string.from_json_string_id(
                json_field(object_, "string")
            )
        )
    elif kind == "expression":
        return TreeAttributeValueExpression(
            expression=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "expression")
            )
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class TreeChildText:
    """Raw tree text."""

    value: destack._generated.core.string.StringId
    kind: typing.Literal["text"] = "text"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tree_child(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tree_child(self)


@dataclass(frozen=True, slots=True)
class TreeChildExpression:
    """Expression container child."""

    value: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["expression"] = "expression"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tree_child(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tree_child(self)


@dataclass(frozen=True, slots=True)
class TreeChildSpread:
    """Spread expression container child."""

    value: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["spread"] = "spread"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tree_child(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tree_child(self)


@dataclass(frozen=True, slots=True)
class TreeChildTree:
    """Nested tree expression child."""

    value: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["tree"] = "tree"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tree_child(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tree_child(self)


@dataclass(frozen=True, slots=True)
class TreeChildError:
    """Malformed child slot."""

    kind: typing.Literal["error"] = "error"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tree_child(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tree_child(self)


"""A tree child."""
TreeChild: typing.TypeAlias = (
    TreeChildText
    | TreeChildExpression
    | TreeChildSpread
    | TreeChildTree
    | TreeChildError
)


def encode_tree_child(writer: BinaryWriter, value: TreeChild) -> None:
    """Encode one TreeChild."""
    if value.kind == "text":
        writer.write_unsigned(0)
        destack._generated.core.string.encode_string_id(writer, value.value)
    elif value.kind == "expression":
        writer.write_unsigned(1)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.value)
    elif value.kind == "spread":
        writer.write_unsigned(2)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.value)
    elif value.kind == "tree":
        writer.write_unsigned(3)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.value)
    elif value.kind == "error":
        writer.write_unsigned(4)
    else:
        raise SerdeError("unknown enum variant")


def decode_tree_child(reader: BinaryReader) -> TreeChild:
    """Decode one TreeChild."""
    variant = reader.read_number()

    if variant == 0:
        value_ = destack._generated.core.string.decode_string_id(reader)

        return TreeChildText(
            value=value_,
        )
    elif variant == 1:
        value_ = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return TreeChildExpression(
            value=value_,
        )
    elif variant == 2:
        value_ = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return TreeChildSpread(
            value=value_,
        )
    elif variant == 3:
        value_ = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return TreeChildTree(
            value=value_,
        )
    elif variant == 4:
        return TreeChildError()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_tree_child(value: TreeChild) -> Json:
    """Return one JSON value for one TreeChild."""
    if value.kind == "text":
        return {
            "kind": "text",
            "value": destack._generated.core.string.to_json_string_id(value.value),
        }
    elif value.kind == "expression":
        return {
            "kind": "expression",
            "value": destack._generated.dir.tree.node.to_json_local_node_id(
                value.value
            ),
        }
    elif value.kind == "spread":
        return {
            "kind": "spread",
            "value": destack._generated.dir.tree.node.to_json_local_node_id(
                value.value
            ),
        }
    elif value.kind == "tree":
        return {
            "kind": "tree",
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


def from_json_tree_child(value: Json) -> TreeChild:
    """Return one TreeChild from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "text":
        return TreeChildText(
            value=destack._generated.core.string.from_json_string_id(
                json_field(object_, "value")
            ),
        )
    elif kind == "expression":
        return TreeChildExpression(
            value=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "value")
            ),
        )
    elif kind == "spread":
        return TreeChildSpread(
            value=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "value")
            ),
        )
    elif kind == "tree":
        return TreeChildTree(
            value=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "value")
            ),
        )
    elif kind == "error":
        return TreeChildError()
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
    "ScalarLiteralInteger",
    "ScalarLiteralBigint",
    "ScalarLiteralFloat",
    "ScalarLiteralCharacter",
    "ScalarLiteralString",
    "ScalarLiteralRegexString",
    "TemplateLiteral",
    "encode_template_literal",
    "decode_template_literal",
    "to_json_template_literal",
    "from_json_template_literal",
    "TemplateLiteralString",
    "TemplateLiteralInterpolatedString",
    "TypeLiteral",
    "encode_type_literal",
    "decode_type_literal",
    "to_json_type_literal",
    "from_json_type_literal",
    "TypeLiteralNever",
    "TypeLiteralAny",
    "TypeLiteralUndefined",
    "TypeLiteralUnknown",
    "TypeLiteralObject",
    "TypeLiteralVoid",
    "TypeLiteralNull",
    "TypeLiteralBoolean",
    "TypeLiteralCharacter",
    "TypeLiteralString",
    "TypeLiteralBigint",
    "TypeLiteralNumber",
    "TypeLiteralAlias",
    "TypeLiteralInteger",
    "TypeLiteralFloat",
    "TypeLiteralSymbol",
    "TypeLiteralUniqueSymbol",
    "TreeAttribute",
    "encode_tree_attribute",
    "decode_tree_attribute",
    "to_json_tree_attribute",
    "from_json_tree_attribute",
    "TreeAttributeNamed",
    "TreeAttributeSpread",
    "TreeAttributeError",
    "TreeAttributeValue",
    "encode_tree_attribute_value",
    "decode_tree_attribute_value",
    "to_json_tree_attribute_value",
    "from_json_tree_attribute_value",
    "TreeAttributeValueString",
    "TreeAttributeValueExpression",
    "TreeChild",
    "encode_tree_child",
    "decode_tree_child",
    "to_json_tree_child",
    "from_json_tree_child",
    "TreeChildText",
    "TreeChildExpression",
    "TreeChildSpread",
    "TreeChildTree",
    "TreeChildError",
]
