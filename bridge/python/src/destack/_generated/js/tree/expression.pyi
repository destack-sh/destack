# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.string
import destack._generated.js.tree.function
import destack._generated.js.tree.literal
import destack._generated.js.tree.node
import destack._generated.js.tree.operator
import destack._generated.js.tree.path
import destack._generated.source.file.model.module

@dataclass(frozen=True, slots=True)
class ExpressionDeclaration:
    """Declaration expression."""

    declaration: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["declaration"] = "declaration"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionPath:
    """Path."""

    path: destack._generated.js.tree.path.Path
    generic_arguments: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["path"] = "path"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionImportMeta:
    """Import meta expression."""

    kind: typing.Literal["importMeta"] = "importMeta"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionThis:
    """This intrinsic value."""

    kind: typing.Literal["this"] = "this"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionSuper:
    """Super intrinsic value."""

    kind: typing.Literal["super"] = "super"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionPrivateIdentifier:
    """Private identifier."""

    name: destack._generated.core.string.StringId
    kind: typing.Literal["privateIdentifier"] = "privateIdentifier"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionScalarLiteral:
    """Scalar literal."""

    value: destack._generated.js.tree.literal.ScalarLiteral
    kind: typing.Literal["scalarLiteral"] = "scalarLiteral"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionTemplateLiteral:
    """Template literal."""

    value: destack._generated.js.tree.literal.TemplateLiteral
    kind: typing.Literal["templateLiteral"] = "templateLiteral"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionArrayLiteral:
    """Array literal."""

    elements: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["arrayLiteral"] = "arrayLiteral"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionSequenceExpression:
    """Sequence expression (JS comma operator)."""

    expressions: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["sequenceExpression"] = "sequenceExpression"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionObjectLiteral:
    """Object literal."""

    properties: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["objectLiteral"] = "objectLiteral"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionParenthesized:
    """Parenthesized expression."""

    expression: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["parenthesized"] = "parenthesized"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionAs:
    """TypeScript-style `as` assertion."""

    expression: destack._generated.js.tree.node.LocalNodeId
    target_type: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["as"] = "as"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionSatisfies:
    """TypeScript-style `satisfies` expression."""

    expression: destack._generated.js.tree.node.LocalNodeId
    target_type: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["satisfies"] = "satisfies"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionInstanceOf:
    """Runtime constructor guard."""

    value: destack._generated.js.tree.node.LocalNodeId
    target: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["instanceOf"] = "instanceOf"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionUnary:
    """Unary operation."""

    operator: destack._generated.js.tree.operator.UnaryOperator
    right: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["unary"] = "unary"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionBinary:
    """Binary operation."""

    left: destack._generated.js.tree.node.LocalNodeId
    operator: destack._generated.js.tree.operator.BinaryOperator
    right: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["binary"] = "binary"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionAssign:
    """Assignment operation."""

    left: destack._generated.js.tree.node.LocalNodeId
    right: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["assign"] = "assign"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionAssignBinary:
    """Assignment binary operation."""

    left: destack._generated.js.tree.node.LocalNodeId
    operator: destack._generated.js.tree.operator.AssignOperator
    right: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["assignBinary"] = "assignBinary"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionMaybe:
    """Maybe unwrap an expression with `?`."""

    position: PostfixPosition
    left: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["maybe"] = "maybe"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionMust:
    """Force unwrap an expression with `!`."""

    position: PostfixPosition
    left: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["must"] = "must"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionMember:
    """Member access."""

    left: destack._generated.js.tree.node.LocalNodeId
    name: destack._generated.core.string.StringId
    kind: typing.Literal["member"] = "member"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionPrivateMember:
    """Private member access."""

    left: destack._generated.js.tree.node.LocalNodeId
    name: destack._generated.core.string.StringId
    kind: typing.Literal["privateMember"] = "privateMember"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionIndex:
    """Index."""

    position: PostfixPosition
    left: destack._generated.js.tree.node.LocalNodeId
    right: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["index"] = "index"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionInstantiation:
    """Instantiation expression."""

    left: destack._generated.js.tree.node.LocalNodeId
    generic_arguments: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["instantiation"] = "instantiation"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionCall:
    """Call."""

    position: PostfixPosition
    left: destack._generated.js.tree.node.LocalNodeId
    generic_arguments: Sequence[destack._generated.js.tree.node.LocalNodeId]
    arguments: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["call"] = "call"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionImportCall:
    """Dynamic import call."""

    target: destack._generated.js.tree.node.LocalNodeId
    target_module: destack._generated.source.file.model.module.ModuleId | None
    arguments: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["importCall"] = "importCall"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionAwait:
    """Await expression."""

    value: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["await"] = "await"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionYield:
    """Yield expression."""

    is_delegate: bool
    value: destack._generated.js.tree.node.LocalNodeId | None
    kind: typing.Literal["yield"] = "yield"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionNew:
    """New."""

    left: destack._generated.js.tree.node.LocalNodeId
    generic_arguments: Sequence[destack._generated.js.tree.node.LocalNodeId]
    arguments: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["new"] = "new"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionArrowFunction:
    """Arrow function expression."""

    signature: destack._generated.js.tree.function.FunctionSignature
    body: ArrowFunctionBody
    kind: typing.Literal["arrowFunction"] = "arrowFunction"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionIfTernary:
    """If ternary."""

    condition: destack._generated.js.tree.node.LocalNodeId
    then_expression: destack._generated.js.tree.node.LocalNodeId
    else_expression: destack._generated.js.tree.node.LocalNodeId | None
    kind: typing.Literal["ifTernary"] = "ifTernary"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionMissing:
    """Missing expression child."""

    kind: typing.Literal["missing"] = "missing"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionStub:
    """Stub placeholder for annotation-only files."""

    kind: typing.Literal["stub"] = "stub"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionError:
    """Error placeholder."""

    kind: typing.Literal["error"] = "error"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""An Expression is value-producing JS form."""
Expression: typing.TypeAlias = (
    ExpressionDeclaration
    | ExpressionPath
    | ExpressionImportMeta
    | ExpressionThis
    | ExpressionSuper
    | ExpressionPrivateIdentifier
    | ExpressionScalarLiteral
    | ExpressionTemplateLiteral
    | ExpressionArrayLiteral
    | ExpressionSequenceExpression
    | ExpressionObjectLiteral
    | ExpressionParenthesized
    | ExpressionAs
    | ExpressionSatisfies
    | ExpressionInstanceOf
    | ExpressionUnary
    | ExpressionBinary
    | ExpressionAssign
    | ExpressionAssignBinary
    | ExpressionMaybe
    | ExpressionMust
    | ExpressionMember
    | ExpressionPrivateMember
    | ExpressionIndex
    | ExpressionInstantiation
    | ExpressionCall
    | ExpressionImportCall
    | ExpressionAwait
    | ExpressionYield
    | ExpressionNew
    | ExpressionArrowFunction
    | ExpressionIfTernary
    | ExpressionMissing
    | ExpressionStub
    | ExpressionError
)

def encode_expression(writer: BinaryWriter, value: Expression) -> None: ...
def decode_expression(reader: BinaryReader) -> Expression: ...
def to_json_expression(value: Expression) -> Json: ...
def from_json_expression(value: Json) -> Expression: ...

"""The position of a postfix expression."""
PostfixPosition: typing.TypeAlias = (
    typing.Literal["direct"] | typing.Literal["indirect"]
)

def encode_postfix_position(writer: BinaryWriter, value: PostfixPosition) -> None: ...
def decode_postfix_position(reader: BinaryReader) -> PostfixPosition: ...
def to_json_postfix_position(value: PostfixPosition) -> Json: ...
def from_json_postfix_position(value: Json) -> PostfixPosition: ...

@dataclass(frozen=True, slots=True)
class ArrowFunctionBodyExpression:
    """Expression body."""

    expression: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["expression"] = "expression"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArrowFunctionBodyBlock:
    """Block body."""

    block: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["block"] = "block"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""One arrow function body."""
