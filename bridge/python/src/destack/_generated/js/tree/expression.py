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
    json_object,
    json_optional,
    json_string,
)

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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionPath:
    """Path."""

    path: destack._generated.js.tree.path.Path
    generic_arguments: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["path"] = "path"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionImportMeta:
    """Import meta expression."""

    kind: typing.Literal["importMeta"] = "importMeta"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionThis:
    """This intrinsic value."""

    kind: typing.Literal["this"] = "this"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionSuper:
    """Super intrinsic value."""

    kind: typing.Literal["super"] = "super"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionPrivateIdentifier:
    """Private identifier."""

    name: destack._generated.core.string.StringId
    kind: typing.Literal["privateIdentifier"] = "privateIdentifier"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionScalarLiteral:
    """Scalar literal."""

    value: destack._generated.js.tree.literal.ScalarLiteral
    kind: typing.Literal["scalarLiteral"] = "scalarLiteral"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionTemplateLiteral:
    """Template literal."""

    value: destack._generated.js.tree.literal.TemplateLiteral
    kind: typing.Literal["templateLiteral"] = "templateLiteral"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionArrayLiteral:
    """Array literal."""

    elements: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["arrayLiteral"] = "arrayLiteral"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionSequenceExpression:
    """Sequence expression (JS comma operator)."""

    expressions: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["sequenceExpression"] = "sequenceExpression"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionObjectLiteral:
    """Object literal."""

    properties: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["objectLiteral"] = "objectLiteral"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionParenthesized:
    """Parenthesized expression."""

    expression: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["parenthesized"] = "parenthesized"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionAs:
    """TypeScript-style `as` assertion."""

    expression: destack._generated.js.tree.node.LocalNodeId
    target_type: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["as"] = "as"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionSatisfies:
    """TypeScript-style `satisfies` expression."""

    expression: destack._generated.js.tree.node.LocalNodeId
    target_type: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["satisfies"] = "satisfies"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionInstanceOf:
    """Runtime constructor guard."""

    value: destack._generated.js.tree.node.LocalNodeId
    target: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["instanceOf"] = "instanceOf"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionUnary:
    """Unary operation."""

    operator: destack._generated.js.tree.operator.UnaryOperator
    right: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["unary"] = "unary"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionBinary:
    """Binary operation."""

    left: destack._generated.js.tree.node.LocalNodeId
    operator: destack._generated.js.tree.operator.BinaryOperator
    right: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["binary"] = "binary"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionAssign:
    """Assignment operation."""

    left: destack._generated.js.tree.node.LocalNodeId
    right: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["assign"] = "assign"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionAssignBinary:
    """Assignment binary operation."""

    left: destack._generated.js.tree.node.LocalNodeId
    operator: destack._generated.js.tree.operator.AssignOperator
    right: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["assignBinary"] = "assignBinary"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionMaybe:
    """Maybe unwrap an expression with `?`."""

    position: PostfixPosition
    left: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["maybe"] = "maybe"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionMust:
    """Force unwrap an expression with `!`."""

    position: PostfixPosition
    left: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["must"] = "must"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionMember:
    """Member access."""

    left: destack._generated.js.tree.node.LocalNodeId
    name: destack._generated.core.string.StringId
    kind: typing.Literal["member"] = "member"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionPrivateMember:
    """Private member access."""

    left: destack._generated.js.tree.node.LocalNodeId
    name: destack._generated.core.string.StringId
    kind: typing.Literal["privateMember"] = "privateMember"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionIndex:
    """Index."""

    position: PostfixPosition
    left: destack._generated.js.tree.node.LocalNodeId
    right: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["index"] = "index"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionInstantiation:
    """Instantiation expression."""

    left: destack._generated.js.tree.node.LocalNodeId
    generic_arguments: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["instantiation"] = "instantiation"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionCall:
    """Call."""

    position: PostfixPosition
    left: destack._generated.js.tree.node.LocalNodeId
    generic_arguments: Sequence[destack._generated.js.tree.node.LocalNodeId]
    arguments: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["call"] = "call"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionImportCall:
    """Dynamic import call."""

    target: destack._generated.js.tree.node.LocalNodeId
    target_module: destack._generated.source.file.model.module.ModuleId | None
    arguments: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["importCall"] = "importCall"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionAwait:
    """Await expression."""

    value: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["await"] = "await"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionYield:
    """Yield expression."""

    is_delegate: bool
    value: destack._generated.js.tree.node.LocalNodeId | None
    kind: typing.Literal["yield"] = "yield"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionNew:
    """New."""

    left: destack._generated.js.tree.node.LocalNodeId
    generic_arguments: Sequence[destack._generated.js.tree.node.LocalNodeId]
    arguments: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["new"] = "new"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionArrowFunction:
    """Arrow function expression."""

    signature: destack._generated.js.tree.function.FunctionSignature
    body: ArrowFunctionBody
    kind: typing.Literal["arrowFunction"] = "arrowFunction"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionIfTernary:
    """If ternary."""

    condition: destack._generated.js.tree.node.LocalNodeId
    then_expression: destack._generated.js.tree.node.LocalNodeId
    else_expression: destack._generated.js.tree.node.LocalNodeId | None
    kind: typing.Literal["ifTernary"] = "ifTernary"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionMissing:
    """Missing expression child."""

    kind: typing.Literal["missing"] = "missing"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionStub:
    """Stub placeholder for annotation-only files."""

    kind: typing.Literal["stub"] = "stub"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionError:
    """Error placeholder."""

    kind: typing.Literal["error"] = "error"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


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


