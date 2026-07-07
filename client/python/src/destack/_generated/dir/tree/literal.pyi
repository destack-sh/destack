# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.string
import destack._generated.dir.tree.key
import destack._generated.dir.tree.node
import destack._generated.dir.type.primitive

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
class ScalarLiteralInteger:
    """Integer value."""

    integer: int
    kind: typing.Literal["integer"] = "integer"

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
class ScalarLiteralFloat:
    """Float value."""

    float: float
    kind: typing.Literal["float"] = "float"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ScalarLiteralCharacter:
    """Character value."""

    character: str
    kind: typing.Literal["character"] = "character"

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
    | ScalarLiteralInteger
    | ScalarLiteralBigint
    | ScalarLiteralFloat
    | ScalarLiteralCharacter
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

    string: destack._generated.core.string.StringId
    kind: typing.Literal["string"] = "string"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TemplateLiteralInterpolatedString:
    """Interpolated template literal value."""

    strings: Sequence[destack._generated.core.string.StringId]
    arguments: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["interpolatedString"] = "interpolatedString"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""A TemplateLiteral is literal template value."""
TemplateLiteral: typing.TypeAlias = (
    TemplateLiteralString | TemplateLiteralInterpolatedString
)

def encode_template_literal(writer: BinaryWriter, value: TemplateLiteral) -> None: ...
def decode_template_literal(reader: BinaryReader) -> TemplateLiteral: ...
def to_json_template_literal(value: TemplateLiteral) -> Json: ...
def from_json_template_literal(value: Json) -> TemplateLiteral: ...

@dataclass(frozen=True, slots=True)
class TypeLiteralNever:
    """Never type `never`."""

    kind: typing.Literal["never"] = "never"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeLiteralAny:
    """Any type `any`."""

    kind: typing.Literal["any"] = "any"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeLiteralUndefined:
    """Uninitialized type and value."""

    kind: typing.Literal["undefined"] = "undefined"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeLiteralUnknown:
    """Unknown type."""

    kind: typing.Literal["unknown"] = "unknown"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeLiteralObject:
    """Object type (any non-primitive)."""

    kind: typing.Literal["object"] = "object"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeLiteralVoid:
    """Void type."""

    kind: typing.Literal["void"] = "void"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeLiteralNull:
    """Null type and value."""

    kind: typing.Literal["null"] = "null"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeLiteralBoolean:
    """Boolean type."""

    kind: typing.Literal["boolean"] = "boolean"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeLiteralCharacter:
    """Character type."""

    kind: typing.Literal["character"] = "character"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeLiteralString:
    """String type (unsized)."""

    kind: typing.Literal["string"] = "string"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeLiteralBigint:
    """Bigint type (unsized)."""

    kind: typing.Literal["bigint"] = "bigint"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeLiteralNumber:
    """`number`, the `float64` source alias."""

    kind: typing.Literal["number"] = "number"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeLiteralAlias:
    """A widthless source alias for a sized scalar, like `int` for `int64`."""

    alias: destack._generated.dir.type.primitive.ScalarAlias
    kind: typing.Literal["alias"] = "alias"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeLiteralInteger:
    """Width-spelled integer type, like `int32` or `usize`."""

    integer: destack._generated.dir.type.primitive.IntegerType
    kind: typing.Literal["integer"] = "integer"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeLiteralFloat:
    """Width-spelled floating-point type, like `float32`."""

    float: destack._generated.dir.type.primitive.FloatType
    kind: typing.Literal["float"] = "float"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeLiteralSymbol:
    """Symbol type."""

    kind: typing.Literal["symbol"] = "symbol"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeLiteralUniqueSymbol:
    """Unique symbol type."""

    kind: typing.Literal["uniqueSymbol"] = "uniqueSymbol"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

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

def encode_type_literal(writer: BinaryWriter, value: TypeLiteral) -> None: ...
def decode_type_literal(reader: BinaryReader) -> TypeLiteral: ...
def to_json_type_literal(value: TypeLiteral) -> Json: ...
def from_json_type_literal(value: Json) -> TypeLiteral: ...

@dataclass(frozen=True, slots=True)
class TreeAttributeNamed:
    """Named attribute with an optional value."""

    name: destack._generated.dir.tree.key.Name
    value: TreeAttributeValue | None
    kind: typing.Literal["named"] = "named"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TreeAttributeSpread:
    """Spread attribute."""

    value: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["spread"] = "spread"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TreeAttributeError:
    """Malformed attribute slot."""

    kind: typing.Literal["error"] = "error"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""A tree tag attribute."""
TreeAttribute: typing.TypeAlias = (
    TreeAttributeNamed | TreeAttributeSpread | TreeAttributeError
)

def encode_tree_attribute(writer: BinaryWriter, value: TreeAttribute) -> None: ...
def decode_tree_attribute(reader: BinaryReader) -> TreeAttribute: ...
def to_json_tree_attribute(value: TreeAttribute) -> Json: ...
def from_json_tree_attribute(value: Json) -> TreeAttribute: ...

@dataclass(frozen=True, slots=True)
class TreeAttributeValueString:
    """Quoted string attribute value."""

    string: destack._generated.core.string.StringId
    kind: typing.Literal["string"] = "string"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TreeAttributeValueExpression:
    """Expression container attribute value."""

    expression: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["expression"] = "expression"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""The value form of a tree tag attribute."""
TreeAttributeValue: typing.TypeAlias = (
    TreeAttributeValueString | TreeAttributeValueExpression
)

def encode_tree_attribute_value(
    writer: BinaryWriter, value: TreeAttributeValue
) -> None: ...
def decode_tree_attribute_value(reader: BinaryReader) -> TreeAttributeValue: ...
def to_json_tree_attribute_value(value: TreeAttributeValue) -> Json: ...
def from_json_tree_attribute_value(value: Json) -> TreeAttributeValue: ...

@dataclass(frozen=True, slots=True)
class TreeChildText:
    """Raw tree text."""

    value: destack._generated.core.string.StringId
    kind: typing.Literal["text"] = "text"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TreeChildExpression:
    """Expression container child."""

    value: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["expression"] = "expression"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TreeChildSpread:
    """Spread expression container child."""

    value: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["spread"] = "spread"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TreeChildTree:
    """Nested tree expression child."""

    value: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["tree"] = "tree"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TreeChildError:
    """Malformed child slot."""

    kind: typing.Literal["error"] = "error"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""A tree child."""
TreeChild: typing.TypeAlias = (
    TreeChildText
    | TreeChildExpression
    | TreeChildSpread
    | TreeChildTree
    | TreeChildError
)

def encode_tree_child(writer: BinaryWriter, value: TreeChild) -> None: ...
def decode_tree_child(reader: BinaryReader) -> TreeChild: ...
def to_json_tree_child(value: TreeChild) -> Json: ...
def from_json_tree_child(value: Json) -> TreeChild: ...

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
