# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.string
import destack._generated.js.tree.key
import destack._generated.js.tree.node

@dataclass(frozen=True, slots=True)
class PatternBinding:
    """Binding pattern (like `x`)."""

    mutability: destack._generated.js.tree.node.Mutability | None
    name: destack._generated.core.string.StringId
    kind: typing.Literal["binding"] = "binding"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternAssign:
    """Assignment pattern (like `x = 1`)."""

    pattern: destack._generated.js.tree.node.LocalNodeId
    value: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["assign"] = "assign"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternArray:
    """Array pattern (like `[1, 2, .., x, 3]`)."""

    fields: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["array"] = "array"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternObject:
    """Object pattern (like `{ a: 1, b: 2, ..., x: 3 }`)."""

    fields: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["object"] = "object"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternHole:
    """Hole pattern (like the empty in `, ,`)."""

    kind: typing.Literal["hole"] = "hole"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""A Pattern is a pattern to match something and unwrap it."""
Pattern: typing.TypeAlias = (
    PatternBinding | PatternAssign | PatternArray | PatternObject | PatternHole
)

def encode_pattern(writer: BinaryWriter, value: Pattern) -> None: ...
def decode_pattern(reader: BinaryReader) -> Pattern: ...
def to_json_pattern(value: Pattern) -> Json: ...
def from_json_pattern(value: Json) -> Pattern: ...

@dataclass(frozen=True, slots=True)
class PatternFieldNamed:
    """Named pattern field (like `x` or `x: y` or `x = 4`)."""

    mutability: destack._generated.js.tree.node.Mutability | None
    name: destack._generated.core.string.StringId
    is_shorthand: bool
    pattern: destack._generated.js.tree.node.LocalNodeId | None
    kind: typing.Literal["named"] = "named"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternFieldComputed:
    """Computed pattern field (like `[key]: value`)."""

    mutability: destack._generated.js.tree.node.Mutability | None
    key: destack._generated.js.tree.node.LocalNodeId
    pattern: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["computed"] = "computed"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternFieldPositional:
    """Positional field with a pattern (like `4` or `x = 1`)."""

    pattern: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["positional"] = "positional"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternFieldSpread:
    """Spread field (like `...x` or `...[a, b]`)."""

    mutability: destack._generated.js.tree.node.Mutability | None
    pattern: destack._generated.js.tree.node.LocalNodeId | None
    kind: typing.Literal["spread"] = "spread"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternFieldElision:
    """Elision (hole) in an array pattern (like `[,a]`)."""

    kind: typing.Literal["elision"] = "elision"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""A PatternField is a field in a pattern (object, array, etc.)."""
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
class AssignPatternExpression:
    """Expression target like `x`, `obj.x`, or `obj[key]`."""

    value: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["expression"] = "expression"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class AssignPatternAssign:
    """Defaulted destructuring target like `x = 1`."""

    pattern: destack._generated.js.tree.node.LocalNodeId
    value: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["assign"] = "assign"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class AssignPatternArray:
    """Array destructuring target like `[a, , ...rest]`."""

    fields: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["array"] = "array"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class AssignPatternObject:
    """Object destructuring target like `{ x, y: z }`."""

    fields: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["object"] = "object"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""An AssignPattern is one assignment left hand side."""
AssignPattern: typing.TypeAlias = (
    AssignPatternExpression
    | AssignPatternAssign
    | AssignPatternArray
    | AssignPatternObject
)

def encode_assign_pattern(writer: BinaryWriter, value: AssignPattern) -> None: ...
def decode_assign_pattern(reader: BinaryReader) -> AssignPattern: ...
def to_json_assign_pattern(value: AssignPattern) -> Json: ...
def from_json_assign_pattern(value: Json) -> AssignPattern: ...

@dataclass(frozen=True, slots=True)
class AssignPatternFieldNamed:
    """Named field like `{ x }` or `{ x: y }`."""

    name: destack._generated.js.tree.key.Name
    is_shorthand: bool
    pattern: destack._generated.js.tree.node.LocalNodeId | None
    kind: typing.Literal["named"] = "named"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class AssignPatternFieldComputed:
    """Computed field like `{ [key]: value }`."""

    key: destack._generated.js.tree.node.LocalNodeId
    pattern: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["computed"] = "computed"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class AssignPatternFieldPositional:
    """Positional field like `[value]`."""

    pattern: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["positional"] = "positional"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class AssignPatternFieldSpread:
    """Spread field like `{ ...rest }` or `[...rest]`."""

    pattern: destack._generated.js.tree.node.LocalNodeId | None
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
