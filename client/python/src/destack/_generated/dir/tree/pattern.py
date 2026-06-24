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
import destack._generated.dir.tree.key
import destack._generated.dir.tree.node
import destack._generated.dir.tree.operator


@dataclass(frozen=True, slots=True)
class PatternWildcard:
    """Wildcard scalar pattern (`_`)."""

    kind: typing.Literal["wildcard"] = "wildcard"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern(self)


@dataclass(frozen=True, slots=True)
class PatternMust:
    """Must pattern (like `x!`)."""

    must: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["must"] = "must"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern(self)


@dataclass(frozen=True, slots=True)
class PatternAssign:
    """Assignment pattern (like `x = 1` or `{ x } = {}`)."""

    pattern: destack._generated.dir.tree.node.LocalNodeId
    value: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["assign"] = "assign"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern(self)


@dataclass(frozen=True, slots=True)
class PatternBorrowOf:
    """Borrow pattern (like `&x`)."""

    mutability: destack._generated.dir.tree.node.Mutability | None
    right: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["borrowOf"] = "borrowOf"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern(self)


@dataclass(frozen=True, slots=True)
class PatternMoveOf:
    """Move pattern (like `^x`)."""

    mutability: destack._generated.dir.tree.node.Mutability | None
    right: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["moveOf"] = "moveOf"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern(self)


@dataclass(frozen=True, slots=True)
class PatternDereferenceOf:
    """Dereference pattern (like `*x`)."""

    right: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["dereferenceOf"] = "dereferenceOf"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern(self)


@dataclass(frozen=True, slots=True)
class PatternBinding:
    """Binding pattern (basically a PatternField, like `x`, `x: 4`, or `x: int32`)."""

    name: destack._generated.core.string.StringId
    pattern: destack._generated.dir.tree.node.LocalNodeId | None
    kind: typing.Literal["binding"] = "binding"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern(self)


@dataclass(frozen=True, slots=True)
class PatternExpression:
    """Literal value or value-space path pattern."""

    value: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["expression"] = "expression"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern(self)


@dataclass(frozen=True, slots=True)
class PatternRange:
    """Ordered scalar interval pattern like `0..10` or `..=255`."""

    start: destack._generated.dir.tree.node.LocalNodeId | None
    end: destack._generated.dir.tree.node.LocalNodeId | None
    end_kind: destack._generated.dir.tree.operator.RangeEnd
    kind: typing.Literal["range"] = "range"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern(self)


@dataclass(frozen=True, slots=True)
class PatternTuple:
    """Tuple pattern (like `(x, 0)`)."""

    fields: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["tuple"] = "tuple"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern(self)


@dataclass(frozen=True, slots=True)
class PatternNominalTuple:
    """Nominal tuple pattern (like `T(1)`)."""

    ty: destack._generated.dir.tree.node.LocalNodeId
    fields: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["nominalTuple"] = "nominalTuple"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern(self)


@dataclass(frozen=True, slots=True)
class PatternSequence:
    """Sequence pattern like `[1, 2, x]` or `[1, y, ..]`."""

    fields: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["sequence"] = "sequence"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern(self)


@dataclass(frozen=True, slots=True)
class PatternObject:
    """Object pattern (like `{ x, y }`)."""

    fields: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["object"] = "object"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern(self)


@dataclass(frozen=True, slots=True)
class PatternNominalObject:
    """Nominal object pattern (like `T { x, y }`)."""

    ty: destack._generated.dir.tree.node.LocalNodeId
    fields: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["nominalObject"] = "nominalObject"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern(self)


@dataclass(frozen=True, slots=True)
class PatternUnion:
    """Union pattern (like `1 | 2 | 3`)."""

    patterns: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["union"] = "union"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern(self)


"""A Pattern is a pattern to match something and unwrap it."""
Pattern: typing.TypeAlias = (
    PatternWildcard
    | PatternMust
    | PatternAssign
    | PatternBorrowOf
    | PatternMoveOf
    | PatternDereferenceOf
    | PatternBinding
    | PatternExpression
    | PatternRange
    | PatternTuple
    | PatternNominalTuple
    | PatternSequence
    | PatternObject
    | PatternNominalObject
    | PatternUnion
)


