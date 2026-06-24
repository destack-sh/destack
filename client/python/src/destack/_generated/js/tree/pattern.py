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
    json_object,
    json_optional,
    json_string,
)

import destack._generated.core.string
import destack._generated.js.tree.key
import destack._generated.js.tree.node


@dataclass(frozen=True, slots=True)
class PatternBinding:
    """Binding pattern (like `x`)."""

    mutability: destack._generated.js.tree.node.Mutability | None
    name: destack._generated.core.string.StringId
    kind: typing.Literal["binding"] = "binding"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern(self)


@dataclass(frozen=True, slots=True)
class PatternAssign:
    """Assignment pattern (like `x = 1`)."""

    pattern: destack._generated.js.tree.node.LocalNodeId
    value: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["assign"] = "assign"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern(self)


@dataclass(frozen=True, slots=True)
class PatternArray:
    """Array pattern (like `[1, 2, .., x, 3]`)."""

    fields: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["array"] = "array"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern(self)


@dataclass(frozen=True, slots=True)
class PatternObject:
    """Object pattern (like `{ a: 1, b: 2, ..., x: 3 }`)."""

    fields: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["object"] = "object"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern(self)


@dataclass(frozen=True, slots=True)
class PatternHole:
    """Hole pattern (like the empty in `, ,`)."""

    kind: typing.Literal["hole"] = "hole"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern(self)


"""A Pattern is a pattern to match something and unwrap it."""
Pattern: typing.TypeAlias = (
    PatternBinding | PatternAssign | PatternArray | PatternObject | PatternHole
)


def encode_pattern(writer: BinaryWriter, value: Pattern) -> None:
    """Encode one Pattern."""
    if value.kind == "binding":
        writer.write_unsigned(0)
        if value.mutability is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.js.tree.node.encode_mutability(writer, value.mutability)
        destack._generated.core.string.encode_string_id(writer, value.name)
    elif value.kind == "assign":
        writer.write_unsigned(1)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.pattern)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.value)
    elif value.kind == "array":
        writer.write_unsigned(2)
        writer.write_unsigned(len(value.fields))
        for item_value_fields_0 in value.fields:
            destack._generated.js.tree.node.encode_local_node_id(
                writer, item_value_fields_0
            )
    elif value.kind == "object":
        writer.write_unsigned(3)
        writer.write_unsigned(len(value.fields))
        for item_value_fields_0 in value.fields:
            destack._generated.js.tree.node.encode_local_node_id(
                writer, item_value_fields_0
            )
    elif value.kind == "hole":
        writer.write_unsigned(4)
    else:
        raise SerdeError("unknown enum variant")