def encode_expression(writer: BinaryWriter, value: Expression) -> None:
    """Encode one Expression."""
    if value.kind == "declaration":
        writer.write_unsigned(0)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.declaration)
    elif value.kind == "path":
        writer.write_unsigned(1)
        destack._generated.js.tree.path.encode_path(writer, value.path)
        writer.write_unsigned(len(value.generic_arguments))
        for item_value_generic_arguments_0 in value.generic_arguments:
            destack._generated.js.tree.node.encode_local_node_id(
                writer, item_value_generic_arguments_0
            )
    elif value.kind == "importMeta":
        writer.write_unsigned(2)
    elif value.kind == "this":
        writer.write_unsigned(3)
    elif value.kind == "super":
        writer.write_unsigned(4)
    elif value.kind == "privateIdentifier":
        writer.write_unsigned(5)
        destack._generated.core.string.encode_string_id(writer, value.name)
    elif value.kind == "scalarLiteral":
        writer.write_unsigned(6)
        destack._generated.js.tree.literal.encode_scalar_literal(writer, value.value)
    elif value.kind == "templateLiteral":
        writer.write_unsigned(7)
        destack._generated.js.tree.literal.encode_template_literal(writer, value.value)
    elif value.kind == "arrayLiteral":
        writer.write_unsigned(8)
        writer.write_unsigned(len(value.elements))
        for item_value_elements_0 in value.elements:
            destack._generated.js.tree.node.encode_local_node_id(
                writer, item_value_elements_0
            )
    elif value.kind == "sequenceExpression":
        writer.write_unsigned(9)
        writer.write_unsigned(len(value.expressions))
        for item_value_expressions_0 in value.expressions:
            destack._generated.js.tree.node.encode_local_node_id(
                writer, item_value_expressions_0
            )
    elif value.kind == "objectLiteral":
        writer.write_unsigned(10)
        writer.write_unsigned(len(value.properties))
        for item_value_properties_0 in value.properties:
            destack._generated.js.tree.node.encode_local_node_id(
                writer, item_value_properties_0
            )
    elif value.kind == "parenthesized":
        writer.write_unsigned(11)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.expression)
    elif value.kind == "as":
        writer.write_unsigned(12)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.expression)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.target_type)
    elif value.kind == "satisfies":
        writer.write_unsigned(13)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.expression)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.target_type)
    elif value.kind == "instanceOf":
        writer.write_unsigned(14)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.value)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.target)
    elif value.kind == "unary":
        writer.write_unsigned(15)
        destack._generated.js.tree.operator.encode_unary_operator(
            writer, value.operator
        )
        destack._generated.js.tree.node.encode_local_node_id(writer, value.right)
    elif value.kind == "binary":
        writer.write_unsigned(16)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.left)
        destack._generated.js.tree.operator.encode_binary_operator(
            writer, value.operator
        )
        destack._generated.js.tree.node.encode_local_node_id(writer, value.right)
    elif value.kind == "assign":
        writer.write_unsigned(17)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.left)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.right)
    elif value.kind == "assignBinary":
        writer.write_unsigned(18)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.left)
        destack._generated.js.tree.operator.encode_assign_operator(
            writer, value.operator
        )
        destack._generated.js.tree.node.encode_local_node_id(writer, value.right)
    elif value.kind == "maybe":
        writer.write_unsigned(19)
        encode_postfix_position(writer, value.position)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.left)
    elif value.kind == "must":
        writer.write_unsigned(20)
        encode_postfix_position(writer, value.position)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.left)
    elif value.kind == "member":
        writer.write_unsigned(21)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.left)
        destack._generated.core.string.encode_string_id(writer, value.name)
    elif value.kind == "privateMember":
        writer.write_unsigned(22)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.left)
        destack._generated.core.string.encode_string_id(writer, value.name)
    elif value.kind == "index":
        writer.write_unsigned(23)
        encode_postfix_position(writer, value.position)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.left)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.right)
    elif value.kind == "instantiation":
        writer.write_unsigned(24)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.left)
        writer.write_unsigned(len(value.generic_arguments))
        for item_value_generic_arguments_0 in value.generic_arguments:
            destack._generated.js.tree.node.encode_local_node_id(
                writer, item_value_generic_arguments_0
            )
    elif value.kind == "call":
        writer.write_unsigned(25)
        encode_postfix_position(writer, value.position)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.left)
        writer.write_unsigned(len(value.generic_arguments))
        for item_value_generic_arguments_0 in value.generic_arguments:
            destack._generated.js.tree.node.encode_local_node_id(
                writer, item_value_generic_arguments_0
            )
        writer.write_unsigned(len(value.arguments))
        for item_value_arguments_0 in value.arguments:
            destack._generated.js.tree.node.encode_local_node_id(
                writer, item_value_arguments_0
            )
    elif value.kind == "importCall":
        writer.write_unsigned(26)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.target)
        if value.target_module is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.source.file.model.module.encode_module_id(
                writer, value.target_module
            )
        writer.write_unsigned(len(value.arguments))
        for item_value_arguments_0 in value.arguments:
            destack._generated.js.tree.node.encode_local_node_id(
                writer, item_value_arguments_0
            )
    elif value.kind == "await":
        writer.write_unsigned(27)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.value)
    elif value.kind == "yield":
        writer.write_unsigned(28)
        writer.write_bool(value.is_delegate)
        if value.value is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.js.tree.node.encode_local_node_id(writer, value.value)
    elif value.kind == "new":
        writer.write_unsigned(29)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.left)
        writer.write_unsigned(len(value.generic_arguments))
        for item_value_generic_arguments_0 in value.generic_arguments:
            destack._generated.js.tree.node.encode_local_node_id(
                writer, item_value_generic_arguments_0
            )
        writer.write_unsigned(len(value.arguments))
        for item_value_arguments_0 in value.arguments:
            destack._generated.js.tree.node.encode_local_node_id(
                writer, item_value_arguments_0
            )
    elif value.kind == "arrowFunction":
        writer.write_unsigned(30)
        destack._generated.js.tree.function.encode_function_signature(
            writer, value.signature
        )
        encode_arrow_function_body(writer, value.body)
    elif value.kind == "ifTernary":
        writer.write_unsigned(31)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.condition)
        destack._generated.js.tree.node.encode_local_node_id(
            writer, value.then_expression
        )
        if value.else_expression is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.js.tree.node.encode_local_node_id(
                writer, value.else_expression
            )
    elif value.kind == "missing":
        writer.write_unsigned(32)
    elif value.kind == "stub":
        writer.write_unsigned(33)
    elif value.kind == "error":
        writer.write_unsigned(34)
    else:
        raise SerdeError("unknown enum variant")


