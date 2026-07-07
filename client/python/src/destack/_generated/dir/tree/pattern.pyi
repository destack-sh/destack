# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.string
import destack._generated.dir.tree.key
import destack._generated.dir.tree.node
import destack._generated.dir.tree.operator

@dataclass(frozen=True, slots=True)
class PatternWildcard:
    """Wildcard scalar pattern (`_`)."""

    kind: typing.Literal["wildcard"] = "wildcard"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternMust:
    """Must pattern (like `x!`)."""

    must: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["must"] = "must"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternDefault:
    """Defaulted pattern like `x = 1`."""

    pattern: destack._generated.dir.tree.node.LocalNodeId
    value: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["default"] = "default"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternBorrowOf:
    """Borrow pattern (like `&x`)."""

    mutability: destack._generated.dir.tree.node.Mutability | None
    right: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["borrowOf"] = "borrowOf"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternMoveOf:
    """Move pattern (like `^x`)."""

    mutability: destack._generated.dir.tree.node.Mutability | None
    right: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["moveOf"] = "moveOf"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternDereferenceOf:
    """Dereference pattern (like `*x`)."""

    right: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["dereferenceOf"] = "dereferenceOf"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternBinding:
    """Binding pattern (basically a PatternField, like `x`, `x: 4`, or `x: int32`)."""

    name: destack._generated.core.string.StringId
    pattern: destack._generated.dir.tree.node.LocalNodeId | None
    kind: typing.Literal["binding"] = "binding"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternExpression:
    """Literal value or value-space path pattern."""

    value: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["expression"] = "expression"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternRange:
    """Ordered scalar interval pattern like `0..10` or `..=255`."""

    start: destack._generated.dir.tree.node.LocalNodeId | None
    end: destack._generated.dir.tree.node.LocalNodeId | None
    end_kind: destack._generated.dir.tree.operator.RangeEnd
    kind: typing.Literal["range"] = "range"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternTuple:
    """Tuple pattern (like `(x, 0)`)."""

    fields: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["tuple"] = "tuple"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternNominalTuple:
    """Nominal tuple pattern (like `T(1)`)."""

    ty: destack._generated.dir.tree.node.LocalNodeId
    fields: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["nominalTuple"] = "nominalTuple"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternSequence:
    """Sequence pattern like `[1, 2, x]` or `[1, y, ..]`."""

    fields: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["sequence"] = "sequence"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternObject:
    """Object pattern (like `{ x, y }`)."""

    fields: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["object"] = "object"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternNominalObject:
    """Nominal object pattern (like `T { x, y }`)."""

    ty: destack._generated.dir.tree.node.LocalNodeId
    fields: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["nominalObject"] = "nominalObject"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternUnion:
    """Union pattern (like `1 | 2 | 3`)."""

    patterns: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["union"] = "union"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""A Pattern is a pattern to match something and unwrap it."""
Pattern: typing.TypeAlias = (
    PatternWildcard
    | PatternMust
    | PatternDefault
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

def encode_pattern(writer: BinaryWriter, value: Pattern) -> None: ...
def decode_pattern(reader: BinaryReader) -> Pattern: ...
def to_json_pattern(value: Pattern) -> Json: ...
def from_json_pattern(value: Json) -> Pattern: ...

@dataclass(frozen=True, slots=True)
class PatternFieldNamed:
    """Named field, maybe shorthand and maybe with a nested pattern."""

    name: destack._generated.dir.tree.key.Name
    pattern: destack._generated.dir.tree.node.LocalNodeId | None
    is_shorthand: bool
    kind: typing.Literal["named"] = "named"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternFieldComputed:
    """Computed field (like `[key]: value`)."""

    key: destack._generated.dir.tree.node.LocalNodeId
    pattern: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["computed"] = "computed"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternFieldPositional:
    """Positional field with a pattern (like `4` or `x = 1`)."""

    pattern: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["positional"] = "positional"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternFieldSpread:
    """Spread field (like `...x` or `...[a, b]`)."""

    pattern: destack._generated.dir.tree.node.LocalNodeId | None
    kind: typing.Literal["spread"] = "spread"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternFieldElision:
    """Elision in a sequence pattern like `[,a]` or `[,,b]`."""

    kind: typing.Literal["elision"] = "elision"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""A PatternField is a field of a variant pattern."""
