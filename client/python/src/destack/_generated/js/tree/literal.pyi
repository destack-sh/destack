# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.string
import destack._generated.js.tree.node
import destack._generated.js.tree.path

@dataclass(frozen=True, slots=True)
class ScalarLiteralNull:
    """Null value."""

    kind: typing.Literal["null"] = "null"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ScalarLiteralUndefined:
    """Undefined value."""

    kind: typing.Literal["undefined"] = "undefined"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ScalarLiteralBoolean:
    """Boolean value."""

    boolean: bool
    kind: typing.Literal["boolean"] = "boolean"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ScalarLiteralNumber:
    """Number value."""

    number: float
    kind: typing.Literal["number"] = "number"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ScalarLiteralBigint:
    """Bigint value."""

    bigint: int
    kind: typing.Literal["bigint"] = "bigint"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ScalarLiteralString:
    """String value."""

    string: destack._generated.core.string.StringId
    kind: typing.Literal["string"] = "string"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ScalarLiteralRegexString:
    """Regex string value."""

    content: destack._generated.core.string.StringId
    flags: destack._generated.core.string.StringId | None
    kind: typing.Literal["regexString"] = "regexString"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

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

def encode_scalar_literal(writer: BinaryWriter, value: ScalarLiteral) -> None: ...
def decode_scalar_literal(reader: BinaryReader) -> ScalarLiteral: ...
def to_json_scalar_literal(value: ScalarLiteral) -> Json: ...
def from_json_scalar_literal(value: Json) -> ScalarLiteral: ...

@dataclass(frozen=True, slots=True)
class TemplateLiteralString:
    """Template string value."""

    template: destack._generated.core.string.StringId
    kind: typing.Literal["string"] = "string"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TemplateLiteralTaggedString:
    """Tagged template literal value."""

    tag: destack._generated.js.tree.path.Path
    template: destack._generated.core.string.StringId
    kind: typing.Literal["taggedString"] = "taggedString"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TemplateLiteralInterpolatedString:
    """Interpolated template literal value."""

    template: Sequence[destack._generated.core.string.StringId]
    expressions: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["interpolatedString"] = "interpolatedString"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TemplateLiteralTaggedInterpolatedString:
    """Tagged interpolated template literal value."""

    tag: destack._generated.js.tree.path.Path
    template: Sequence[destack._generated.core.string.StringId]
    expressions: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["taggedInterpolatedString"] = "taggedInterpolatedString"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""A TemplateLiteral is literal template value."""
TemplateLiteral: typing.TypeAlias = (
    TemplateLiteralString
    | TemplateLiteralTaggedString
    | TemplateLiteralInterpolatedString
    | TemplateLiteralTaggedInterpolatedString
)

def encode_template_literal(writer: BinaryWriter, value: TemplateLiteral) -> None: ...
def decode_template_literal(reader: BinaryReader) -> TemplateLiteral: ...
def to_json_template_literal(value: TemplateLiteral) -> Json: ...
def from_json_template_literal(value: Json) -> TemplateLiteral: ...

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