def encode_pattern(writer: BinaryWriter, value: Pattern) -> None:
    """Encode one Pattern."""
    if value.kind == "wildcard":
        writer.write_unsigned(0)
    elif value.kind == "must":
        writer.write_unsigned(1)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.must)
    elif value.kind == "assign":
        writer.write_unsigned(2)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.pattern)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.value)
    elif value.kind == "borrowOf":
        writer.write_unsigned(3)
        if value.mutability is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_mutability(writer, value.mutability)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.right)
    elif value.kind == "moveOf":
        writer.write_unsigned(4)
        if value.mutability is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_mutability(writer, value.mutability)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.right)
    elif value.kind == "dereferenceOf":
        writer.write_unsigned(5)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.right)
    elif value.kind == "binding":
        writer.write_unsigned(6)
        destack._generated.core.string.encode_string_id(writer, value.name)
        if value.pattern is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_local_node_id(writer, value.pattern)
    elif value.kind == "expression":
        writer.write_unsigned(7)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.value)
    elif value.kind == "range":
        writer.write_unsigned(8)
        if value.start is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_local_node_id(writer, value.start)
        if value.end is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_local_node_id(writer, value.end)
        destack._generated.dir.tree.operator.encode_range_end(writer, value.end_kind)
    elif value.kind == "tuple":
        writer.write_unsigned(9)
        writer.write_unsigned(len(value.fields))
        for item_value_fields_0 in value.fields:
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, item_value_fields_0
            )
    elif value.kind == "nominalTuple":
        writer.write_unsigned(10)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.ty)
        writer.write_unsigned(len(value.fields))
        for item_value_fields_0 in value.fields:
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, item_value_fields_0
            )
    elif value.kind == "sequence":
        writer.write_unsigned(11)
        writer.write_unsigned(len(value.fields))
        for item_value_fields_0 in value.fields:
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, item_value_fields_0
            )
    elif value.kind == "object":
        writer.write_unsigned(12)
        writer.write_unsigned(len(value.fields))
        for item_value_fields_0 in value.fields:
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, item_value_fields_0
            )
    elif value.kind == "nominalObject":
        writer.write_unsigned(13)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.ty)
        writer.write_unsigned(len(value.fields))
        for item_value_fields_0 in value.fields:
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, item_value_fields_0
            )
    elif value.kind == "union":
        writer.write_unsigned(14)
        writer.write_unsigned(len(value.patterns))
        for item_value_patterns_0 in value.patterns:
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, item_value_patterns_0
            )
    else:
        raise SerdeError("unknown enum variant")