PatternField: typing.TypeAlias = (
    PatternFieldNamed
    | PatternFieldComputed
    | PatternFieldPositional
    | PatternFieldSpread
    | PatternFieldElision
)

def encode_pattern_field(writer: BinaryWriter, value: PatternField) -> None: ...
def decode_pattern_field(reader: BinaryReader) -> PatternField: ...
def to_json_pattern_field(value: PatternField) -> Json: ...
def from_json_pattern_field(value: Json) -> PatternField: ...

@dataclass(frozen=True, slots=True)
class AssignPatternPlace:
    """Writable place target like `x`, `obj.x`, or `obj[key]`."""

    expression: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["place"] = "place"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class AssignPatternDefault:
    """Defaulted destructuring target like `x = 1`."""

    pattern: destack._generated.dir.tree.node.LocalNodeId
    value: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["default"] = "default"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class AssignPatternSequence:
    """Sequence destructuring target like `[a, , ...rest]`."""

    fields: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["sequence"] = "sequence"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class AssignPatternTuple:
    """Tuple destructuring target like `(a, b)` or `(a,)`."""

    fields: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["tuple"] = "tuple"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class AssignPatternObject:
    """Object destructuring target like `{ x, y: z }`."""

    fields: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["object"] = "object"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""An AssignPattern is one assignment left hand side."""
AssignPattern: typing.TypeAlias = (
    AssignPatternPlace
    | AssignPatternDefault
    | AssignPatternSequence
    | AssignPatternTuple
    | AssignPatternObject
)

def encode_assign_pattern(writer: BinaryWriter, value: AssignPattern) -> None: ...
def decode_assign_pattern(reader: BinaryReader) -> AssignPattern: ...
def to_json_assign_pattern(value: AssignPattern) -> Json: ...
def from_json_assign_pattern(value: Json) -> AssignPattern: ...

@dataclass(frozen=True, slots=True)
class AssignPatternFieldNamed:
    """Named field like `{ x }` or `{ x: y }`."""

    name: destack._generated.dir.tree.key.Name
    pattern: destack._generated.dir.tree.node.LocalNodeId
    is_shorthand: bool
    kind: typing.Literal["named"] = "named"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class AssignPatternFieldComputed:
    """Computed field like `{ [key]: value }`."""

    key: destack._generated.dir.tree.node.LocalNodeId
    pattern: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["computed"] = "computed"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class AssignPatternFieldPositional:
    """Positional field like `[value]`."""

    pattern: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["positional"] = "positional"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class AssignPatternFieldSpread:
    """Spread field like `{ ...rest }` or `[...rest]`."""

    pattern: destack._generated.dir.tree.node.LocalNodeId | None
    kind: typing.Literal["spread"] = "spread"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class AssignPatternFieldElision:
    """Elision like `[, value]`."""

    kind: typing.Literal["elision"] = "elision"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

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
) -> None: ...
def decode_assign_pattern_field(reader: BinaryReader) -> AssignPatternField: ...
def to_json_assign_pattern_field(value: AssignPatternField) -> Json: ...
def from_json_assign_pattern_field(value: Json) -> AssignPatternField: ...

__all__ = [
    "Pattern",
    "encode_pattern",
    "decode_pattern",
    "to_json_pattern",
    "from_json_pattern",
    "PatternWildcard",
    "PatternMust",
    "PatternDefault",
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
    "AssignPatternPlace",
    "AssignPatternDefault",
    "AssignPatternSequence",
    "AssignPatternTuple",
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