ArrowFunctionBody: typing.TypeAlias = (
    ArrowFunctionBodyExpression | ArrowFunctionBodyBlock
)

def encode_arrow_function_body(
    writer: BinaryWriter, value: ArrowFunctionBody
) -> None: ...
def decode_arrow_function_body(reader: BinaryReader) -> ArrowFunctionBody: ...
def to_json_arrow_function_body(value: ArrowFunctionBody) -> Json: ...
def from_json_arrow_function_body(value: Json) -> ArrowFunctionBody: ...

@dataclass(frozen=True, slots=True)
class ArrayElementExpression:
    """One positional array element."""

    value: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["expression"] = "expression"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArrayElementSpread:
    """One spread array element."""

    value: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["spread"] = "spread"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArrayElementElision:
    """One elided array slot."""

    kind: typing.Literal["elision"] = "elision"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""One element in an array literal."""
ArrayElement: typing.TypeAlias = (
    ArrayElementExpression | ArrayElementSpread | ArrayElementElision
)

def encode_array_element(writer: BinaryWriter, value: ArrayElement) -> None: ...
def decode_array_element(reader: BinaryReader) -> ArrayElement: ...
def to_json_array_element(value: ArrayElement) -> Json: ...
def from_json_array_element(value: Json) -> ArrayElement: ...

__all__ = [
    "Expression",
    "encode_expression",
    "decode_expression",
    "to_json_expression",
    "from_json_expression",
    "ExpressionDeclaration",
    "ExpressionPath",
    "ExpressionImportMeta",
    "ExpressionThis",
    "ExpressionSuper",
    "ExpressionPrivateIdentifier",
    "ExpressionScalarLiteral",
    "ExpressionTemplateLiteral",
    "ExpressionArrayLiteral",
    "ExpressionSequenceExpression",
    "ExpressionObjectLiteral",
    "ExpressionParenthesized",
    "ExpressionAs",
    "ExpressionSatisfies",
    "ExpressionInstanceOf",
    "ExpressionUnary",
    "ExpressionBinary",
    "ExpressionAssign",
    "ExpressionAssignBinary",
    "ExpressionMaybe",
    "ExpressionMust",
    "ExpressionMember",
    "ExpressionPrivateMember",
    "ExpressionIndex",
    "ExpressionInstantiation",
    "ExpressionCall",
    "ExpressionImportCall",
    "ExpressionAwait",
    "ExpressionYield",
    "ExpressionNew",
    "ExpressionArrowFunction",
    "ExpressionIfTernary",
    "ExpressionMissing",
    "ExpressionStub",
    "ExpressionError",
    "PostfixPosition",
    "encode_postfix_position",
    "decode_postfix_position",
    "to_json_postfix_position",
    "from_json_postfix_position",
    "ArrowFunctionBody",
    "encode_arrow_function_body",
    "decode_arrow_function_body",
    "to_json_arrow_function_body",
    "from_json_arrow_function_body",
    "ArrowFunctionBodyExpression",
    "ArrowFunctionBodyBlock",
    "ArrayElement",
    "encode_array_element",
    "decode_array_element",
    "to_json_array_element",
    "from_json_array_element",
    "ArrayElementExpression",
    "ArrayElementSpread",
    "ArrayElementElision",
]