def decode_pattern(reader: BinaryReader) -> Pattern:
    """Decode one Pattern."""
    variant = reader.read_number()

    if variant == 0:
        return PatternWildcard()
    elif variant == 1:
        must = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return PatternMust(must=must)
    elif variant == 2:
        pattern = destack._generated.dir.tree.node.decode_local_node_id(reader)
        value_ = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return PatternAssign(
            pattern=pattern,
            value=value_,
        )
    elif variant == 3:
        mutability = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_mutability(reader)
        )
        right = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return PatternBorrowOf(
            mutability=mutability,
            right=right,
        )
    elif variant == 4:
        mutability = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_mutability(reader)
        )
        right = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return PatternMoveOf(
            mutability=mutability,
            right=right,
        )
    elif variant == 5:
        right = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return PatternDereferenceOf(
            right=right,
        )
    elif variant == 6:
        name = destack._generated.core.string.decode_string_id(reader)
        pattern = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )

        return PatternBinding(
            name=name,
            pattern=pattern,
        )
    elif variant == 7:
        value_ = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return PatternExpression(
            value=value_,
        )
    elif variant == 8:
        start = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )
        end = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )
        end_kind = destack._generated.dir.tree.operator.decode_range_end(reader)

        return PatternRange(
            start=start,
            end=end,
            end_kind=end_kind,
        )
    elif variant == 9:
        fields = [
            destack._generated.dir.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return PatternTuple(
            fields=fields,
        )
    elif variant == 10:
        ty = destack._generated.dir.tree.node.decode_local_node_id(reader)
        fields = [
            destack._generated.dir.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return PatternNominalTuple(
            ty=ty,
            fields=fields,
        )
    elif variant == 11:
        fields = [
            destack._generated.dir.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return PatternSequence(
            fields=fields,
        )
    elif variant == 12:
        fields = [
            destack._generated.dir.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return PatternObject(
            fields=fields,
        )
    elif variant == 13:
        ty = destack._generated.dir.tree.node.decode_local_node_id(reader)
        fields = [
            destack._generated.dir.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return PatternNominalObject(
            ty=ty,
            fields=fields,
        )
    elif variant == 14:
        patterns = [
            destack._generated.dir.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return PatternUnion(
            patterns=patterns,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_pattern(value: Pattern) -> Json:
    """Return one JSON value for one Pattern."""
    if value.kind == "wildcard":
        return {
            "kind": "wildcard",
        }
    elif value.kind == "must":
        return {
            "kind": "must",
            "must": destack._generated.dir.tree.node.to_json_local_node_id(value.must),
        }
    elif value.kind == "assign":
        return {
            "kind": "assign",
            "pattern": destack._generated.dir.tree.node.to_json_local_node_id(
                value.pattern
            ),
            "value": destack._generated.dir.tree.node.to_json_local_node_id(
                value.value
            ),
        }
    elif value.kind == "borrowOf":
        return {
            "kind": "borrowOf",
            **(
                {}
                if value.mutability is None
                else {
                    "mutability": destack._generated.dir.tree.node.to_json_mutability(
                        value.mutability
                    )
                }
            ),
            "right": destack._generated.dir.tree.node.to_json_local_node_id(
                value.right
            ),
        }
    elif value.kind == "moveOf":
        return {
            "kind": "moveOf",
            **(
                {}
                if value.mutability is None
                else {
                    "mutability": destack._generated.dir.tree.node.to_json_mutability(
                        value.mutability
                    )
                }
            ),
            "right": destack._generated.dir.tree.node.to_json_local_node_id(
                value.right
            ),
        }
    elif value.kind == "dereferenceOf":
        return {
            "kind": "dereferenceOf",
            "right": destack._generated.dir.tree.node.to_json_local_node_id(
                value.right
            ),
        }
    elif value.kind == "binding":
        return {
            "kind": "binding",
            "name": destack._generated.core.string.to_json_string_id(value.name),
            **(
                {}
                if value.pattern is None
                else {
                    "pattern": destack._generated.dir.tree.node.to_json_local_node_id(
                        value.pattern
                    )
                }
            ),
        }
    elif value.kind == "expression":
        return {
            "kind": "expression",
            "value": destack._generated.dir.tree.node.to_json_local_node_id(
                value.value
            ),
        }
    elif value.kind == "range":
        return {
            "kind": "range",
            **(
                {}
                if value.start is None
                else {
                    "start": destack._generated.dir.tree.node.to_json_local_node_id(
                        value.start
                    )
                }
            ),
            **(
                {}
                if value.end is None
                else {
                    "end": destack._generated.dir.tree.node.to_json_local_node_id(
                        value.end
                    )
                }
            ),
            "endKind": destack._generated.dir.tree.operator.to_json_range_end(
                value.end_kind
            ),
        }
    elif value.kind == "tuple":
        return {
            "kind": "tuple",
            "fields": [
                destack._generated.dir.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.fields
            ],
        }
    elif value.kind == "nominalTuple":
        return {
            "kind": "nominalTuple",
            "ty": destack._generated.dir.tree.node.to_json_local_node_id(value.ty),
            "fields": [
                destack._generated.dir.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.fields
            ],
        }
    elif value.kind == "sequence":
        return {
            "kind": "sequence",
            "fields": [
                destack._generated.dir.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.fields
            ],
        }
    elif value.kind == "object":
        return {
            "kind": "object",
            "fields": [
                destack._generated.dir.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.fields
            ],
        }
    elif value.kind == "nominalObject":
        return {
            "kind": "nominalObject",
            "ty": destack._generated.dir.tree.node.to_json_local_node_id(value.ty),
            "fields": [
                destack._generated.dir.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.fields
            ],
        }
    elif value.kind == "union":
        return {
            "kind": "union",
            "patterns": [
                destack._generated.dir.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.patterns
            ],
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_pattern(value: Json) -> Pattern:
    """Return one Pattern from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "wildcard":
        return PatternWildcard()
    elif kind == "must":
        return PatternMust(
            must=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "must")
            )
        )
    elif kind == "assign":
        return PatternAssign(
            pattern=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "pattern")
            ),
            value=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "value")
            ),
        )
    elif kind == "borrowOf":
        return PatternBorrowOf(
            mutability=json_optional(
                object_,
                "mutability",
                lambda value: destack._generated.dir.tree.node.from_json_mutability(
                    value
                ),
            ),
            right=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "right")
            ),
        )
    elif kind == "moveOf":
        return PatternMoveOf(
            mutability=json_optional(
                object_,
                "mutability",
                lambda value: destack._generated.dir.tree.node.from_json_mutability(
                    value
                ),
            ),
            right=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "right")
            ),
        )
    elif kind == "dereferenceOf":
        return PatternDereferenceOf(
            right=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "right")
            ),
        )
    elif kind == "binding":
        return PatternBinding(
            name=destack._generated.core.string.from_json_string_id(
                json_field(object_, "name")
            ),
            pattern=json_optional(
                object_,
                "pattern",
                lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                    value
                ),
            ),
        )
    elif kind == "expression":
        return PatternExpression(
            value=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "value")
            ),
        )
    elif kind == "range":
        return PatternRange(
            start=json_optional(
                object_,
                "start",
                lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                    value
                ),
            ),
            end=json_optional(
                object_,
                "end",
                lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                    value
                ),
            ),
            end_kind=destack._generated.dir.tree.operator.from_json_range_end(
                json_field(object_, "endKind")
            ),
        )
    elif kind == "tuple":
        return PatternTuple(
            fields=[
                destack._generated.dir.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "fields"))
            ],
        )
    elif kind == "nominalTuple":
        return PatternNominalTuple(
            ty=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "ty")
            ),
            fields=[
                destack._generated.dir.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "fields"))
            ],
        )
    elif kind == "sequence":
        return PatternSequence(
            fields=[
                destack._generated.dir.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "fields"))
            ],
        )
    elif kind == "object":
        return PatternObject(
            fields=[
                destack._generated.dir.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "fields"))
            ],
        )
    elif kind == "nominalObject":
        return PatternNominalObject(
            ty=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "ty")
            ),
            fields=[
                destack._generated.dir.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "fields"))
            ],
        )
    elif kind == "union":
        return PatternUnion(
            patterns=[
                destack._generated.dir.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "patterns"))
            ],
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class PatternFieldNamed:
    """Named field, maybe shorthand and maybe with a nested pattern."""

    name: destack._generated.dir.tree.key.Name
    pattern: destack._generated.dir.tree.node.LocalNodeId | None
    is_shorthand: bool
    kind: typing.Literal["named"] = "named"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_field(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_field(self)


@dataclass(frozen=True, slots=True)
class PatternFieldComputed:
    """Computed field (like `[key]: value`)."""

    key: destack._generated.dir.tree.node.LocalNodeId
    pattern: destack._generated.dir.tree.node.LocalNodeId
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

    pattern: destack._generated.dir.tree.node.LocalNodeId
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

    pattern: destack._generated.dir.tree.node.LocalNodeId | None
    kind: typing.Literal["spread"] = "spread"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_field(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_field(self)


@dataclass(frozen=True, slots=True)
class PatternFieldElision:
    """Elision in a sequence pattern like `[,a]` or `[,,b]`."""

    kind: typing.Literal["elision"] = "elision"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_field(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_field(self)


"""A PatternField is a field of a variant pattern."""
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
        destack._generated.dir.tree.key.encode_name(writer, value.name)
        if value.pattern is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_local_node_id(writer, value.pattern)
        writer.write_bool(value.is_shorthand)
    elif value.kind == "computed":
        writer.write_unsigned(1)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.key)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.pattern)
    elif value.kind == "positional":
        writer.write_unsigned(2)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.pattern)
    elif value.kind == "spread":
        writer.write_unsigned(3)
        if value.pattern is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_local_node_id(writer, value.pattern)
    elif value.kind == "elision":
        writer.write_unsigned(4)
    else:
        raise SerdeError("unknown enum variant")


def decode_pattern_field(reader: BinaryReader) -> PatternField:
    """Decode one PatternField."""
    variant = reader.read_number()

    if variant == 0:
        name = destack._generated.dir.tree.key.decode_name(reader)
        pattern = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )
        is_shorthand = reader.read_bool()

        return PatternFieldNamed(
            name=name,
            pattern=pattern,
            is_shorthand=is_shorthand,
        )
    elif variant == 1:
        key = destack._generated.dir.tree.node.decode_local_node_id(reader)
        pattern = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return PatternFieldComputed(
            key=key,
            pattern=pattern,
        )
    elif variant == 2:
        pattern = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return PatternFieldPositional(
            pattern=pattern,
        )
    elif variant == 3:
        pattern = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )

        return PatternFieldSpread(
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
            "name": destack._generated.dir.tree.key.to_json_name(value.name),
            **(
                {}
                if value.pattern is None
                else {
                    "pattern": destack._generated.dir.tree.node.to_json_local_node_id(
                        value.pattern
                    )
                }
            ),
            "isShorthand": value.is_shorthand,
        }
    elif value.kind == "computed":
        return {
            "kind": "computed",
            "key": destack._generated.dir.tree.node.to_json_local_node_id(value.key),
            "pattern": destack._generated.dir.tree.node.to_json_local_node_id(
                value.pattern
            ),
        }
    elif value.kind == "positional":
        return {
            "kind": "positional",
            "pattern": destack._generated.dir.tree.node.to_json_local_node_id(
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
                    "pattern": destack._generated.dir.tree.node.to_json_local_node_id(
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
            name=destack._generated.dir.tree.key.from_json_name(
                json_field(object_, "name")
            ),
            pattern=json_optional(
                object_,
                "pattern",
                lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                    value
                ),
            ),
            is_shorthand=json_bool(json_field(object_, "isShorthand")),
        )
    elif kind == "computed":
        return PatternFieldComputed(
            key=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "key")
            ),
            pattern=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "pattern")
            ),
        )
    elif kind == "positional":
        return PatternFieldPositional(
            pattern=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "pattern")
            ),
        )
    elif kind == "spread":
        return PatternFieldSpread(
            pattern=json_optional(
                object_,
                "pattern",
                lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
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

    value: destack._generated.dir.tree.node.LocalNodeId
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

    pattern: destack._generated.dir.tree.node.LocalNodeId
    value: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["assign"] = "assign"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_assign_pattern(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_assign_pattern(self)


@dataclass(frozen=True, slots=True)
class AssignPatternSequence:
    """Sequence destructuring target like `[a, , ...rest]`."""

    fields: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["sequence"] = "sequence"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_assign_pattern(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_assign_pattern(self)


@dataclass(frozen=True, slots=True)
class AssignPatternObject:
    """Object destructuring target like `{ x, y: z }`."""

    fields: Sequence[destack._generated.dir.tree.node.LocalNodeId]
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
    | AssignPatternSequence
    | AssignPatternObject
)


def encode_assign_pattern(writer: BinaryWriter, value: AssignPattern) -> None:
    """Encode one AssignPattern."""
    if value.kind == "expression":
        writer.write_unsigned(0)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.value)
    elif value.kind == "assign":
        writer.write_unsigned(1)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.pattern)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.value)
    elif value.kind == "sequence":
        writer.write_unsigned(2)
        writer.write_unsigned(len(value.fields))
        for item_value_fields_0 in value.fields:
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, item_value_fields_0
            )
    elif value.kind == "object":
        writer.write_unsigned(3)
        writer.write_unsigned(len(value.fields))
        for item_value_fields_0 in value.fields:
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, item_value_fields_0
            )
    else:
        raise SerdeError("unknown enum variant")


def decode_assign_pattern(reader: BinaryReader) -> AssignPattern:
    """Decode one AssignPattern."""
    variant = reader.read_number()

    if variant == 0:
        value_ = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return AssignPatternExpression(
            value=value_,
        )
    elif variant == 1:
        pattern = destack._generated.dir.tree.node.decode_local_node_id(reader)
        value_ = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return AssignPatternAssign(
            pattern=pattern,
            value=value_,
        )
    elif variant == 2:
        fields = [
            destack._generated.dir.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return AssignPatternSequence(
            fields=fields,
        )
    elif variant == 3:
        fields = [
            destack._generated.dir.tree.node.decode_local_node_id(reader)
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
            "value": destack._generated.dir.tree.node.to_json_local_node_id(
                value.value
            ),
        }
    elif value.kind == "assign":
        return {
            "kind": "assign",
            "pattern": destack._generated.dir.tree.node.to_json_local_node_id(
                value.pattern
            ),
            "value": destack._generated.dir.tree.node.to_json_local_node_id(
                value.value
            ),
        }
    elif value.kind == "sequence":
        return {
            "kind": "sequence",
            "fields": [
                destack._generated.dir.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.fields
            ],
        }
    elif value.kind == "object":
        return {
            "kind": "object",
            "fields": [
                destack._generated.dir.tree.node.to_json_local_node_id(item_0)
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
            value=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "value")
            ),
        )
    elif kind == "assign":
        return AssignPatternAssign(
            pattern=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "pattern")
            ),
            value=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "value")
            ),
        )
    elif kind == "sequence":
        return AssignPatternSequence(
            fields=[
                destack._generated.dir.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "fields"))
            ],
        )
    elif kind == "object":
        return AssignPatternObject(
            fields=[
                destack._generated.dir.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "fields"))
            ],
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class AssignPatternFieldNamed:
    """Named field like `{ x }` or `{ x: y }`."""

    name: destack._generated.dir.tree.key.Name
    pattern: destack._generated.dir.tree.node.LocalNodeId | None
    is_shorthand: bool
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

    key: destack._generated.dir.tree.node.LocalNodeId
    pattern: destack._generated.dir.tree.node.LocalNodeId
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

    pattern: destack._generated.dir.tree.node.LocalNodeId
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

    pattern: destack._generated.dir.tree.node.LocalNodeId | None
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
        destack._generated.dir.tree.key.encode_name(writer, value.name)
        if value.pattern is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_local_node_id(writer, value.pattern)
        writer.write_bool(value.is_shorthand)
    elif value.kind == "computed":
        writer.write_unsigned(1)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.key)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.pattern)
    elif value.kind == "positional":
        writer.write_unsigned(2)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.pattern)
    elif value.kind == "spread":
        writer.write_unsigned(3)
        if value.pattern is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_local_node_id(writer, value.pattern)
    elif value.kind == "elision":
        writer.write_unsigned(4)
    else:
        raise SerdeError("unknown enum variant")


def decode_assign_pattern_field(reader: BinaryReader) -> AssignPatternField:
    """Decode one AssignPatternField."""
    variant = reader.read_number()

    if variant == 0:
        name = destack._generated.dir.tree.key.decode_name(reader)
        pattern = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )
        is_shorthand = reader.read_bool()

        return AssignPatternFieldNamed(
            name=name,
            pattern=pattern,
            is_shorthand=is_shorthand,
        )
    elif variant == 1:
        key = destack._generated.dir.tree.node.decode_local_node_id(reader)
        pattern = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return AssignPatternFieldComputed(
            key=key,
            pattern=pattern,
        )
    elif variant == 2:
        pattern = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return AssignPatternFieldPositional(
            pattern=pattern,
        )
    elif variant == 3:
        pattern = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
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
            "name": destack._generated.dir.tree.key.to_json_name(value.name),
            **(
                {}
                if value.pattern is None
                else {
                    "pattern": destack._generated.dir.tree.node.to_json_local_node_id(
                        value.pattern
                    )
                }
            ),
            "isShorthand": value.is_shorthand,
        }
    elif value.kind == "computed":
        return {
            "kind": "computed",
            "key": destack._generated.dir.tree.node.to_json_local_node_id(value.key),
            "pattern": destack._generated.dir.tree.node.to_json_local_node_id(
                value.pattern
            ),
        }
    elif value.kind == "positional":
        return {
            "kind": "positional",
            "pattern": destack._generated.dir.tree.node.to_json_local_node_id(
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
                    "pattern": destack._generated.dir.tree.node.to_json_local_node_id(
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
            name=destack._generated.dir.tree.key.from_json_name(
                json_field(object_, "name")
            ),
            pattern=json_optional(
                object_,
                "pattern",
                lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                    value
                ),
            ),
            is_shorthand=json_bool(json_field(object_, "isShorthand")),
        )
    elif kind == "computed":
        return AssignPatternFieldComputed(
            key=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "key")
            ),
            pattern=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "pattern")
            ),
        )
    elif kind == "positional":
        return AssignPatternFieldPositional(
            pattern=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "pattern")
            ),
        )
    elif kind == "spread":
        return AssignPatternFieldSpread(
            pattern=json_optional(
                object_,
                "pattern",
                lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
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
    "PatternWildcard",
    "PatternMust",
    "PatternAssign",
    "PatternBorrowOf",
    "PatternMoveOf",
    "PatternDereferenceOf",
    "PatternBinding",
    "PatternExpression",
    "PatternRange",
    "PatternTuple",
    "PatternNominalTuple",
    "PatternSequence",
    "PatternObject",
    "PatternNominalObject",
    "PatternUnion",
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
    "AssignPatternSequence",
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