def decode_pattern(reader: BinaryReader) -> Pattern:
    """Decode one Pattern."""
    variant = reader.read_number()

    if variant == 0:
        mutability = reader.read_option(
            lambda: destack._generated.js.tree.node.decode_mutability(reader)
        )
        name = destack._generated.core.string.decode_string_id(reader)

        return PatternBinding(
            mutability=mutability,
            name=name,
        )
    elif variant == 1:
        pattern = destack._generated.js.tree.node.decode_local_node_id(reader)
        value_ = destack._generated.js.tree.node.decode_local_node_id(reader)

        return PatternAssign(
            pattern=pattern,
            value=value_,
        )
    elif variant == 2:
        fields = [
            destack._generated.js.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return PatternArray(
            fields=fields,
        )
    elif variant == 3:
        fields = [
            destack._generated.js.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return PatternObject(
            fields=fields,
        )
    elif variant == 4:
        return PatternHole()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_pattern(value: Pattern) -> Json:
    """Return one JSON value for one Pattern."""
    if value.kind == "binding":
        return {
            "kind": "binding",
            **(
                {}
                if value.mutability is None
                else {
                    "mutability": destack._generated.js.tree.node.to_json_mutability(
                        value.mutability
                    )
                }
            ),
            "name": destack._generated.core.string.to_json_string_id(value.name),
        }
    elif value.kind == "assign":
        return {
            "kind": "assign",
            "pattern": destack._generated.js.tree.node.to_json_local_node_id(
                value.pattern
            ),
            "value": destack._generated.js.tree.node.to_json_local_node_id(value.value),
        }
    elif value.kind == "array":
        return {
            "kind": "array",
            "fields": [
                destack._generated.js.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.fields
            ],
        }
    elif value.kind == "object":
        return {
            "kind": "object",
            "fields": [
                destack._generated.js.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.fields
            ],
        }
    elif value.kind == "hole":
        return {
            "kind": "hole",
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_pattern(value: Json) -> Pattern:
    """Return one Pattern from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "binding":
        return PatternBinding(
            mutability=json_optional(
                object_,
                "mutability",
                lambda value: destack._generated.js.tree.node.from_json_mutability(
                    value
                ),
            ),
            name=destack._generated.core.string.from_json_string_id(
                json_field(object_, "name")
            ),
        )
    elif kind == "assign":
        return PatternAssign(
            pattern=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "pattern")
            ),
            value=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "value")
            ),
        )
    elif kind == "array":
        return PatternArray(
            fields=[
                destack._generated.js.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "fields"))
            ],
        )
    elif kind == "object":
        return PatternObject(
            fields=[
                destack._generated.js.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "fields"))
            ],
        )
    elif kind == "hole":
        return PatternHole()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class PatternFieldNamed:
    """Named pattern field (like `x` or `x: y` or `x = 4`)."""

    mutability: destack._generated.js.tree.node.Mutability | None
    name: destack._generated.core.string.StringId
    is_shorthand: bool
    pattern: destack._generated.js.tree.node.LocalNodeId | None
    kind: typing.Literal["named"] = "named"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_field(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_field(self)


@dataclass(frozen=True, slots=True)
class PatternFieldComputed:
    """Computed pattern field (like `[key]: value`)."""

    mutability: destack._generated.js.tree.node.Mutability | None
    key: destack._generated.js.tree.node.LocalNodeId
    pattern: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["computed"] = "computed"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_field(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_field(self)


@dataclass(frozen=True, slots=True)
class PatternFieldPositional:
    """Positional field with a pattern (like `4` or `x = 1`)."""

    pattern: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["positional"] = "positional"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_field(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_field(self)


@dataclass(frozen=True, slots=True)
class PatternFieldSpread:
    """Spread field (like `...x` or `...[a, b]`)."""

    mutability: destack._generated.js.tree.node.Mutability | None
    pattern: destack._generated.js.tree.node.LocalNodeId | None
    kind: typing.Literal["spread"] = "spread"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_field(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_field(self)


@dataclass(frozen=True, slots=True)
class PatternFieldElision:
    """Elision (hole) in an array pattern (like `[,a]`)."""

    kind: typing.Literal["elision"] = "elision"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_field(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_field(self)


"""A PatternField is a field in a pattern (object, array, etc.)."""
PatternField: typing.TypeAlias = (
    PatternFieldNamed
    | PatternFieldComputed
    | PatternFieldPositional
    | PatternFieldSpread
    | PatternFieldElision
)


def encode_pattern_field(writer: BinaryWriter, value: PatternField) -> None:
    """Encode one PatternField."""
    if value.kind == "named":
        writer.write_unsigned(0)
        if value.mutability is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.js.tree.node.encode_mutability(writer, value.mutability)
        destack._generated.core.string.encode_string_id(writer, value.name)
        writer.write_bool(value.is_shorthand)
        if value.pattern is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.js.tree.node.encode_local_node_id(writer, value.pattern)
    elif value.kind == "computed":
        writer.write_unsigned(1)
        if value.mutability is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.js.tree.node.encode_mutability(writer, value.mutability)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.key)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.pattern)
    elif value.kind == "positional":
        writer.write_unsigned(2)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.pattern)
    elif value.kind == "spread":
        writer.write_unsigned(3)
        if value.mutability is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.js.tree.node.encode_mutability(writer, value.mutability)
        if value.pattern is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.js.tree.node.encode_local_node_id(writer, value.pattern)
    elif value.kind == "elision":
        writer.write_unsigned(4)
    else:
        raise SerdeError("unknown enum variant")


def decode_pattern_field(reader: BinaryReader) -> PatternField:
    """Decode one PatternField."""
    variant = reader.read_number()

    if variant == 0:
        mutability = reader.read_option(
            lambda: destack._generated.js.tree.node.decode_mutability(reader)
        )
        name = destack._generated.core.string.decode_string_id(reader)
        is_shorthand = reader.read_bool()
        pattern = reader.read_option(
            lambda: destack._generated.js.tree.node.decode_local_node_id(reader)
        )

        return PatternFieldNamed(
            mutability=mutability,
            name=name,
            is_shorthand=is_shorthand,
            pattern=pattern,
        )
    elif variant == 1:
        mutability = reader.read_option(
            lambda: destack._generated.js.tree.node.decode_mutability(reader)
        )
        key = destack._generated.js.tree.node.decode_local_node_id(reader)
        pattern = destack._generated.js.tree.node.decode_local_node_id(reader)

        return PatternFieldComputed(
            mutability=mutability,
            key=key,
            pattern=pattern,
        )
    elif variant == 2:
        pattern = destack._generated.js.tree.node.decode_local_node_id(reader)

        return PatternFieldPositional(
            pattern=pattern,
        )
    elif variant == 3:
        mutability = reader.read_option(
            lambda: destack._generated.js.tree.node.decode_mutability(reader)
        )
        pattern = reader.read_option(
            lambda: destack._generated.js.tree.node.decode_local_node_id(reader)
        )

        return PatternFieldSpread(
            mutability=mutability,
            pattern=pattern,
        )
    elif variant == 4:
        return PatternFieldElision()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_pattern_field(value: PatternField) -> Json:
    """Return one JSON value for one PatternField."""
    if value.kind == "named":
        return {
            "kind": "named",
            **(
                {}
                if value.mutability is None
                else {
                    "mutability": destack._generated.js.tree.node.to_json_mutability(
                        value.mutability
                    )
                }
            ),
            "name": destack._generated.core.string.to_json_string_id(value.name),
            "isShorthand": value.is_shorthand,
            **(
                {}
                if value.pattern is None
                else {
                    "pattern": destack._generated.js.tree.node.to_json_local_node_id(
                        value.pattern
                    )
                }
            ),
        }
    elif value.kind == "computed":
        return {
            "kind": "computed",
            **(
                {}
                if value.mutability is None
                else {
                    "mutability": destack._generated.js.tree.node.to_json_mutability(
                        value.mutability
                    )
                }
            ),
            "key": destack._generated.js.tree.node.to_json_local_node_id(value.key),
            "pattern": destack._generated.js.tree.node.to_json_local_node_id(
                value.pattern
            ),
        }
    elif value.kind == "positional":
        return {
            "kind": "positional",
            "pattern": destack._generated.js.tree.node.to_json_local_node_id(
                value.pattern
            ),
        }
    elif value.kind == "spread":
        return {
            "kind": "spread",
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
                if value.pattern is None
                else {
                    "pattern": destack._generated.js.tree.node.to_json_local_node_id(
                        value.pattern
                    )
                }
            ),
        }
    elif value.kind == "elision":
        return {
            "kind": "elision",
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_pattern_field(value: Json) -> PatternField:
    """Return one PatternField from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "named":
        return PatternFieldNamed(
            mutability=json_optional(
                object_,
                "mutability",
                lambda value: destack._generated.js.tree.node.from_json_mutability(
                    value
                ),
            ),
            name=destack._generated.core.string.from_json_string_id(
                json_field(object_, "name")
            ),
            is_shorthand=json_bool(json_field(object_, "isShorthand")),
            pattern=json_optional(
                object_,
                "pattern",
                lambda value: destack._generated.js.tree.node.from_json_local_node_id(
                    value
                ),
            ),
        )
    elif kind == "computed":
        return PatternFieldComputed(
            mutability=json_optional(
                object_,
                "mutability",
                lambda value: destack._generated.js.tree.node.from_json_mutability(
                    value
                ),
            ),
            key=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "key")
            ),
            pattern=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "pattern")
            ),
        )
    elif kind == "positional":
        return PatternFieldPositional(
            pattern=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "pattern")
            ),
        )
    elif kind == "spread":
        return PatternFieldSpread(
            mutability=json_optional(
                object_,
                "mutability",
                lambda value: destack._generated.js.tree.node.from_json_mutability(
                    value
                ),
            ),
            pattern=json_optional(
                object_,
                "pattern",
                lambda value: destack._generated.js.tree.node.from_json_local_node_id(
                    value
                ),
            ),
        )
    elif kind == "elision":
        return PatternFieldElision()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class AssignPatternExpression:
    """Expression target like `x`, `obj.x`, or `obj[key]`."""

    value: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["expression"] = "expression"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_assign_pattern(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_assign_pattern(self)


@dataclass(frozen=True, slots=True)
class AssignPatternAssign:
    """Defaulted destructuring target like `x = 1`."""

    pattern: destack._generated.js.tree.node.LocalNodeId
    value: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["assign"] = "assign"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_assign_pattern(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_assign_pattern(self)


@dataclass(frozen=True, slots=True)
class AssignPatternArray:
    """Array destructuring target like `[a, , ...rest]`."""

    fields: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["array"] = "array"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_assign_pattern(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_assign_pattern(self)


@dataclass(frozen=True, slots=True)
class AssignPatternObject:
    """Object destructuring target like `{ x, y: z }`."""

    fields: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["object"] = "object"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_assign_pattern(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_assign_pattern(self)


"""An AssignPattern is one assignment left hand side."""
AssignPattern: typing.TypeAlias = (
    AssignPatternExpression
    | AssignPatternAssign
    | AssignPatternArray
    | AssignPatternObject
)


def encode_assign_pattern(writer: BinaryWriter, value: AssignPattern) -> None:
    """Encode one AssignPattern."""
    if value.kind == "expression":
        writer.write_unsigned(0)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.value)
    elif value.kind == "assign":
        writer.write_unsigned(1)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.pattern)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.value)
    elif value.kind == "array":
        writer.write_unsigned(2)
        writer.write_unsigned(len(value.fields))
        for item_value_fields_0 in value.fields:
            destack._generated.js.tree.node.encode_local_node_id(
                writer, item_value_fields_0
            )
    elif value.kind == "object":
        writer.write_unsigned(3)
        writer.write_unsigned(len(value.fields))
        for item_value_fields_0 in value.fields:
            destack._generated.js.tree.node.encode_local_node_id(
                writer, item_value_fields_0
            )
    else:
        raise SerdeError("unknown enum variant")


def decode_assign_pattern(reader: BinaryReader) -> AssignPattern:
    """Decode one AssignPattern."""
    variant = reader.read_number()

    if variant == 0:
        value_ = destack._generated.js.tree.node.decode_local_node_id(reader)

        return AssignPatternExpression(
            value=value_,
        )
    elif variant == 1:
        pattern = destack._generated.js.tree.node.decode_local_node_id(reader)
        value_ = destack._generated.js.tree.node.decode_local_node_id(reader)

        return AssignPatternAssign(
            pattern=pattern,
            value=value_,
        )
    elif variant == 2:
        fields = [
            destack._generated.js.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return AssignPatternArray(
            fields=fields,
        )
    elif variant == 3:
        fields = [
            destack._generated.js.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return AssignPatternObject(
            fields=fields,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_assign_pattern(value: AssignPattern) -> Json:
    """Return one JSON value for one AssignPattern."""
    if value.kind == "expression":
        return {
            "kind": "expression",
            "value": destack._generated.js.tree.node.to_json_local_node_id(value.value),
        }
    elif value.kind == "assign":
        return {
            "kind": "assign",
            "pattern": destack._generated.js.tree.node.to_json_local_node_id(
                value.pattern
            ),
            "value": destack._generated.js.tree.node.to_json_local_node_id(value.value),
        }
    elif value.kind == "array":
        return {
            "kind": "array",
            "fields": [
                destack._generated.js.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.fields
            ],
        }
    elif value.kind == "object":
        return {
            "kind": "object",
            "fields": [
                destack._generated.js.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.fields
            ],
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_assign_pattern(value: Json) -> AssignPattern:
    """Return one AssignPattern from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "expression":
        return AssignPatternExpression(
            value=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "value")
            ),
        )
    elif kind == "assign":
        return AssignPatternAssign(
            pattern=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "pattern")
            ),
            value=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "value")
            ),
        )
    elif kind == "array":
        return AssignPatternArray(
            fields=[
                destack._generated.js.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "fields"))
            ],
        )
    elif kind == "object":
        return AssignPatternObject(
            fields=[
                destack._generated.js.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "fields"))
            ],
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class AssignPatternFieldNamed:
    """Named field like `{ x }` or `{ x: y }`."""

    name: destack._generated.js.tree.key.Name
    is_shorthand: bool
    pattern: destack._generated.js.tree.node.LocalNodeId | None
    kind: typing.Literal["named"] = "named"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_assign_pattern_field(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_assign_pattern_field(self)


@dataclass(frozen=True, slots=True)
class AssignPatternFieldComputed:
    """Computed field like `{ [key]: value }`."""

    key: destack._generated.js.tree.node.LocalNodeId
    pattern: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["computed"] = "computed"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_assign_pattern_field(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_assign_pattern_field(self)


@dataclass(frozen=True, slots=True)
class AssignPatternFieldPositional:
    """Positional field like `[value]`."""

    pattern: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["positional"] = "positional"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_assign_pattern_field(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_assign_pattern_field(self)


@dataclass(frozen=True, slots=True)
class AssignPatternFieldSpread:
    """Spread field like `{ ...rest }` or `[...rest]`."""

    pattern: destack._generated.js.tree.node.LocalNodeId | None
    kind: typing.Literal["spread"] = "spread"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_assign_pattern_field(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_assign_pattern_field(self)


@dataclass(frozen=True, slots=True)
class AssignPatternFieldElision:
    """Elision like `[, value]`."""

    kind: typing.Literal["elision"] = "elision"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_assign_pattern_field(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_assign_pattern_field(self)


"""An AssignPatternField is one field in a destructuring assignment target."""
AssignPatternField: typing.TypeAlias = (
    AssignPatternFieldNamed
    | AssignPatternFieldComputed
    | AssignPatternFieldPositional
    | AssignPatternFieldSpread
    | AssignPatternFieldElision
)


def encode_assign_pattern_field(
    writer: BinaryWriter, value: AssignPatternField
) -> None:
    """Encode one AssignPatternField."""
    if value.kind == "named":
        writer.write_unsigned(0)
        destack._generated.js.tree.key.encode_name(writer, value.name)
        writer.write_bool(value.is_shorthand)
        if value.pattern is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.js.tree.node.encode_local_node_id(writer, value.pattern)
    elif value.kind == "computed":
        writer.write_unsigned(1)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.key)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.pattern)
    elif value.kind == "positional":
        writer.write_unsigned(2)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.pattern)
    elif value.kind == "spread":
        writer.write_unsigned(3)
        if value.pattern is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.js.tree.node.encode_local_node_id(writer, value.pattern)
    elif value.kind == "elision":
        writer.write_unsigned(4)
    else:
        raise SerdeError("unknown enum variant")


def decode_assign_pattern_field(reader: BinaryReader) -> AssignPatternField:
    """Decode one AssignPatternField."""
    variant = reader.read_number()

    if variant == 0:
        name = destack._generated.js.tree.key.decode_name(reader)
        is_shorthand = reader.read_bool()
        pattern = reader.read_option(
            lambda: destack._generated.js.tree.node.decode_local_node_id(reader)
        )

        return AssignPatternFieldNamed(
            name=name,
            is_shorthand=is_shorthand,
            pattern=pattern,
        )
    elif variant == 1:
        key = destack._generated.js.tree.node.decode_local_node_id(reader)
        pattern = destack._generated.js.tree.node.decode_local_node_id(reader)

        return AssignPatternFieldComputed(
            key=key,
            pattern=pattern,
        )
    elif variant == 2:
        pattern = destack._generated.js.tree.node.decode_local_node_id(reader)

        return AssignPatternFieldPositional(
            pattern=pattern,
        )
    elif variant == 3:
        pattern = reader.read_option(
            lambda: destack._generated.js.tree.node.decode_local_node_id(reader)
        )

        return AssignPatternFieldSpread(
            pattern=pattern,
        )
    elif variant == 4:
        return AssignPatternFieldElision()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_assign_pattern_field(value: AssignPatternField) -> Json:
    """Return one JSON value for one AssignPatternField."""
    if value.kind == "named":
        return {
            "kind": "named",
            "name": destack._generated.js.tree.key.to_json_name(value.name),
            "isShorthand": value.is_shorthand,
            **(
                {}
                if value.pattern is None
                else {
                    "pattern": destack._generated.js.tree.node.to_json_local_node_id(
                        value.pattern
                    )
                }
            ),
        }
    elif value.kind == "computed":
        return {
            "kind": "computed",
            "key": destack._generated.js.tree.node.to_json_local_node_id(value.key),
            "pattern": destack._generated.js.tree.node.to_json_local_node_id(
                value.pattern
            ),
        }
    elif value.kind == "positional":
        return {
            "kind": "positional",
            "pattern": destack._generated.js.tree.node.to_json_local_node_id(
                value.pattern
            ),
        }
    elif value.kind == "spread":
        return {
            "kind": "spread",
            **(
                {}
                if value.pattern is None
                else {
                    "pattern": destack._generated.js.tree.node.to_json_local_node_id(
                        value.pattern
                    )
                }
            ),
        }
    elif value.kind == "elision":
        return {
            "kind": "elision",
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_assign_pattern_field(value: Json) -> AssignPatternField:
    """Return one AssignPatternField from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "named":
        return AssignPatternFieldNamed(
            name=destack._generated.js.tree.key.from_json_name(
                json_field(object_, "name")
            ),
            is_shorthand=json_bool(json_field(object_, "isShorthand")),
            pattern=json_optional(
                object_,
                "pattern",
                lambda value: destack._generated.js.tree.node.from_json_local_node_id(
                    value
                ),
            ),
        )
    elif kind == "computed":
        return AssignPatternFieldComputed(
            key=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "key")
            ),
            pattern=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "pattern")
            ),
        )
    elif kind == "positional":
        return AssignPatternFieldPositional(
            pattern=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "pattern")
            ),
        )
    elif kind == "spread":
        return AssignPatternFieldSpread(
            pattern=json_optional(
                object_,
                "pattern",
                lambda value: destack._generated.js.tree.node.from_json_local_node_id(
                    value
                ),
            ),
        )
    elif kind == "elision":
        return AssignPatternFieldElision()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


__all__ = [
    "Pattern",
    "encode_pattern",
    "decode_pattern",
    "to_json_pattern",
    "from_json_pattern",
    "PatternBinding",
    "PatternAssign",
    "PatternArray",
    "PatternObject",
    "PatternHole",
    "PatternField",
    "encode_pattern_field",
    "decode_pattern_field",
    "to_json_pattern_field",
    "from_json_pattern_field",
    "PatternFieldNamed",
    "PatternFieldComputed",
    "PatternFieldPositional",
    "PatternFieldSpread",
    "PatternFieldElision",
    "AssignPattern",
    "encode_assign_pattern",
    "decode_assign_pattern",
    "to_json_assign_pattern",
    "from_json_assign_pattern",
    "AssignPatternExpression",
    "AssignPatternAssign",
    "AssignPatternArray",
    "AssignPatternObject",
    "AssignPatternField",
    "encode_assign_pattern_field",
    "decode_assign_pattern_field",
    "to_json_assign_pattern_field",
    "from_json_assign_pattern_field",
    "AssignPatternFieldNamed",
    "AssignPatternFieldComputed",
    "AssignPatternFieldPositional",
    "AssignPatternFieldSpread",
    "AssignPatternFieldElision",
]