def decode_expression(reader: BinaryReader) -> Expression:
    """Decode one Expression."""
    variant = reader.read_number()

    if variant == 0:
        declaration = destack._generated.js.tree.node.decode_local_node_id(reader)

        return ExpressionDeclaration(
            declaration=declaration,
        )
    elif variant == 1:
        path = destack._generated.js.tree.path.decode_path(reader)
        generic_arguments = [
            destack._generated.js.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return ExpressionPath(
            path=path,
            generic_arguments=generic_arguments,
        )
    elif variant == 2:
        return ExpressionImportMeta()
    elif variant == 3:
        return ExpressionThis()
    elif variant == 4:
        return ExpressionSuper()
    elif variant == 5:
        name = destack._generated.core.string.decode_string_id(reader)

        return ExpressionPrivateIdentifier(
            name=name,
        )
    elif variant == 6:
        value_ = destack._generated.js.tree.literal.decode_scalar_literal(reader)

        return ExpressionScalarLiteral(
            value=value_,
        )
    elif variant == 7:
        value_ = destack._generated.js.tree.literal.decode_template_literal(reader)

        return ExpressionTemplateLiteral(
            value=value_,
        )
    elif variant == 8:
        elements = [
            destack._generated.js.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return ExpressionArrayLiteral(
            elements=elements,
        )
    elif variant == 9:
        expressions = [
            destack._generated.js.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return ExpressionSequenceExpression(
            expressions=expressions,
        )
    elif variant == 10:
        properties = [
            destack._generated.js.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return ExpressionObjectLiteral(
            properties=properties,
        )
    elif variant == 11:
        expression = destack._generated.js.tree.node.decode_local_node_id(reader)

        return ExpressionParenthesized(
            expression=expression,
        )
    elif variant == 12:
        expression = destack._generated.js.tree.node.decode_local_node_id(reader)
        target_type = destack._generated.js.tree.node.decode_local_node_id(reader)

        return ExpressionAs(
            expression=expression,
            target_type=target_type,
        )
    elif variant == 13:
        expression = destack._generated.js.tree.node.decode_local_node_id(reader)
        target_type = destack._generated.js.tree.node.decode_local_node_id(reader)

        return ExpressionSatisfies(
            expression=expression,
            target_type=target_type,
        )
    elif variant == 14:
        value_ = destack._generated.js.tree.node.decode_local_node_id(reader)
        target = destack._generated.js.tree.node.decode_local_node_id(reader)

        return ExpressionInstanceOf(
            value=value_,
            target=target,
        )
    elif variant == 15:
        operator = destack._generated.js.tree.operator.decode_unary_operator(reader)
        right = destack._generated.js.tree.node.decode_local_node_id(reader)

        return ExpressionUnary(
            operator=operator,
            right=right,
        )
    elif variant == 16:
        left = destack._generated.js.tree.node.decode_local_node_id(reader)
        operator = destack._generated.js.tree.operator.decode_binary_operator(reader)
        right = destack._generated.js.tree.node.decode_local_node_id(reader)

        return ExpressionBinary(
            left=left,
            operator=operator,
            right=right,
        )
    elif variant == 17:
        left = destack._generated.js.tree.node.decode_local_node_id(reader)
        right = destack._generated.js.tree.node.decode_local_node_id(reader)

        return ExpressionAssign(
            left=left,
            right=right,
        )
    elif variant == 18:
        left = destack._generated.js.tree.node.decode_local_node_id(reader)
        operator = destack._generated.js.tree.operator.decode_assign_operator(reader)
        right = destack._generated.js.tree.node.decode_local_node_id(reader)

        return ExpressionAssignBinary(
            left=left,
            operator=operator,
            right=right,
        )
    elif variant == 19:
        position = decode_postfix_position(reader)
        left = destack._generated.js.tree.node.decode_local_node_id(reader)

        return ExpressionMaybe(
            position=position,
            left=left,
        )
    elif variant == 20:
        position = decode_postfix_position(reader)
        left = destack._generated.js.tree.node.decode_local_node_id(reader)

        return ExpressionMust(
            position=position,
            left=left,
        )
    elif variant == 21:
        left = destack._generated.js.tree.node.decode_local_node_id(reader)
        name = destack._generated.core.string.decode_string_id(reader)

        return ExpressionMember(
            left=left,
            name=name,
        )
    elif variant == 22:
        left = destack._generated.js.tree.node.decode_local_node_id(reader)
        name = destack._generated.core.string.decode_string_id(reader)

        return ExpressionPrivateMember(
            left=left,
            name=name,
        )
    elif variant == 23:
        position = decode_postfix_position(reader)
        left = destack._generated.js.tree.node.decode_local_node_id(reader)
        right = destack._generated.js.tree.node.decode_local_node_id(reader)

        return ExpressionIndex(
            position=position,
            left=left,
            right=right,
        )
    elif variant == 24:
        left = destack._generated.js.tree.node.decode_local_node_id(reader)
        generic_arguments = [
            destack._generated.js.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return ExpressionInstantiation(
            left=left,
            generic_arguments=generic_arguments,
        )
    elif variant == 25:
        position = decode_postfix_position(reader)
        left = destack._generated.js.tree.node.decode_local_node_id(reader)
        generic_arguments = [
            destack._generated.js.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]
        arguments = [
            destack._generated.js.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return ExpressionCall(
            position=position,
            left=left,
            generic_arguments=generic_arguments,
            arguments=arguments,
        )
    elif variant == 26:
        target = destack._generated.js.tree.node.decode_local_node_id(reader)
        target_module = reader.read_option(
            lambda: destack._generated.source.file.model.module.decode_module_id(reader)
        )
        arguments = [
            destack._generated.js.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return ExpressionImportCall(
            target=target,
            target_module=target_module,
            arguments=arguments,
        )
    elif variant == 27:
        value_ = destack._generated.js.tree.node.decode_local_node_id(reader)

        return ExpressionAwait(
            value=value_,
        )
    elif variant == 28:
        is_delegate = reader.read_bool()
        value_ = reader.read_option(
            lambda: destack._generated.js.tree.node.decode_local_node_id(reader)
        )

        return ExpressionYield(
            is_delegate=is_delegate,
            value=value_,
        )
    elif variant == 29:
        left = destack._generated.js.tree.node.decode_local_node_id(reader)
        generic_arguments = [
            destack._generated.js.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]
        arguments = [
            destack._generated.js.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return ExpressionNew(
            left=left,
            generic_arguments=generic_arguments,
            arguments=arguments,
        )
    elif variant == 30:
        signature = destack._generated.js.tree.function.decode_function_signature(
            reader
        )
        body = decode_arrow_function_body(reader)

        return ExpressionArrowFunction(
            signature=signature,
            body=body,
        )
    elif variant == 31:
        condition = destack._generated.js.tree.node.decode_local_node_id(reader)
        then_expression = destack._generated.js.tree.node.decode_local_node_id(reader)
        else_expression = reader.read_option(
            lambda: destack._generated.js.tree.node.decode_local_node_id(reader)
        )

        return ExpressionIfTernary(
            condition=condition,
            then_expression=then_expression,
            else_expression=else_expression,
        )
    elif variant == 32:
        return ExpressionMissing()
    elif variant == 33:
        return ExpressionStub()
    elif variant == 34:
        return ExpressionError()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_expression(value: Expression) -> Json:
    """Return one JSON value for one Expression."""
    if value.kind == "declaration":
        return {
            "kind": "declaration",
            "declaration": destack._generated.js.tree.node.to_json_local_node_id(
                value.declaration
            ),
        }
    elif value.kind == "path":
        return {
            "kind": "path",
            "path": destack._generated.js.tree.path.to_json_path(value.path),
            "genericArguments": [
                destack._generated.js.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.generic_arguments
            ],
        }
    elif value.kind == "importMeta":
        return {
            "kind": "importMeta",
        }
    elif value.kind == "this":
        return {
            "kind": "this",
        }
    elif value.kind == "super":
        return {
            "kind": "super",
        }
    elif value.kind == "privateIdentifier":
        return {
            "kind": "privateIdentifier",
            "name": destack._generated.core.string.to_json_string_id(value.name),
        }
    elif value.kind == "scalarLiteral":
        return {
            "kind": "scalarLiteral",
            "value": destack._generated.js.tree.literal.to_json_scalar_literal(
                value.value
            ),
        }
    elif value.kind == "templateLiteral":
        return {
            "kind": "templateLiteral",
            "value": destack._generated.js.tree.literal.to_json_template_literal(
                value.value
            ),
        }
    elif value.kind == "arrayLiteral":
        return {
            "kind": "arrayLiteral",
            "elements": [
                destack._generated.js.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.elements
            ],
        }
    elif value.kind == "sequenceExpression":
        return {
            "kind": "sequenceExpression",
            "expressions": [
                destack._generated.js.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.expressions
            ],
        }
    elif value.kind == "objectLiteral":
        return {
            "kind": "objectLiteral",
            "properties": [
                destack._generated.js.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.properties
            ],
        }
    elif value.kind == "parenthesized":
        return {
            "kind": "parenthesized",
            "expression": destack._generated.js.tree.node.to_json_local_node_id(
                value.expression
            ),
        }
    elif value.kind == "as":
        return {
            "kind": "as",
            "expression": destack._generated.js.tree.node.to_json_local_node_id(
                value.expression
            ),
            "targetType": destack._generated.js.tree.node.to_json_local_node_id(
                value.target_type
            ),
        }
    elif value.kind == "satisfies":
        return {
            "kind": "satisfies",
            "expression": destack._generated.js.tree.node.to_json_local_node_id(
                value.expression
            ),
            "targetType": destack._generated.js.tree.node.to_json_local_node_id(
                value.target_type
            ),
        }
    elif value.kind == "instanceOf":
        return {
            "kind": "instanceOf",
            "value": destack._generated.js.tree.node.to_json_local_node_id(value.value),
            "target": destack._generated.js.tree.node.to_json_local_node_id(
                value.target
            ),
        }
    elif value.kind == "unary":
        return {
            "kind": "unary",
            "operator": destack._generated.js.tree.operator.to_json_unary_operator(
                value.operator
            ),
            "right": destack._generated.js.tree.node.to_json_local_node_id(value.right),
        }
    elif value.kind == "binary":
        return {
            "kind": "binary",
            "left": destack._generated.js.tree.node.to_json_local_node_id(value.left),
            "operator": destack._generated.js.tree.operator.to_json_binary_operator(
                value.operator
            ),
            "right": destack._generated.js.tree.node.to_json_local_node_id(value.right),
        }
    elif value.kind == "assign":
        return {
            "kind": "assign",
            "left": destack._generated.js.tree.node.to_json_local_node_id(value.left),
            "right": destack._generated.js.tree.node.to_json_local_node_id(value.right),
        }
    elif value.kind == "assignBinary":
        return {
            "kind": "assignBinary",
            "left": destack._generated.js.tree.node.to_json_local_node_id(value.left),
            "operator": destack._generated.js.tree.operator.to_json_assign_operator(
                value.operator
            ),
            "right": destack._generated.js.tree.node.to_json_local_node_id(value.right),
        }
    elif value.kind == "maybe":
        return {
            "kind": "maybe",
            "position": to_json_postfix_position(value.position),
            "left": destack._generated.js.tree.node.to_json_local_node_id(value.left),
        }
    elif value.kind == "must":
        return {
            "kind": "must",
            "position": to_json_postfix_position(value.position),
            "left": destack._generated.js.tree.node.to_json_local_node_id(value.left),
        }
    elif value.kind == "member":
        return {
            "kind": "member",
            "left": destack._generated.js.tree.node.to_json_local_node_id(value.left),
            "name": destack._generated.core.string.to_json_string_id(value.name),
        }
    elif value.kind == "privateMember":
        return {
            "kind": "privateMember",
            "left": destack._generated.js.tree.node.to_json_local_node_id(value.left),
            "name": destack._generated.core.string.to_json_string_id(value.name),
        }
    elif value.kind == "index":
        return {
            "kind": "index",
            "position": to_json_postfix_position(value.position),
            "left": destack._generated.js.tree.node.to_json_local_node_id(value.left),
            "right": destack._generated.js.tree.node.to_json_local_node_id(value.right),
        }
    elif value.kind == "instantiation":
        return {
            "kind": "instantiation",
            "left": destack._generated.js.tree.node.to_json_local_node_id(value.left),
            "genericArguments": [
                destack._generated.js.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.generic_arguments
            ],
        }
    elif value.kind == "call":
        return {
            "kind": "call",
            "position": to_json_postfix_position(value.position),
            "left": destack._generated.js.tree.node.to_json_local_node_id(value.left),
            "genericArguments": [
                destack._generated.js.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.generic_arguments
            ],
            "arguments": [
                destack._generated.js.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.arguments
            ],
        }
    elif value.kind == "importCall":
        return {
            "kind": "importCall",
            "target": destack._generated.js.tree.node.to_json_local_node_id(
                value.target
            ),
            **(
                {}
                if value.target_module is None
                else {
                    "targetModule": destack._generated.source.file.model.module.to_json_module_id(
                        value.target_module
                    )
                }
            ),
            "arguments": [
                destack._generated.js.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.arguments
            ],
        }
    elif value.kind == "await":
        return {
            "kind": "await",
            "value": destack._generated.js.tree.node.to_json_local_node_id(value.value),
        }
    elif value.kind == "yield":
        return {
            "kind": "yield",
            "isDelegate": value.is_delegate,
            **(
                {}
                if value.value is None
                else {
                    "value": destack._generated.js.tree.node.to_json_local_node_id(
                        value.value
                    )
                }
            ),
        }
    elif value.kind == "new":
        return {
            "kind": "new",
            "left": destack._generated.js.tree.node.to_json_local_node_id(value.left),
            "genericArguments": [
                destack._generated.js.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.generic_arguments
            ],
            "arguments": [
                destack._generated.js.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.arguments
            ],
        }
    elif value.kind == "arrowFunction":
        return {
            "kind": "arrowFunction",
            "signature": destack._generated.js.tree.function.to_json_function_signature(
                value.signature
            ),
            "body": to_json_arrow_function_body(value.body),
        }
    elif value.kind == "ifTernary":
        return {
            "kind": "ifTernary",
            "condition": destack._generated.js.tree.node.to_json_local_node_id(
                value.condition
            ),
            "thenExpression": destack._generated.js.tree.node.to_json_local_node_id(
                value.then_expression
            ),
            **(
                {}
                if value.else_expression is None
                else {
                    "elseExpression": destack._generated.js.tree.node.to_json_local_node_id(
                        value.else_expression
                    )
                }
            ),
        }
    elif value.kind == "missing":
        return {
            "kind": "missing",
        }
    elif value.kind == "stub":
        return {
            "kind": "stub",
        }
    elif value.kind == "error":
        return {
            "kind": "error",
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_expression(value: Json) -> Expression:
    """Return one Expression from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "declaration":
        return ExpressionDeclaration(
            declaration=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "declaration")
            ),
        )
    elif kind == "path":
        return ExpressionPath(
            path=destack._generated.js.tree.path.from_json_path(
                json_field(object_, "path")
            ),
            generic_arguments=[
                destack._generated.js.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "genericArguments"))
            ],
        )
    elif kind == "importMeta":
        return ExpressionImportMeta()
    elif kind == "this":
        return ExpressionThis()
    elif kind == "super":
        return ExpressionSuper()
    elif kind == "privateIdentifier":
        return ExpressionPrivateIdentifier(
            name=destack._generated.core.string.from_json_string_id(
                json_field(object_, "name")
            ),
        )
    elif kind == "scalarLiteral":
        return ExpressionScalarLiteral(
            value=destack._generated.js.tree.literal.from_json_scalar_literal(
                json_field(object_, "value")
            ),
        )
    elif kind == "templateLiteral":
        return ExpressionTemplateLiteral(
            value=destack._generated.js.tree.literal.from_json_template_literal(
                json_field(object_, "value")
            ),
        )
    elif kind == "arrayLiteral":
        return ExpressionArrayLiteral(
            elements=[
                destack._generated.js.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "elements"))
            ],
        )
    elif kind == "sequenceExpression":
        return ExpressionSequenceExpression(
            expressions=[
                destack._generated.js.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "expressions"))
            ],
        )
    elif kind == "objectLiteral":
        return ExpressionObjectLiteral(
            properties=[
                destack._generated.js.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "properties"))
            ],
        )
    elif kind == "parenthesized":
        return ExpressionParenthesized(
            expression=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "expression")
            ),
        )
    elif kind == "as":
        return ExpressionAs(
            expression=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "expression")
            ),
            target_type=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "targetType")
            ),
        )
    elif kind == "satisfies":
        return ExpressionSatisfies(
            expression=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "expression")
            ),
            target_type=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "targetType")
            ),
        )
    elif kind == "instanceOf":
        return ExpressionInstanceOf(
            value=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "value")
            ),
            target=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "target")
            ),
        )
    elif kind == "unary":
        return ExpressionUnary(
            operator=destack._generated.js.tree.operator.from_json_unary_operator(
                json_field(object_, "operator")
            ),
            right=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "right")
            ),
        )
    elif kind == "binary":
        return ExpressionBinary(
            left=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "left")
            ),
            operator=destack._generated.js.tree.operator.from_json_binary_operator(
                json_field(object_, "operator")
            ),
            right=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "right")
            ),
        )
    elif kind == "assign":
        return ExpressionAssign(
            left=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "left")
            ),
            right=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "right")
            ),
        )
    elif kind == "assignBinary":
        return ExpressionAssignBinary(
            left=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "left")
            ),
            operator=destack._generated.js.tree.operator.from_json_assign_operator(
                json_field(object_, "operator")
            ),
            right=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "right")
            ),
        )
    elif kind == "maybe":
        return ExpressionMaybe(
            position=from_json_postfix_position(json_field(object_, "position")),
            left=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "left")
            ),
        )
    elif kind == "must":
        return ExpressionMust(
            position=from_json_postfix_position(json_field(object_, "position")),
            left=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "left")
            ),
        )
    elif kind == "member":
        return ExpressionMember(
            left=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "left")
            ),
            name=destack._generated.core.string.from_json_string_id(
                json_field(object_, "name")
            ),
        )
    elif kind == "privateMember":
        return ExpressionPrivateMember(
            left=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "left")
            ),
            name=destack._generated.core.string.from_json_string_id(
                json_field(object_, "name")
            ),
        )
    elif kind == "index":
        return ExpressionIndex(
            position=from_json_postfix_position(json_field(object_, "position")),
            left=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "left")
            ),
            right=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "right")
            ),
        )
    elif kind == "instantiation":
        return ExpressionInstantiation(
            left=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "left")
            ),
            generic_arguments=[
                destack._generated.js.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "genericArguments"))
            ],
        )
    elif kind == "call":
        return ExpressionCall(
            position=from_json_postfix_position(json_field(object_, "position")),
            left=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "left")
            ),
            generic_arguments=[
                destack._generated.js.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "genericArguments"))
            ],
            arguments=[
                destack._generated.js.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "arguments"))
            ],
        )
    elif kind == "importCall":
        return ExpressionImportCall(
            target=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "target")
            ),
            target_module=json_optional(
                object_,
                "targetModule",
                lambda value: (
                    destack._generated.source.file.model.module.from_json_module_id(
                        value
                    )
                ),
            ),
            arguments=[
                destack._generated.js.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "arguments"))
            ],
        )
    elif kind == "await":
        return ExpressionAwait(
            value=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "value")
            ),
        )
    elif kind == "yield":
        return ExpressionYield(
            is_delegate=json_bool(json_field(object_, "isDelegate")),
            value=json_optional(
                object_,
                "value",
                lambda value: destack._generated.js.tree.node.from_json_local_node_id(
                    value
                ),
            ),
        )
    elif kind == "new":
        return ExpressionNew(
            left=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "left")
            ),
            generic_arguments=[
                destack._generated.js.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "genericArguments"))
            ],
            arguments=[
                destack._generated.js.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "arguments"))
            ],
        )
    elif kind == "arrowFunction":
        return ExpressionArrowFunction(
            signature=destack._generated.js.tree.function.from_json_function_signature(
                json_field(object_, "signature")
            ),
            body=from_json_arrow_function_body(json_field(object_, "body")),
        )
    elif kind == "ifTernary":
        return ExpressionIfTernary(
            condition=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "condition")
            ),
            then_expression=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "thenExpression")
            ),
            else_expression=json_optional(
                object_,
                "elseExpression",
                lambda value: destack._generated.js.tree.node.from_json_local_node_id(
                    value
                ),
            ),
        )
    elif kind == "missing":
        return ExpressionMissing()
    elif kind == "stub":
        return ExpressionStub()
    elif kind == "error":
        return ExpressionError()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


"""The position of a postfix expression."""
PostfixPosition: typing.TypeAlias = (
    typing.Literal["direct"] | typing.Literal["indirect"]
)


def encode_postfix_position(writer: BinaryWriter, value: PostfixPosition) -> None:
    """Encode one PostfixPosition."""
    if value == "direct":
        writer.write_unsigned(0)
    elif value == "indirect":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_postfix_position(reader: BinaryReader) -> PostfixPosition:
    """Decode one PostfixPosition."""
    variant = reader.read_number()

    if variant == 0:
        return "direct"
    elif variant == 1:
        return "indirect"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_postfix_position(value: PostfixPosition) -> Json:
    """Return one JSON value for one PostfixPosition."""
    return value


def from_json_postfix_position(value: Json) -> PostfixPosition:
    """Return one PostfixPosition from one JSON value."""
    variant = json_string(value)

    if variant == "direct":
        return "direct"
    elif variant == "indirect":
        return "indirect"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class ArrowFunctionBodyExpression:
    """Expression body."""

    expression: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["expression"] = "expression"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_arrow_function_body(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_arrow_function_body(self)


@dataclass(frozen=True, slots=True)
class ArrowFunctionBodyBlock:
    """Block body."""

    block: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["block"] = "block"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_arrow_function_body(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_arrow_function_body(self)


"""One arrow function body."""
ArrowFunctionBody: typing.TypeAlias = (
    ArrowFunctionBodyExpression | ArrowFunctionBodyBlock
)


def encode_arrow_function_body(writer: BinaryWriter, value: ArrowFunctionBody) -> None:
    """Encode one ArrowFunctionBody."""
    if value.kind == "expression":
        writer.write_unsigned(0)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.expression)
    elif value.kind == "block":
        writer.write_unsigned(1)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.block)
    else:
        raise SerdeError("unknown enum variant")


def decode_arrow_function_body(reader: BinaryReader) -> ArrowFunctionBody:
    """Decode one ArrowFunctionBody."""
    variant = reader.read_number()

    if variant == 0:
        expression = destack._generated.js.tree.node.decode_local_node_id(reader)

        return ArrowFunctionBodyExpression(expression=expression)
    elif variant == 1:
        block = destack._generated.js.tree.node.decode_local_node_id(reader)

        return ArrowFunctionBodyBlock(block=block)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_arrow_function_body(value: ArrowFunctionBody) -> Json:
    """Return one JSON value for one ArrowFunctionBody."""
    if value.kind == "expression":
        return {
            "kind": "expression",
            "expression": destack._generated.js.tree.node.to_json_local_node_id(
                value.expression
            ),
        }
    elif value.kind == "block":
        return {
            "kind": "block",
            "block": destack._generated.js.tree.node.to_json_local_node_id(value.block),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_arrow_function_body(value: Json) -> ArrowFunctionBody:
    """Return one ArrowFunctionBody from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "expression":
        return ArrowFunctionBodyExpression(
            expression=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "expression")
            )
        )
    elif kind == "block":
        return ArrowFunctionBodyBlock(
            block=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "block")
            )
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class ArrayElementExpression:
    """One positional array element."""

    value: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["expression"] = "expression"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_array_element(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_array_element(self)


@dataclass(frozen=True, slots=True)
class ArrayElementSpread:
    """One spread array element."""

    value: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["spread"] = "spread"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_array_element(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_array_element(self)


@dataclass(frozen=True, slots=True)
class ArrayElementElision:
    """One elided array slot."""

    kind: typing.Literal["elision"] = "elision"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_array_element(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_array_element(self)


"""One element in an array literal."""
ArrayElement: typing.TypeAlias = (
    ArrayElementExpression | ArrayElementSpread | ArrayElementElision
)


def encode_array_element(writer: BinaryWriter, value: ArrayElement) -> None:
    """Encode one ArrayElement."""
    if value.kind == "expression":
        writer.write_unsigned(0)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.value)
    elif value.kind == "spread":
        writer.write_unsigned(1)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.value)
    elif value.kind == "elision":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_array_element(reader: BinaryReader) -> ArrayElement:
    """Decode one ArrayElement."""
    variant = reader.read_number()

    if variant == 0:
        value_ = destack._generated.js.tree.node.decode_local_node_id(reader)

        return ArrayElementExpression(
            value=value_,
        )
    elif variant == 1:
        value_ = destack._generated.js.tree.node.decode_local_node_id(reader)

        return ArrayElementSpread(
            value=value_,
        )
    elif variant == 2:
        return ArrayElementElision()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_array_element(value: ArrayElement) -> Json:
    """Return one JSON value for one ArrayElement."""
    if value.kind == "expression":
        return {
            "kind": "expression",
            "value": destack._generated.js.tree.node.to_json_local_node_id(value.value),
        }
    elif value.kind == "spread":
        return {
            "kind": "spread",
            "value": destack._generated.js.tree.node.to_json_local_node_id(value.value),
        }
    elif value.kind == "elision":
        return {
            "kind": "elision",
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_array_element(value: Json) -> ArrayElement:
    """Return one ArrayElement from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "expression":
        return ArrayElementExpression(
            value=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "value")
            ),
        )
    elif kind == "spread":
        return ArrayElementSpread(
            value=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "value")
            ),
        )
    elif kind == "elision":
        return ArrayElementElision()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


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
