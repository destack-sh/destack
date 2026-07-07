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

from destack._impl.dir.tree.expression import (
    ExpressionImpl,
)

import destack._generated.core.string
import destack._generated.dir.tree.declaration
import destack._generated.dir.tree.dependency
import destack._generated.dir.tree.import_
import destack._generated.dir.tree.literal
import destack._generated.dir.tree.match
import destack._generated.dir.tree.node
import destack._generated.dir.tree.operator


@dataclass(frozen=True, slots=True)
class ExpressionDeclaration(ExpressionImpl):
    """Declaration (with a name or anonymous)."""

    declaration: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["declaration"] = "declaration"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionBlock(ExpressionImpl):
    """Block of Expressions."""

    block: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["block"] = "block"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionLabel(ExpressionImpl):
    """Label statement (like `label: stmt` in JavaScript)."""

    label: destack._generated.core.string.StringId
    body: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["label"] = "label"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionImport(ExpressionImpl):
    """An Import is an import declaration for dependency management."""

    form: destack._generated.dir.tree.dependency.DependencyForm
    target: destack._generated.core.string.StringId
    items: Sequence[destack._generated.dir.tree.node.LocalNodeId] | None
    attributes: destack._generated.dir.tree.import_.ImportAttributeClause | None
    kind: typing.Literal["import"] = "import"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionExport(ExpressionImpl):
    """An Export is an explicit export declaration for dependency management."""

    form: destack._generated.dir.tree.dependency.DependencyForm
    target: destack._generated.core.string.StringId | None
    items: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    attributes: destack._generated.dir.tree.import_.ImportAttributeClause | None
    kind: typing.Literal["export"] = "export"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionLet(ExpressionImpl):
    """Let binding for mutable and immutable variables."""

    kind_value: LetKind
    export: destack._generated.dir.tree.dependency.ExportKind | None
    mutability: destack._generated.dir.tree.node.Mutability
    declarators: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    is_ambient: bool
    place: destack._generated.dir.tree.declaration.PlaceModifier | None
    kind: typing.Literal["let"] = "let"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionLetElse(ExpressionImpl):
    """Let-else binding with an early-exit branch."""

    kind_value: LetKind
    mutability: destack._generated.dir.tree.node.Mutability
    declarator: destack._generated.dir.tree.node.LocalNodeId
    else_branch: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["letElse"] = "letElse"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionUsing(ExpressionImpl):
    """Using binding for resources with deterministic disposal."""

    asynchrony: destack._generated.dir.tree.node.Asynchrony
    export: destack._generated.dir.tree.dependency.ExportKind | None
    declarators: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    is_ambient: bool
    kind: typing.Literal["using"] = "using"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionIf(ExpressionImpl):
    """If/then/else expression."""

    form: IfForm
    condition: Condition
    then_expression: destack._generated.dir.tree.node.LocalNodeId
    else_expression: destack._generated.dir.tree.node.LocalNodeId | None
    kind: typing.Literal["if"] = "if"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionWhile(ExpressionImpl):
    """A While is while or do-while loop."""

    form: WhileForm
    condition: destack._generated.dir.tree.node.LocalNodeId
    body: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["while"] = "while"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionForEach(ExpressionImpl):
    """A ForEach is a for loop over an iterator with a binding."""

    asynchrony: destack._generated.dir.tree.node.Asynchrony
    operator: ForEachOperator
    binding: ForEachBinding
    iterator: destack._generated.dir.tree.node.LocalNodeId
    body: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["forEach"] = "forEach"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionFor(ExpressionImpl):
    """A For is a for loop with the traditional three-part (initialization, condition, increment)."""

    initialization: destack._generated.dir.tree.node.LocalNodeId | None
    condition: destack._generated.dir.tree.node.LocalNodeId | None
    increment: destack._generated.dir.tree.node.LocalNodeId | None
    body: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["for"] = "for"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionLoop(ExpressionImpl):
    """A Loop is an unconditional loop."""

    body: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["loop"] = "loop"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionTry(ExpressionImpl):
    """A Try is a try/catch/finally expression."""

    body: destack._generated.dir.tree.node.LocalNodeId
    catch: destack._generated.dir.tree.node.LocalNodeId | None
    finally_: destack._generated.dir.tree.node.LocalNodeId | None
    kind: typing.Literal["try"] = "try"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionMatch(ExpressionImpl):
    """A Match is a match expression with case patterns."""

    form: destack._generated.dir.tree.match.MatchForm
    value: destack._generated.dir.tree.node.LocalNodeId
    cases: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["match"] = "match"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionBreak(ExpressionImpl):
    """A break statement."""

    label: destack._generated.core.string.StringId | None
    value: destack._generated.dir.tree.node.LocalNodeId | None
    kind: typing.Literal["break"] = "break"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionContinue(ExpressionImpl):
    """A continue statement."""

    label: destack._generated.core.string.StringId | None
    kind: typing.Literal["continue"] = "continue"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionAwait(ExpressionImpl):
    """Await an expression."""

    expression: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["await"] = "await"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionAwaitMaybe(ExpressionImpl):
    """Await an expression with immediate error propagation (`await? expr`)."""

    expression: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["awaitMaybe"] = "awaitMaybe"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionAwaitMust(ExpressionImpl):
    """Await an expression with immediate trapping error propagation (`await! expr`)."""

    expression: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["awaitMust"] = "awaitMust"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionYield(ExpressionImpl):
    """Yield an expression."""

    cardinality: YieldCardinality
    value: destack._generated.dir.tree.node.LocalNodeId | None
    kind: typing.Literal["yield"] = "yield"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionThrow(ExpressionImpl):
    """Throw an expression."""

    value: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["throw"] = "throw"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionReturn(ExpressionImpl):
    """Return an expression."""

    value: destack._generated.dir.tree.node.LocalNodeId | None
    kind: typing.Literal["return"] = "return"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionIdentifier(ExpressionImpl):
    """Bare identifier reference."""

    name: destack._generated.core.string.StringId
    kind: typing.Literal["identifier"] = "identifier"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionPrivateIdentifier(ExpressionImpl):
    """Private identifier (JavaScript/TypeScript)."""

    name: destack._generated.core.string.StringId
    kind: typing.Literal["privateIdentifier"] = "privateIdentifier"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionThis(ExpressionImpl):
    """This reference (value or type context)."""

    kind: typing.Literal["this"] = "this"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionSuper(ExpressionImpl):
    """Super reference (value context)."""

    kind: typing.Literal["super"] = "super"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionImportMeta(ExpressionImpl):
    """Import meta intrinsic value."""

    kind: typing.Literal["importMeta"] = "importMeta"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionImportSource(ExpressionImpl):
    """Import source intrinsic value."""

    kind: typing.Literal["importSource"] = "importSource"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionScalarLiteral(ExpressionImpl):
    """Literal scalar value."""

    scalar_literal: destack._generated.dir.tree.literal.ScalarLiteral
    kind: typing.Literal["scalarLiteral"] = "scalarLiteral"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionRangeExpression(ExpressionImpl):
    """Range expression."""

    start: destack._generated.dir.tree.node.LocalNodeId | None
    end: destack._generated.dir.tree.node.LocalNodeId | None
    end_kind: destack._generated.dir.tree.operator.RangeEnd
    kind: typing.Literal["rangeExpression"] = "rangeExpression"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionTemplateExpression(ExpressionImpl):
    """Template expression. May include interpolation arguments."""

    value: destack._generated.dir.tree.literal.TemplateLiteral
    kind: typing.Literal["templateExpression"] = "templateExpression"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionTaggedTemplateExpression(ExpressionImpl):
    """Tagged template expression. May include interpolation arguments."""

    tag: destack._generated.dir.tree.node.LocalNodeId
    generic_arguments: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    value: destack._generated.dir.tree.literal.TemplateLiteral
    kind: typing.Literal["taggedTemplateExpression"] = "taggedTemplateExpression"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionArrayExpression(ExpressionImpl):
    """An ArrayExpression constructs an array of homogeneous elements."""

    elements: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["arrayExpression"] = "arrayExpression"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionFixedArrayExpression(ExpressionImpl):
    """A FixedArrayExpression constructs a fixed-length array by repeating one value."""

    # the repeated value expression
    value: destack._generated.dir.tree.node.LocalNodeId
    # the fixed array length expression
    length: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["fixedArrayExpression"] = "fixedArrayExpression"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionTupleExpression(ExpressionImpl):
    """A TupleExpression constructs an anonymous tuple of heterogeneous elements."""

    elements: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["tupleExpression"] = "tupleExpression"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionSequenceExpression(ExpressionImpl):
    """A SequenceExpression is the JavaScript/TypeScript comma operator."""

    expressions: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["sequenceExpression"] = "sequenceExpression"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionObjectExpression(ExpressionImpl):
    """An ObjectExpression constructs an object with heterogeneous fields."""

    properties: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["objectExpression"] = "objectExpression"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionStructExpression(ExpressionImpl):
    """A StructExpression constructs a nominal value with named fields."""

    ty: destack._generated.dir.tree.node.LocalNodeId
    properties: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["structExpression"] = "structExpression"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionTreeExpression(ExpressionImpl):
    """A TreeExpression constructs a tree fragment with attributes and children."""

    left: destack._generated.dir.tree.node.LocalNodeId | None
    generic_arguments: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    attributes: Sequence[destack._generated.dir.tree.node.LocalNodeId] | None
    children: Sequence[destack._generated.dir.tree.node.LocalNodeId] | None
    kind: typing.Literal["treeExpression"] = "treeExpression"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionParenthesized(ExpressionImpl):
    """Parenthesized expression."""

    expression: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["parenthesized"] = "parenthesized"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionType(ExpressionImpl):
    """Type expression used as a runtime type value."""

    value: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["type"] = "type"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionComptime(ExpressionImpl):
    """Compile time evaluated expression."""

    body: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["comptime"] = "comptime"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionAs(ExpressionImpl):
    """TypeScript-style `as` assertion."""

    expression: destack._generated.dir.tree.node.LocalNodeId
    target_type: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["as"] = "as"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionSatisfies(ExpressionImpl):
    """TypeScript-style `satisfies` expression."""

    expression: destack._generated.dir.tree.node.LocalNodeId
    target_type: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["satisfies"] = "satisfies"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionIs(ExpressionImpl):
    """Runtime type guard."""

    value: destack._generated.dir.tree.node.LocalNodeId
    target_type: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["is"] = "is"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionInstanceOf(ExpressionImpl):
    """Runtime constructor guard."""

    value: destack._generated.dir.tree.node.LocalNodeId
    target: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["instanceOf"] = "instanceOf"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionUnary(ExpressionImpl):
    """Unary operation (prefix or postfix)."""

    operator: destack._generated.dir.tree.operator.UnaryOperator
    right: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["unary"] = "unary"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionMoveOf(ExpressionImpl):
    """Move operation (e.g., `^x`)."""

    mutability: destack._generated.dir.tree.node.Mutability | None
    variance: VarianceBound | None
    right: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["moveOf"] = "moveOf"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionBorrowOf(ExpressionImpl):
    """Borrow operation (e.g., `&x`)."""

    mutability: destack._generated.dir.tree.node.Mutability | None
    variance: VarianceBound | None
    right: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["borrowOf"] = "borrowOf"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionMember(ExpressionImpl):
    """Member access."""

    left: destack._generated.dir.tree.node.LocalNodeId
    name: destack._generated.core.string.StringId | None
    kind: typing.Literal["member"] = "member"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionPrivateMember(ExpressionImpl):
    """Private member access."""

    left: destack._generated.dir.tree.node.LocalNodeId
    name: destack._generated.core.string.StringId | None
    kind: typing.Literal["privateMember"] = "privateMember"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionIndex(ExpressionImpl):
    """Index into a receiver expression."""

    position: PostfixPosition
    left: destack._generated.dir.tree.node.LocalNodeId
    index: destack._generated.dir.tree.node.LocalNodeId | None
    kind: typing.Literal["index"] = "index"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionInstantiation(ExpressionImpl):
    """Instantiation expression (TypeScript)."""

    left: destack._generated.dir.tree.node.LocalNodeId
    generic_arguments: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["instantiation"] = "instantiation"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionCall(ExpressionImpl):
    """A Call is call to a function OR an instantiation of a tuple type."""

    position: PostfixPosition
    left: destack._generated.dir.tree.node.LocalNodeId
    generic_arguments: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    arguments: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["call"] = "call"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionNew(ExpressionImpl):
    """New constructor call."""

    ty: destack._generated.dir.tree.node.LocalNodeId
    arguments: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["new"] = "new"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionNewMaybe(ExpressionImpl):
    """Fallible new constructor call with immediate allocation failure propagation."""

    ty: destack._generated.dir.tree.node.LocalNodeId
    arguments: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["newMaybe"] = "newMaybe"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionMaybe(ExpressionImpl):
    """Maybe unwrap an expression with `?` and propagate."""

    position: PostfixPosition
    left: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["maybe"] = "maybe"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionMust(ExpressionImpl):
    """Force unwrap an expression with `!` and propagate."""

    position: PostfixPosition
    left: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["must"] = "must"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionBinary(ExpressionImpl):
    """Binary operation."""

    left: destack._generated.dir.tree.node.LocalNodeId
    operator: destack._generated.dir.tree.operator.BinaryOperator
    right: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["binary"] = "binary"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionAssign(ExpressionImpl):
    """Assignment operation."""

    left: destack._generated.dir.tree.node.LocalNodeId
    operator: destack._generated.dir.tree.operator.AssignOperator
    right: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["assign"] = "assign"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionDebugger(ExpressionImpl):
    """Debugger statement."""

    kind: typing.Literal["debugger"] = "debugger"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionMissing(ExpressionImpl):
    """Missing expression child."""

    kind: typing.Literal["missing"] = "missing"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionStub(ExpressionImpl):
    """Stub placeholder."""

    kind: typing.Literal["stub"] = "stub"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


@dataclass(frozen=True, slots=True)
class ExpressionError(ExpressionImpl):
    """Error placeholder."""

    kind: typing.Literal["error"] = "error"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_expression(self)


"""An Expression is a generic container for all constructs."""
Expression: typing.TypeAlias = (
    ExpressionDeclaration
    | ExpressionBlock
    | ExpressionLabel
    | ExpressionImport
    | ExpressionExport
    | ExpressionLet
    | ExpressionLetElse
    | ExpressionUsing
    | ExpressionIf
    | ExpressionWhile
    | ExpressionForEach
    | ExpressionFor
    | ExpressionLoop
    | ExpressionTry
    | ExpressionMatch
    | ExpressionBreak
    | ExpressionContinue
    | ExpressionAwait
    | ExpressionAwaitMaybe
    | ExpressionAwaitMust
    | ExpressionYield
    | ExpressionThrow
    | ExpressionReturn
    | ExpressionIdentifier
    | ExpressionPrivateIdentifier
    | ExpressionThis
    | ExpressionSuper
    | ExpressionImportMeta
    | ExpressionImportSource
    | ExpressionScalarLiteral
    | ExpressionRangeExpression
    | ExpressionTemplateExpression
    | ExpressionTaggedTemplateExpression
    | ExpressionArrayExpression
    | ExpressionFixedArrayExpression
    | ExpressionTupleExpression
    | ExpressionSequenceExpression
    | ExpressionObjectExpression
    | ExpressionStructExpression
    | ExpressionTreeExpression
    | ExpressionParenthesized
    | ExpressionType
    | ExpressionComptime
    | ExpressionAs
    | ExpressionSatisfies
    | ExpressionIs
    | ExpressionInstanceOf
    | ExpressionUnary
    | ExpressionMoveOf
    | ExpressionBorrowOf
    | ExpressionMember
    | ExpressionPrivateMember
    | ExpressionIndex
    | ExpressionInstantiation
    | ExpressionCall
    | ExpressionNew
    | ExpressionNewMaybe
    | ExpressionMaybe
    | ExpressionMust
    | ExpressionBinary
    | ExpressionAssign
    | ExpressionDebugger
    | ExpressionMissing
    | ExpressionStub
    | ExpressionError
)


def encode_expression(writer: BinaryWriter, value: Expression) -> None:
    """Encode one Expression."""
    if value.kind == "declaration":
        writer.write_unsigned(0)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.declaration)
    elif value.kind == "block":
        writer.write_unsigned(1)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.block)
    elif value.kind == "label":
        writer.write_unsigned(2)
        destack._generated.core.string.encode_string_id(writer, value.label)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.body)
    elif value.kind == "import":
        writer.write_unsigned(3)
        destack._generated.dir.tree.dependency.encode_dependency_form(
            writer, value.form
        )
        destack._generated.core.string.encode_string_id(writer, value.target)
        if value.items is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            writer.write_unsigned(len(value.items))
            for item_value_items_1 in value.items:
                destack._generated.dir.tree.node.encode_local_node_id(
                    writer, item_value_items_1
                )
        if value.attributes is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.import_.encode_import_attribute_clause(
                writer, value.attributes
            )
    elif value.kind == "export":
        writer.write_unsigned(4)
        destack._generated.dir.tree.dependency.encode_dependency_form(
            writer, value.form
        )
        if value.target is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.core.string.encode_string_id(writer, value.target)
        writer.write_unsigned(len(value.items))
        for item_value_items_0 in value.items:
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, item_value_items_0
            )
        if value.attributes is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.import_.encode_import_attribute_clause(
                writer, value.attributes
            )
    elif value.kind == "let":
        writer.write_unsigned(5)
        encode_let_kind(writer, value.kind_value)
        if value.export is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.dependency.encode_export_kind(
                writer, value.export
            )
        destack._generated.dir.tree.node.encode_mutability(writer, value.mutability)
        writer.write_unsigned(len(value.declarators))
        for item_value_declarators_0 in value.declarators:
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, item_value_declarators_0
            )
        writer.write_bool(value.is_ambient)
        if value.place is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.declaration.encode_place_modifier(
                writer, value.place
            )
    elif value.kind == "letElse":
        writer.write_unsigned(6)
        encode_let_kind(writer, value.kind_value)
        destack._generated.dir.tree.node.encode_mutability(writer, value.mutability)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.declarator)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.else_branch)
    elif value.kind == "using":
        writer.write_unsigned(7)
        destack._generated.dir.tree.node.encode_asynchrony(writer, value.asynchrony)
        if value.export is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.dependency.encode_export_kind(
                writer, value.export
            )
        writer.write_unsigned(len(value.declarators))
        for item_value_declarators_0 in value.declarators:
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, item_value_declarators_0
            )
        writer.write_bool(value.is_ambient)
    elif value.kind == "if":
        writer.write_unsigned(8)
        encode_if_form(writer, value.form)
        encode_condition(writer, value.condition)
        destack._generated.dir.tree.node.encode_local_node_id(
            writer, value.then_expression
        )
        if value.else_expression is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, value.else_expression
            )
    elif value.kind == "while":
        writer.write_unsigned(9)
        encode_while_form(writer, value.form)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.condition)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.body)
    elif value.kind == "forEach":
        writer.write_unsigned(10)
        destack._generated.dir.tree.node.encode_asynchrony(writer, value.asynchrony)
        encode_for_each_operator(writer, value.operator)
        encode_for_each_binding(writer, value.binding)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.iterator)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.body)
    elif value.kind == "for":
        writer.write_unsigned(11)
        if value.initialization is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, value.initialization
            )
        if value.condition is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, value.condition
            )
        if value.increment is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, value.increment
            )
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.body)
    elif value.kind == "loop":
        writer.write_unsigned(12)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.body)
    elif value.kind == "try":
        writer.write_unsigned(13)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.body)
        if value.catch is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_local_node_id(writer, value.catch)
        if value.finally_ is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, value.finally_
            )
    elif value.kind == "match":
        writer.write_unsigned(14)
        destack._generated.dir.tree.match.encode_match_form(writer, value.form)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.value)
        writer.write_unsigned(len(value.cases))
        for item_value_cases_0 in value.cases:
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, item_value_cases_0
            )
    elif value.kind == "break":
        writer.write_unsigned(15)
        if value.label is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.core.string.encode_string_id(writer, value.label)
        if value.value is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_local_node_id(writer, value.value)
    elif value.kind == "continue":
        writer.write_unsigned(16)
        if value.label is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.core.string.encode_string_id(writer, value.label)
    elif value.kind == "await":
        writer.write_unsigned(17)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.expression)
    elif value.kind == "awaitMaybe":
        writer.write_unsigned(18)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.expression)
    elif value.kind == "awaitMust":
        writer.write_unsigned(19)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.expression)
    elif value.kind == "yield":
        writer.write_unsigned(20)
        encode_yield_cardinality(writer, value.cardinality)
        if value.value is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_local_node_id(writer, value.value)
    elif value.kind == "throw":
        writer.write_unsigned(21)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.value)
    elif value.kind == "return":
        writer.write_unsigned(22)
        if value.value is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_local_node_id(writer, value.value)
    elif value.kind == "identifier":
        writer.write_unsigned(23)
        destack._generated.core.string.encode_string_id(writer, value.name)
    elif value.kind == "privateIdentifier":
        writer.write_unsigned(24)
        destack._generated.core.string.encode_string_id(writer, value.name)
    elif value.kind == "this":
        writer.write_unsigned(25)
    elif value.kind == "super":
        writer.write_unsigned(26)
    elif value.kind == "importMeta":
        writer.write_unsigned(27)
    elif value.kind == "importSource":
        writer.write_unsigned(28)
    elif value.kind == "scalarLiteral":
        writer.write_unsigned(29)
        destack._generated.dir.tree.literal.encode_scalar_literal(
            writer, value.scalar_literal
        )
    elif value.kind == "rangeExpression":
        writer.write_unsigned(30)
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
    elif value.kind == "templateExpression":
        writer.write_unsigned(31)
        destack._generated.dir.tree.literal.encode_template_literal(writer, value.value)
    elif value.kind == "taggedTemplateExpression":
        writer.write_unsigned(32)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.tag)
        writer.write_unsigned(len(value.generic_arguments))
        for item_value_generic_arguments_0 in value.generic_arguments:
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, item_value_generic_arguments_0
            )
        destack._generated.dir.tree.literal.encode_template_literal(writer, value.value)
    elif value.kind == "arrayExpression":
        writer.write_unsigned(33)
        writer.write_unsigned(len(value.elements))
        for item_value_elements_0 in value.elements:
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, item_value_elements_0
            )
    elif value.kind == "fixedArrayExpression":
        writer.write_unsigned(34)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.value)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.length)
    elif value.kind == "tupleExpression":
        writer.write_unsigned(35)
        writer.write_unsigned(len(value.elements))
        for item_value_elements_0 in value.elements:
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, item_value_elements_0
            )
    elif value.kind == "sequenceExpression":
        writer.write_unsigned(36)
        writer.write_unsigned(len(value.expressions))
        for item_value_expressions_0 in value.expressions:
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, item_value_expressions_0
            )
    elif value.kind == "objectExpression":
        writer.write_unsigned(37)
        writer.write_unsigned(len(value.properties))
        for item_value_properties_0 in value.properties:
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, item_value_properties_0
            )
    elif value.kind == "structExpression":
        writer.write_unsigned(38)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.ty)
        writer.write_unsigned(len(value.properties))
        for item_value_properties_0 in value.properties:
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, item_value_properties_0
            )
    elif value.kind == "treeExpression":
        writer.write_unsigned(39)
        if value.left is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_local_node_id(writer, value.left)
        writer.write_unsigned(len(value.generic_arguments))
        for item_value_generic_arguments_0 in value.generic_arguments:
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, item_value_generic_arguments_0
            )
        if value.attributes is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            writer.write_unsigned(len(value.attributes))
            for item_value_attributes_1 in value.attributes:
                destack._generated.dir.tree.node.encode_local_node_id(
                    writer, item_value_attributes_1
                )
        if value.children is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            writer.write_unsigned(len(value.children))
            for item_value_children_1 in value.children:
                destack._generated.dir.tree.node.encode_local_node_id(
                    writer, item_value_children_1
                )
    elif value.kind == "parenthesized":
        writer.write_unsigned(40)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.expression)
    elif value.kind == "type":
        writer.write_unsigned(41)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.value)
    elif value.kind == "comptime":
        writer.write_unsigned(42)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.body)
    elif value.kind == "as":
        writer.write_unsigned(43)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.expression)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.target_type)
    elif value.kind == "satisfies":
        writer.write_unsigned(44)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.expression)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.target_type)
    elif value.kind == "is":
        writer.write_unsigned(45)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.value)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.target_type)
    elif value.kind == "instanceOf":
        writer.write_unsigned(46)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.value)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.target)
    elif value.kind == "unary":
        writer.write_unsigned(47)
        destack._generated.dir.tree.operator.encode_unary_operator(
            writer, value.operator
        )
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.right)
    elif value.kind == "moveOf":
        writer.write_unsigned(48)
        if value.mutability is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_mutability(writer, value.mutability)
        if value.variance is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            encode_variance_bound(writer, value.variance)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.right)
    elif value.kind == "borrowOf":
        writer.write_unsigned(49)
        if value.mutability is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_mutability(writer, value.mutability)
        if value.variance is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            encode_variance_bound(writer, value.variance)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.right)
    elif value.kind == "member":
        writer.write_unsigned(50)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.left)
        if value.name is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.core.string.encode_string_id(writer, value.name)
    elif value.kind == "privateMember":
        writer.write_unsigned(51)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.left)
        if value.name is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.core.string.encode_string_id(writer, value.name)
    elif value.kind == "index":
        writer.write_unsigned(52)
        encode_postfix_position(writer, value.position)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.left)
        if value.index is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_local_node_id(writer, value.index)
    elif value.kind == "instantiation":
        writer.write_unsigned(53)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.left)
        writer.write_unsigned(len(value.generic_arguments))
        for item_value_generic_arguments_0 in value.generic_arguments:
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, item_value_generic_arguments_0
            )
    elif value.kind == "call":
        writer.write_unsigned(54)
        encode_postfix_position(writer, value.position)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.left)
        writer.write_unsigned(len(value.generic_arguments))
        for item_value_generic_arguments_0 in value.generic_arguments:
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, item_value_generic_arguments_0
            )
        writer.write_unsigned(len(value.arguments))
        for item_value_arguments_0 in value.arguments:
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, item_value_arguments_0
            )
    elif value.kind == "new":
        writer.write_unsigned(55)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.ty)
        writer.write_unsigned(len(value.arguments))
        for item_value_arguments_0 in value.arguments:
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, item_value_arguments_0
            )
    elif value.kind == "newMaybe":
        writer.write_unsigned(56)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.ty)
        writer.write_unsigned(len(value.arguments))
        for item_value_arguments_0 in value.arguments:
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, item_value_arguments_0
            )
    elif value.kind == "maybe":
        writer.write_unsigned(57)
        encode_postfix_position(writer, value.position)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.left)
    elif value.kind == "must":
        writer.write_unsigned(58)
        encode_postfix_position(writer, value.position)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.left)
    elif value.kind == "binary":
        writer.write_unsigned(59)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.left)
        destack._generated.dir.tree.operator.encode_binary_operator(
            writer, value.operator
        )
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.right)
    elif value.kind == "assign":
        writer.write_unsigned(60)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.left)
        destack._generated.dir.tree.operator.encode_assign_operator(
            writer, value.operator
        )
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.right)
    elif value.kind == "debugger":
        writer.write_unsigned(61)
    elif value.kind == "missing":
        writer.write_unsigned(62)
    elif value.kind == "stub":
        writer.write_unsigned(63)
    elif value.kind == "error":
        writer.write_unsigned(64)
    else:
        raise SerdeError("unknown enum variant")


def decode_expression(reader: BinaryReader) -> Expression:
    """Decode one Expression."""
    variant = reader.read_number()

    if variant == 0:
        declaration = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return ExpressionDeclaration(declaration=declaration)
    elif variant == 1:
        block = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return ExpressionBlock(block=block)
    elif variant == 2:
        label = destack._generated.core.string.decode_string_id(reader)
        body = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return ExpressionLabel(
            label=label,
            body=body,
        )
    elif variant == 3:
        form = destack._generated.dir.tree.dependency.decode_dependency_form(reader)
        target = destack._generated.core.string.decode_string_id(reader)
        items = reader.read_option(
            lambda: [
                destack._generated.dir.tree.node.decode_local_node_id(reader)
                for _ in range(reader.read_number())
            ]
        )
        attributes = reader.read_option(
            lambda: destack._generated.dir.tree.import_.decode_import_attribute_clause(
                reader
            )
        )

        return ExpressionImport(
            form=form,
            target=target,
            items=items,
            attributes=attributes,
        )
    elif variant == 4:
        form = destack._generated.dir.tree.dependency.decode_dependency_form(reader)
        target = reader.read_option(
            lambda: destack._generated.core.string.decode_string_id(reader)
        )
        items = [
            destack._generated.dir.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]
        attributes = reader.read_option(
            lambda: destack._generated.dir.tree.import_.decode_import_attribute_clause(
                reader
            )
        )

        return ExpressionExport(
            form=form,
            target=target,
            items=items,
            attributes=attributes,
        )
    elif variant == 5:
        kind_value = decode_let_kind(reader)
        export = reader.read_option(
            lambda: destack._generated.dir.tree.dependency.decode_export_kind(reader)
        )
        mutability = destack._generated.dir.tree.node.decode_mutability(reader)
        declarators = [
            destack._generated.dir.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]
        is_ambient = reader.read_bool()
        place = reader.read_option(
            lambda: destack._generated.dir.tree.declaration.decode_place_modifier(
                reader
            )
        )

        return ExpressionLet(
            kind_value=kind_value,
            export=export,
            mutability=mutability,
            declarators=declarators,
            is_ambient=is_ambient,
            place=place,
        )
    elif variant == 6:
        kind_value = decode_let_kind(reader)
        mutability = destack._generated.dir.tree.node.decode_mutability(reader)
        declarator = destack._generated.dir.tree.node.decode_local_node_id(reader)
        else_branch = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return ExpressionLetElse(
            kind_value=kind_value,
            mutability=mutability,
            declarator=declarator,
            else_branch=else_branch,
        )
    elif variant == 7:
        asynchrony = destack._generated.dir.tree.node.decode_asynchrony(reader)
        export = reader.read_option(
            lambda: destack._generated.dir.tree.dependency.decode_export_kind(reader)
        )
        declarators = [
            destack._generated.dir.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]
        is_ambient = reader.read_bool()

        return ExpressionUsing(
            asynchrony=asynchrony,
            export=export,
            declarators=declarators,
            is_ambient=is_ambient,
        )
    elif variant == 8:
        form = decode_if_form(reader)
        condition = decode_condition(reader)
        then_expression = destack._generated.dir.tree.node.decode_local_node_id(reader)
        else_expression = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )

        return ExpressionIf(
            form=form,
            condition=condition,
            then_expression=then_expression,
            else_expression=else_expression,
        )
    elif variant == 9:
        form = decode_while_form(reader)
        condition = destack._generated.dir.tree.node.decode_local_node_id(reader)
        body = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return ExpressionWhile(
            form=form,
            condition=condition,
            body=body,
        )
    elif variant == 10:
        asynchrony = destack._generated.dir.tree.node.decode_asynchrony(reader)
        operator = decode_for_each_operator(reader)
        binding = decode_for_each_binding(reader)
        iterator = destack._generated.dir.tree.node.decode_local_node_id(reader)
        body = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return ExpressionForEach(
            asynchrony=asynchrony,
            operator=operator,
            binding=binding,
            iterator=iterator,
            body=body,
        )
    elif variant == 11:
        initialization = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )
        condition = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )
        increment = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )
        body = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return ExpressionFor(
            initialization=initialization,
            condition=condition,
            increment=increment,
            body=body,
        )
    elif variant == 12:
        body = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return ExpressionLoop(
            body=body,
        )
    elif variant == 13:
        body = destack._generated.dir.tree.node.decode_local_node_id(reader)
        catch = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )
        finally_ = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )

        return ExpressionTry(
            body=body,
            catch=catch,
            finally_=finally_,
        )
    elif variant == 14:
        form = destack._generated.dir.tree.match.decode_match_form(reader)
        value_ = destack._generated.dir.tree.node.decode_local_node_id(reader)
        cases = [
            destack._generated.dir.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return ExpressionMatch(
            form=form,
            value=value_,
            cases=cases,
        )
    elif variant == 15:
        label = reader.read_option(
            lambda: destack._generated.core.string.decode_string_id(reader)
        )
        value_ = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )

        return ExpressionBreak(
            label=label,
            value=value_,
        )
    elif variant == 16:
        label = reader.read_option(
            lambda: destack._generated.core.string.decode_string_id(reader)
        )

        return ExpressionContinue(
            label=label,
        )
    elif variant == 17:
        expression = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return ExpressionAwait(
            expression=expression,
        )
    elif variant == 18:
        expression = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return ExpressionAwaitMaybe(
            expression=expression,
        )
    elif variant == 19:
        expression = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return ExpressionAwaitMust(
            expression=expression,
        )
    elif variant == 20:
        cardinality = decode_yield_cardinality(reader)
        value_ = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )

        return ExpressionYield(
            cardinality=cardinality,
            value=value_,
        )
    elif variant == 21:
        value_ = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return ExpressionThrow(
            value=value_,
        )
    elif variant == 22:
        value_ = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )

        return ExpressionReturn(
            value=value_,
        )
    elif variant == 23:
        name = destack._generated.core.string.decode_string_id(reader)

        return ExpressionIdentifier(
            name=name,
        )
    elif variant == 24:
        name = destack._generated.core.string.decode_string_id(reader)

        return ExpressionPrivateIdentifier(
            name=name,
        )
    elif variant == 25:
        return ExpressionThis()
    elif variant == 26:
        return ExpressionSuper()
    elif variant == 27:
        return ExpressionImportMeta()
    elif variant == 28:
        return ExpressionImportSource()
    elif variant == 29:
        scalar_literal = destack._generated.dir.tree.literal.decode_scalar_literal(
            reader
        )

        return ExpressionScalarLiteral(scalar_literal=scalar_literal)
    elif variant == 30:
        start = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )
        end = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )
        end_kind = destack._generated.dir.tree.operator.decode_range_end(reader)

        return ExpressionRangeExpression(
            start=start,
            end=end,
            end_kind=end_kind,
        )
    elif variant == 31:
        value_ = destack._generated.dir.tree.literal.decode_template_literal(reader)

        return ExpressionTemplateExpression(
            value=value_,
        )
    elif variant == 32:
        tag = destack._generated.dir.tree.node.decode_local_node_id(reader)
        generic_arguments = [
            destack._generated.dir.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]
        value_ = destack._generated.dir.tree.literal.decode_template_literal(reader)

        return ExpressionTaggedTemplateExpression(
            tag=tag,
            generic_arguments=generic_arguments,
            value=value_,
        )
    elif variant == 33:
        elements = [
            destack._generated.dir.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return ExpressionArrayExpression(
            elements=elements,
        )
    elif variant == 34:
        value_ = destack._generated.dir.tree.node.decode_local_node_id(reader)
        length = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return ExpressionFixedArrayExpression(
            value=value_,
            length=length,
        )
    elif variant == 35:
        elements = [
            destack._generated.dir.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return ExpressionTupleExpression(
            elements=elements,
        )
    elif variant == 36:
        expressions = [
            destack._generated.dir.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return ExpressionSequenceExpression(
            expressions=expressions,
        )
    elif variant == 37:
        properties = [
            destack._generated.dir.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return ExpressionObjectExpression(
            properties=properties,
        )
    elif variant == 38:
        ty = destack._generated.dir.tree.node.decode_local_node_id(reader)
        properties = [
            destack._generated.dir.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return ExpressionStructExpression(
            ty=ty,
            properties=properties,
        )
    elif variant == 39:
        left = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )
        generic_arguments = [
            destack._generated.dir.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]
        attributes = reader.read_option(
            lambda: [
                destack._generated.dir.tree.node.decode_local_node_id(reader)
                for _ in range(reader.read_number())
            ]
        )
        children = reader.read_option(
            lambda: [
                destack._generated.dir.tree.node.decode_local_node_id(reader)
                for _ in range(reader.read_number())
            ]
        )

        return ExpressionTreeExpression(
            left=left,
            generic_arguments=generic_arguments,
            attributes=attributes,
            children=children,
        )
    elif variant == 40:
        expression = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return ExpressionParenthesized(
            expression=expression,
        )
    elif variant == 41:
        value_ = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return ExpressionType(
            value=value_,
        )
    elif variant == 42:
        body = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return ExpressionComptime(
            body=body,
        )
    elif variant == 43:
        expression = destack._generated.dir.tree.node.decode_local_node_id(reader)
        target_type = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return ExpressionAs(
            expression=expression,
            target_type=target_type,
        )
    elif variant == 44:
        expression = destack._generated.dir.tree.node.decode_local_node_id(reader)
        target_type = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return ExpressionSatisfies(
            expression=expression,
            target_type=target_type,
        )
    elif variant == 45:
        value_ = destack._generated.dir.tree.node.decode_local_node_id(reader)
        target_type = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return ExpressionIs(
            value=value_,
            target_type=target_type,
        )
    elif variant == 46:
        value_ = destack._generated.dir.tree.node.decode_local_node_id(reader)
        target = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return ExpressionInstanceOf(
            value=value_,
            target=target,
        )
    elif variant == 47:
        operator = destack._generated.dir.tree.operator.decode_unary_operator(reader)
        right = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return ExpressionUnary(
            operator=operator,
            right=right,
        )
    elif variant == 48:
        mutability = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_mutability(reader)
        )
        variance = reader.read_option(lambda: decode_variance_bound(reader))
        right = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return ExpressionMoveOf(
            mutability=mutability,
            variance=variance,
            right=right,
        )
    elif variant == 49:
        mutability = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_mutability(reader)
        )
        variance = reader.read_option(lambda: decode_variance_bound(reader))
        right = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return ExpressionBorrowOf(
            mutability=mutability,
            variance=variance,
            right=right,
        )
    elif variant == 50:
        left = destack._generated.dir.tree.node.decode_local_node_id(reader)
        name = reader.read_option(
            lambda: destack._generated.core.string.decode_string_id(reader)
        )

        return ExpressionMember(
            left=left,
            name=name,
        )
    elif variant == 51:
        left = destack._generated.dir.tree.node.decode_local_node_id(reader)
        name = reader.read_option(
            lambda: destack._generated.core.string.decode_string_id(reader)
        )

        return ExpressionPrivateMember(
            left=left,
            name=name,
        )
    elif variant == 52:
        position = decode_postfix_position(reader)
        left = destack._generated.dir.tree.node.decode_local_node_id(reader)
        index = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )

        return ExpressionIndex(
            position=position,
            left=left,
            index=index,
        )
    elif variant == 53:
        left = destack._generated.dir.tree.node.decode_local_node_id(reader)
        generic_arguments = [
            destack._generated.dir.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return ExpressionInstantiation(
            left=left,
            generic_arguments=generic_arguments,
        )
    elif variant == 54:
        position = decode_postfix_position(reader)
        left = destack._generated.dir.tree.node.decode_local_node_id(reader)
        generic_arguments = [
            destack._generated.dir.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]
        arguments = [
            destack._generated.dir.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return ExpressionCall(
            position=position,
            left=left,
            generic_arguments=generic_arguments,
            arguments=arguments,
        )
    elif variant == 55:
        ty = destack._generated.dir.tree.node.decode_local_node_id(reader)
        arguments = [
            destack._generated.dir.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return ExpressionNew(
            ty=ty,
            arguments=arguments,
        )
    elif variant == 56:
        ty = destack._generated.dir.tree.node.decode_local_node_id(reader)
        arguments = [
            destack._generated.dir.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return ExpressionNewMaybe(
            ty=ty,
            arguments=arguments,
        )
    elif variant == 57:
        position = decode_postfix_position(reader)
        left = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return ExpressionMaybe(
            position=position,
            left=left,
        )
    elif variant == 58:
        position = decode_postfix_position(reader)
        left = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return ExpressionMust(
            position=position,
            left=left,
        )
    elif variant == 59:
        left = destack._generated.dir.tree.node.decode_local_node_id(reader)
        operator = destack._generated.dir.tree.operator.decode_binary_operator(reader)
        right = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return ExpressionBinary(
            left=left,
            operator=operator,
            right=right,
        )
    elif variant == 60:
        left = destack._generated.dir.tree.node.decode_local_node_id(reader)
        operator = destack._generated.dir.tree.operator.decode_assign_operator(reader)
        right = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return ExpressionAssign(
            left=left,
            operator=operator,
            right=right,
        )
    elif variant == 61:
        return ExpressionDebugger()
    elif variant == 62:
        return ExpressionMissing()
    elif variant == 63:
        return ExpressionStub()
    elif variant == 64:
        return ExpressionError()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_expression(value: Expression) -> Json:
    """Return one JSON value for one Expression."""
    if value.kind == "declaration":
        return {
            "kind": "declaration",
            "declaration": destack._generated.dir.tree.node.to_json_local_node_id(
                value.declaration
            ),
        }
    elif value.kind == "block":
        return {
            "kind": "block",
            "block": destack._generated.dir.tree.node.to_json_local_node_id(
                value.block
            ),
        }
    elif value.kind == "label":
        return {
            "kind": "label",
            "label": destack._generated.core.string.to_json_string_id(value.label),
            "body": destack._generated.dir.tree.node.to_json_local_node_id(value.body),
        }
    elif value.kind == "import":
        return {
            "kind": "import",
            "form": destack._generated.dir.tree.dependency.to_json_dependency_form(
                value.form
            ),
            "target": destack._generated.core.string.to_json_string_id(value.target),
            **(
                {}
                if value.items is None
                else {
                    "items": [
                        destack._generated.dir.tree.node.to_json_local_node_id(item_0)
                        for item_0 in value.items
                    ]
                }
            ),
            **(
                {}
                if value.attributes is None
                else {
                    "attributes": destack._generated.dir.tree.import_.to_json_import_attribute_clause(
                        value.attributes
                    )
                }
            ),
        }
    elif value.kind == "export":
        return {
            "kind": "export",
            "form": destack._generated.dir.tree.dependency.to_json_dependency_form(
                value.form
            ),
            **(
                {}
                if value.target is None
                else {
                    "target": destack._generated.core.string.to_json_string_id(
                        value.target
                    )
                }
            ),
            "items": [
                destack._generated.dir.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.items
            ],
            **(
                {}
                if value.attributes is None
                else {
                    "attributes": destack._generated.dir.tree.import_.to_json_import_attribute_clause(
                        value.attributes
                    )
                }
            ),
        }
    elif value.kind == "let":
        return {
            "kind": "let",
            "kind": to_json_let_kind(value.kind_value),
            **(
                {}
                if value.export is None
                else {
                    "export": destack._generated.dir.tree.dependency.to_json_export_kind(
                        value.export
                    )
                }
            ),
            "mutability": destack._generated.dir.tree.node.to_json_mutability(
                value.mutability
            ),
            "declarators": [
                destack._generated.dir.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.declarators
            ],
            "isAmbient": value.is_ambient,
            **(
                {}
                if value.place is None
                else {
                    "place": destack._generated.dir.tree.declaration.to_json_place_modifier(
                        value.place
                    )
                }
            ),
        }
    elif value.kind == "letElse":
        return {
            "kind": "letElse",
            "kind": to_json_let_kind(value.kind_value),
            "mutability": destack._generated.dir.tree.node.to_json_mutability(
                value.mutability
            ),
            "declarator": destack._generated.dir.tree.node.to_json_local_node_id(
                value.declarator
            ),
            "elseBranch": destack._generated.dir.tree.node.to_json_local_node_id(
                value.else_branch
            ),
        }
    elif value.kind == "using":
        return {
            "kind": "using",
            "asynchrony": destack._generated.dir.tree.node.to_json_asynchrony(
                value.asynchrony
            ),
            **(
                {}
                if value.export is None
                else {
                    "export": destack._generated.dir.tree.dependency.to_json_export_kind(
                        value.export
                    )
                }
            ),
            "declarators": [
                destack._generated.dir.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.declarators
            ],
            "isAmbient": value.is_ambient,
        }
    elif value.kind == "if":
        return {
            "kind": "if",
            "form": to_json_if_form(value.form),
            "condition": to_json_condition(value.condition),
            "thenExpression": destack._generated.dir.tree.node.to_json_local_node_id(
                value.then_expression
            ),
            **(
                {}
                if value.else_expression is None
                else {
                    "elseExpression": destack._generated.dir.tree.node.to_json_local_node_id(
                        value.else_expression
                    )
                }
            ),
        }
    elif value.kind == "while":
        return {
            "kind": "while",
            "form": to_json_while_form(value.form),
            "condition": destack._generated.dir.tree.node.to_json_local_node_id(
                value.condition
            ),
            "body": destack._generated.dir.tree.node.to_json_local_node_id(value.body),
        }
    elif value.kind == "forEach":
        return {
            "kind": "forEach",
            "asynchrony": destack._generated.dir.tree.node.to_json_asynchrony(
                value.asynchrony
            ),
            "operator": to_json_for_each_operator(value.operator),
            "binding": to_json_for_each_binding(value.binding),
            "iterator": destack._generated.dir.tree.node.to_json_local_node_id(
                value.iterator
            ),
            "body": destack._generated.dir.tree.node.to_json_local_node_id(value.body),
        }
    elif value.kind == "for":
        return {
            "kind": "for",
            **(
                {}
                if value.initialization is None
                else {
                    "initialization": destack._generated.dir.tree.node.to_json_local_node_id(
                        value.initialization
                    )
                }
            ),
            **(
                {}
                if value.condition is None
                else {
                    "condition": destack._generated.dir.tree.node.to_json_local_node_id(
                        value.condition
                    )
                }
            ),
            **(
                {}
                if value.increment is None
                else {
                    "increment": destack._generated.dir.tree.node.to_json_local_node_id(
                        value.increment
                    )
                }
            ),
            "body": destack._generated.dir.tree.node.to_json_local_node_id(value.body),
        }
    elif value.kind == "loop":
        return {
            "kind": "loop",
            "body": destack._generated.dir.tree.node.to_json_local_node_id(value.body),
        }
    elif value.kind == "try":
        return {
            "kind": "try",
            "body": destack._generated.dir.tree.node.to_json_local_node_id(value.body),
            **(
                {}
                if value.catch is None
                else {
                    "catch": destack._generated.dir.tree.node.to_json_local_node_id(
                        value.catch
                    )
                }
            ),
            **(
                {}
                if value.finally_ is None
                else {
                    "finally": destack._generated.dir.tree.node.to_json_local_node_id(
                        value.finally_
                    )
                }
            ),
        }
    elif value.kind == "match":
        return {
            "kind": "match",
            "form": destack._generated.dir.tree.match.to_json_match_form(value.form),
            "value": destack._generated.dir.tree.node.to_json_local_node_id(
                value.value
            ),
            "cases": [
                destack._generated.dir.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.cases
            ],
        }
    elif value.kind == "break":
        return {
            "kind": "break",
            **(
                {}
                if value.label is None
                else {
                    "label": destack._generated.core.string.to_json_string_id(
                        value.label
                    )
                }
            ),
            **(
                {}
                if value.value is None
                else {
                    "value": destack._generated.dir.tree.node.to_json_local_node_id(
                        value.value
                    )
                }
            ),
        }
    elif value.kind == "continue":
        return {
            "kind": "continue",
            **(
                {}
                if value.label is None
                else {
                    "label": destack._generated.core.string.to_json_string_id(
                        value.label
                    )
                }
            ),
        }
    elif value.kind == "await":
        return {
            "kind": "await",
            "expression": destack._generated.dir.tree.node.to_json_local_node_id(
                value.expression
            ),
        }
    elif value.kind == "awaitMaybe":
        return {
            "kind": "awaitMaybe",
            "expression": destack._generated.dir.tree.node.to_json_local_node_id(
                value.expression
            ),
        }
    elif value.kind == "awaitMust":
        return {
            "kind": "awaitMust",
            "expression": destack._generated.dir.tree.node.to_json_local_node_id(
                value.expression
            ),
        }
    elif value.kind == "yield":
        return {
            "kind": "yield",
            "cardinality": to_json_yield_cardinality(value.cardinality),
            **(
                {}
                if value.value is None
                else {
                    "value": destack._generated.dir.tree.node.to_json_local_node_id(
                        value.value
                    )
                }
            ),
        }
    elif value.kind == "throw":
        return {
            "kind": "throw",
            "value": destack._generated.dir.tree.node.to_json_local_node_id(
                value.value
            ),
        }
    elif value.kind == "return":
        return {
            "kind": "return",
            **(
                {}
                if value.value is None
                else {
                    "value": destack._generated.dir.tree.node.to_json_local_node_id(
                        value.value
                    )
                }
            ),
        }
    elif value.kind == "identifier":
        return {
            "kind": "identifier",
            "name": destack._generated.core.string.to_json_string_id(value.name),
        }
    elif value.kind == "privateIdentifier":
        return {
            "kind": "privateIdentifier",
            "name": destack._generated.core.string.to_json_string_id(value.name),
        }
    elif value.kind == "this":
        return {
            "kind": "this",
        }
    elif value.kind == "super":
        return {
            "kind": "super",
        }
    elif value.kind == "importMeta":
        return {
            "kind": "importMeta",
        }
    elif value.kind == "importSource":
        return {
            "kind": "importSource",
        }
    elif value.kind == "scalarLiteral":
        return {
            "kind": "scalarLiteral",
            "scalar_literal": destack._generated.dir.tree.literal.to_json_scalar_literal(
                value.scalar_literal
            ),
        }
    elif value.kind == "rangeExpression":
        return {
            "kind": "rangeExpression",
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
    elif value.kind == "templateExpression":
        return {
            "kind": "templateExpression",
            "value": destack._generated.dir.tree.literal.to_json_template_literal(
                value.value
            ),
        }
    elif value.kind == "taggedTemplateExpression":
        return {
            "kind": "taggedTemplateExpression",
            "tag": destack._generated.dir.tree.node.to_json_local_node_id(value.tag),
            "genericArguments": [
                destack._generated.dir.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.generic_arguments
            ],
            "value": destack._generated.dir.tree.literal.to_json_template_literal(
                value.value
            ),
        }
    elif value.kind == "arrayExpression":
        return {
            "kind": "arrayExpression",
            "elements": [
                destack._generated.dir.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.elements
            ],
        }
    elif value.kind == "fixedArrayExpression":
        return {
            "kind": "fixedArrayExpression",
            "value": destack._generated.dir.tree.node.to_json_local_node_id(
                value.value
            ),
            "length": destack._generated.dir.tree.node.to_json_local_node_id(
                value.length
            ),
        }
    elif value.kind == "tupleExpression":
        return {
            "kind": "tupleExpression",
            "elements": [
                destack._generated.dir.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.elements
            ],
        }
    elif value.kind == "sequenceExpression":
        return {
            "kind": "sequenceExpression",
            "expressions": [
                destack._generated.dir.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.expressions
            ],
        }
    elif value.kind == "objectExpression":
        return {
            "kind": "objectExpression",
            "properties": [
                destack._generated.dir.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.properties
            ],
        }
    elif value.kind == "structExpression":
        return {
            "kind": "structExpression",
            "ty": destack._generated.dir.tree.node.to_json_local_node_id(value.ty),
            "properties": [
                destack._generated.dir.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.properties
            ],
        }
    elif value.kind == "treeExpression":
        return {
            "kind": "treeExpression",
            **(
                {}
                if value.left is None
                else {
                    "left": destack._generated.dir.tree.node.to_json_local_node_id(
                        value.left
                    )
                }
            ),
            "genericArguments": [
                destack._generated.dir.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.generic_arguments
            ],
            **(
                {}
                if value.attributes is None
                else {
                    "attributes": [
                        destack._generated.dir.tree.node.to_json_local_node_id(item_0)
                        for item_0 in value.attributes
                    ]
                }
            ),
            **(
                {}
                if value.children is None
                else {
                    "children": [
                        destack._generated.dir.tree.node.to_json_local_node_id(item_0)
                        for item_0 in value.children
                    ]
                }
            ),
        }
    elif value.kind == "parenthesized":
        return {
            "kind": "parenthesized",
            "expression": destack._generated.dir.tree.node.to_json_local_node_id(
                value.expression
            ),
        }
    elif value.kind == "type":
        return {
            "kind": "type",
            "value": destack._generated.dir.tree.node.to_json_local_node_id(
                value.value
            ),
        }
    elif value.kind == "comptime":
        return {
            "kind": "comptime",
            "body": destack._generated.dir.tree.node.to_json_local_node_id(value.body),
        }
    elif value.kind == "as":
        return {
            "kind": "as",
            "expression": destack._generated.dir.tree.node.to_json_local_node_id(
                value.expression
            ),
            "targetType": destack._generated.dir.tree.node.to_json_local_node_id(
                value.target_type
            ),
        }
    elif value.kind == "satisfies":
        return {
            "kind": "satisfies",
            "expression": destack._generated.dir.tree.node.to_json_local_node_id(
                value.expression
            ),
            "targetType": destack._generated.dir.tree.node.to_json_local_node_id(
                value.target_type
            ),
        }
    elif value.kind == "is":
        return {
            "kind": "is",
            "value": destack._generated.dir.tree.node.to_json_local_node_id(
                value.value
            ),
            "targetType": destack._generated.dir.tree.node.to_json_local_node_id(
                value.target_type
            ),
        }
    elif value.kind == "instanceOf":
        return {
            "kind": "instanceOf",
            "value": destack._generated.dir.tree.node.to_json_local_node_id(
                value.value
            ),
            "target": destack._generated.dir.tree.node.to_json_local_node_id(
                value.target
            ),
        }
    elif value.kind == "unary":
        return {
            "kind": "unary",
            "operator": destack._generated.dir.tree.operator.to_json_unary_operator(
                value.operator
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
            **(
                {}
                if value.variance is None
                else {"variance": to_json_variance_bound(value.variance)}
            ),
            "right": destack._generated.dir.tree.node.to_json_local_node_id(
                value.right
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
            **(
                {}
                if value.variance is None
                else {"variance": to_json_variance_bound(value.variance)}
            ),
            "right": destack._generated.dir.tree.node.to_json_local_node_id(
                value.right
            ),
        }
    elif value.kind == "member":
        return {
            "kind": "member",
            "left": destack._generated.dir.tree.node.to_json_local_node_id(value.left),
            **(
                {}
                if value.name is None
                else {
                    "name": destack._generated.core.string.to_json_string_id(value.name)
                }
            ),
        }
    elif value.kind == "privateMember":
        return {
            "kind": "privateMember",
            "left": destack._generated.dir.tree.node.to_json_local_node_id(value.left),
            **(
                {}
                if value.name is None
                else {
                    "name": destack._generated.core.string.to_json_string_id(value.name)
                }
            ),
        }
    elif value.kind == "index":
        return {
            "kind": "index",
            "position": to_json_postfix_position(value.position),
            "left": destack._generated.dir.tree.node.to_json_local_node_id(value.left),
            **(
                {}
                if value.index is None
                else {
                    "index": destack._generated.dir.tree.node.to_json_local_node_id(
                        value.index
                    )
                }
            ),
        }
    elif value.kind == "instantiation":
        return {
            "kind": "instantiation",
            "left": destack._generated.dir.tree.node.to_json_local_node_id(value.left),
            "genericArguments": [
                destack._generated.dir.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.generic_arguments
            ],
        }
    elif value.kind == "call":
        return {
            "kind": "call",
            "position": to_json_postfix_position(value.position),
            "left": destack._generated.dir.tree.node.to_json_local_node_id(value.left),
            "genericArguments": [
                destack._generated.dir.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.generic_arguments
            ],
            "arguments": [
                destack._generated.dir.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.arguments
            ],
        }
    elif value.kind == "new":
        return {
            "kind": "new",
            "ty": destack._generated.dir.tree.node.to_json_local_node_id(value.ty),
            "arguments": [
                destack._generated.dir.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.arguments
            ],
        }
    elif value.kind == "newMaybe":
        return {
            "kind": "newMaybe",
            "ty": destack._generated.dir.tree.node.to_json_local_node_id(value.ty),
            "arguments": [
                destack._generated.dir.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.arguments
            ],
        }
    elif value.kind == "maybe":
        return {
            "kind": "maybe",
            "position": to_json_postfix_position(value.position),
            "left": destack._generated.dir.tree.node.to_json_local_node_id(value.left),
        }
    elif value.kind == "must":
        return {
            "kind": "must",
            "position": to_json_postfix_position(value.position),
            "left": destack._generated.dir.tree.node.to_json_local_node_id(value.left),
        }
    elif value.kind == "binary":
        return {
            "kind": "binary",
            "left": destack._generated.dir.tree.node.to_json_local_node_id(value.left),
            "operator": destack._generated.dir.tree.operator.to_json_binary_operator(
                value.operator
            ),
            "right": destack._generated.dir.tree.node.to_json_local_node_id(
                value.right
            ),
        }
    elif value.kind == "assign":
        return {
            "kind": "assign",
            "left": destack._generated.dir.tree.node.to_json_local_node_id(value.left),
            "operator": destack._generated.dir.tree.operator.to_json_assign_operator(
                value.operator
            ),
            "right": destack._generated.dir.tree.node.to_json_local_node_id(
                value.right
            ),
        }
    elif value.kind == "debugger":
        return {
            "kind": "debugger",
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
            declaration=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "declaration")
            )
        )
    elif kind == "block":
        return ExpressionBlock(
            block=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "block")
            )
        )
    elif kind == "label":
        return ExpressionLabel(
            label=destack._generated.core.string.from_json_string_id(
                json_field(object_, "label")
            ),
            body=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "body")
            ),
        )
    elif kind == "import":
        return ExpressionImport(
            form=destack._generated.dir.tree.dependency.from_json_dependency_form(
                json_field(object_, "form")
            ),
            target=destack._generated.core.string.from_json_string_id(
                json_field(object_, "target")
            ),
            items=json_optional(
                object_,
                "items",
                lambda value: [
                    destack._generated.dir.tree.node.from_json_local_node_id(item_0)
                    for item_0 in json_array(value)
                ],
            ),
            attributes=json_optional(
                object_,
                "attributes",
                lambda value: (
                    destack._generated.dir.tree.import_.from_json_import_attribute_clause(
                        value
                    )
                ),
            ),
        )
    elif kind == "export":
        return ExpressionExport(
            form=destack._generated.dir.tree.dependency.from_json_dependency_form(
                json_field(object_, "form")
            ),
            target=json_optional(
                object_,
                "target",
                lambda value: destack._generated.core.string.from_json_string_id(value),
            ),
            items=[
                destack._generated.dir.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "items"))
            ],
            attributes=json_optional(
                object_,
                "attributes",
                lambda value: (
                    destack._generated.dir.tree.import_.from_json_import_attribute_clause(
                        value
                    )
                ),
            ),
        )
    elif kind == "let":
        return ExpressionLet(
            kind_value=from_json_let_kind(json_field(object_, "kind")),
            export=json_optional(
                object_,
                "export",
                lambda value: (
                    destack._generated.dir.tree.dependency.from_json_export_kind(value)
                ),
            ),
            mutability=destack._generated.dir.tree.node.from_json_mutability(
                json_field(object_, "mutability")
            ),
            declarators=[
                destack._generated.dir.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "declarators"))
            ],
            is_ambient=json_bool(json_field(object_, "isAmbient")),
            place=json_optional(
                object_,
                "place",
                lambda value: (
                    destack._generated.dir.tree.declaration.from_json_place_modifier(
                        value
                    )
                ),
            ),
        )
    elif kind == "letElse":
        return ExpressionLetElse(
            kind_value=from_json_let_kind(json_field(object_, "kind")),
            mutability=destack._generated.dir.tree.node.from_json_mutability(
                json_field(object_, "mutability")
            ),
            declarator=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "declarator")
            ),
            else_branch=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "elseBranch")
            ),
        )
    elif kind == "using":
        return ExpressionUsing(
            asynchrony=destack._generated.dir.tree.node.from_json_asynchrony(
                json_field(object_, "asynchrony")
            ),
            export=json_optional(
                object_,
                "export",
                lambda value: (
                    destack._generated.dir.tree.dependency.from_json_export_kind(value)
                ),
            ),
            declarators=[
                destack._generated.dir.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "declarators"))
            ],
            is_ambient=json_bool(json_field(object_, "isAmbient")),
        )
    elif kind == "if":
        return ExpressionIf(
            form=from_json_if_form(json_field(object_, "form")),
            condition=from_json_condition(json_field(object_, "condition")),
            then_expression=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "thenExpression")
            ),
            else_expression=json_optional(
                object_,
                "elseExpression",
                lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                    value
                ),
            ),
        )
    elif kind == "while":
        return ExpressionWhile(
            form=from_json_while_form(json_field(object_, "form")),
            condition=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "condition")
            ),
            body=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "body")
            ),
        )
    elif kind == "forEach":
        return ExpressionForEach(
            asynchrony=destack._generated.dir.tree.node.from_json_asynchrony(
                json_field(object_, "asynchrony")
            ),
            operator=from_json_for_each_operator(json_field(object_, "operator")),
            binding=from_json_for_each_binding(json_field(object_, "binding")),
            iterator=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "iterator")
            ),
            body=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "body")
            ),
        )
    elif kind == "for":
        return ExpressionFor(
            initialization=json_optional(
                object_,
                "initialization",
                lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                    value
                ),
            ),
            condition=json_optional(
                object_,
                "condition",
                lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                    value
                ),
            ),
            increment=json_optional(
                object_,
                "increment",
                lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                    value
                ),
            ),
            body=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "body")
            ),
        )
    elif kind == "loop":
        return ExpressionLoop(
            body=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "body")
            ),
        )
    elif kind == "try":
        return ExpressionTry(
            body=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "body")
            ),
            catch=json_optional(
                object_,
                "catch",
                lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                    value
                ),
            ),
            finally_=json_optional(
                object_,
                "finally",
                lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                    value
                ),
            ),
        )
    elif kind == "match":
        return ExpressionMatch(
            form=destack._generated.dir.tree.match.from_json_match_form(
                json_field(object_, "form")
            ),
            value=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "value")
            ),
            cases=[
                destack._generated.dir.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "cases"))
            ],
        )
    elif kind == "break":
        return ExpressionBreak(
            label=json_optional(
                object_,
                "label",
                lambda value: destack._generated.core.string.from_json_string_id(value),
            ),
            value=json_optional(
                object_,
                "value",
                lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                    value
                ),
            ),
        )
    elif kind == "continue":
        return ExpressionContinue(
            label=json_optional(
                object_,
                "label",
                lambda value: destack._generated.core.string.from_json_string_id(value),
            ),
        )
    elif kind == "await":
        return ExpressionAwait(
            expression=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "expression")
            ),
        )
    elif kind == "awaitMaybe":
        return ExpressionAwaitMaybe(
            expression=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "expression")
            ),
        )
    elif kind == "awaitMust":
        return ExpressionAwaitMust(
            expression=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "expression")
            ),
        )
    elif kind == "yield":
        return ExpressionYield(
            cardinality=from_json_yield_cardinality(json_field(object_, "cardinality")),
            value=json_optional(
                object_,
                "value",
                lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                    value
                ),
            ),
        )
    elif kind == "throw":
        return ExpressionThrow(
            value=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "value")
            ),
        )
    elif kind == "return":
        return ExpressionReturn(
            value=json_optional(
                object_,
                "value",
                lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                    value
                ),
            ),
        )
    elif kind == "identifier":
        return ExpressionIdentifier(
            name=destack._generated.core.string.from_json_string_id(
                json_field(object_, "name")
            ),
        )
    elif kind == "privateIdentifier":
        return ExpressionPrivateIdentifier(
            name=destack._generated.core.string.from_json_string_id(
                json_field(object_, "name")
            ),
        )
    elif kind == "this":
        return ExpressionThis()
    elif kind == "super":
        return ExpressionSuper()
    elif kind == "importMeta":
        return ExpressionImportMeta()
    elif kind == "importSource":
        return ExpressionImportSource()
    elif kind == "scalarLiteral":
        return ExpressionScalarLiteral(
            scalar_literal=destack._generated.dir.tree.literal.from_json_scalar_literal(
                json_field(object_, "scalar_literal")
            )
        )
    elif kind == "rangeExpression":
        return ExpressionRangeExpression(
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
    elif kind == "templateExpression":
        return ExpressionTemplateExpression(
            value=destack._generated.dir.tree.literal.from_json_template_literal(
                json_field(object_, "value")
            ),
        )
    elif kind == "taggedTemplateExpression":
        return ExpressionTaggedTemplateExpression(
            tag=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "tag")
            ),
            generic_arguments=[
                destack._generated.dir.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "genericArguments"))
            ],
            value=destack._generated.dir.tree.literal.from_json_template_literal(
                json_field(object_, "value")
            ),
        )
    elif kind == "arrayExpression":
        return ExpressionArrayExpression(
            elements=[
                destack._generated.dir.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "elements"))
            ],
        )
    elif kind == "fixedArrayExpression":
        return ExpressionFixedArrayExpression(
            value=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "value")
            ),
            length=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "length")
            ),
        )
    elif kind == "tupleExpression":
        return ExpressionTupleExpression(
            elements=[
                destack._generated.dir.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "elements"))
            ],
        )
    elif kind == "sequenceExpression":
        return ExpressionSequenceExpression(
            expressions=[
                destack._generated.dir.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "expressions"))
            ],
        )
    elif kind == "objectExpression":
        return ExpressionObjectExpression(
            properties=[
                destack._generated.dir.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "properties"))
            ],
        )
    elif kind == "structExpression":
        return ExpressionStructExpression(
            ty=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "ty")
            ),
            properties=[
                destack._generated.dir.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "properties"))
            ],
        )
    elif kind == "treeExpression":
        return ExpressionTreeExpression(
            left=json_optional(
                object_,
                "left",
                lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                    value
                ),
            ),
            generic_arguments=[
                destack._generated.dir.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "genericArguments"))
            ],
            attributes=json_optional(
                object_,
                "attributes",
                lambda value: [
                    destack._generated.dir.tree.node.from_json_local_node_id(item_0)
                    for item_0 in json_array(value)
                ],
            ),
            children=json_optional(
                object_,
                "children",
                lambda value: [
                    destack._generated.dir.tree.node.from_json_local_node_id(item_0)
                    for item_0 in json_array(value)
                ],
            ),
        )
    elif kind == "parenthesized":
        return ExpressionParenthesized(
            expression=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "expression")
            ),
        )
    elif kind == "type":
        return ExpressionType(
            value=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "value")
            ),
        )
    elif kind == "comptime":
        return ExpressionComptime(
            body=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "body")
            ),
        )
    elif kind == "as":
        return ExpressionAs(
            expression=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "expression")
            ),
            target_type=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "targetType")
            ),
        )
    elif kind == "satisfies":
        return ExpressionSatisfies(
            expression=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "expression")
            ),
            target_type=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "targetType")
            ),
        )
    elif kind == "is":
        return ExpressionIs(
            value=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "value")
            ),
            target_type=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "targetType")
            ),
        )
    elif kind == "instanceOf":
        return ExpressionInstanceOf(
            value=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "value")
            ),
            target=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "target")
            ),
        )
    elif kind == "unary":
        return ExpressionUnary(
            operator=destack._generated.dir.tree.operator.from_json_unary_operator(
                json_field(object_, "operator")
            ),
            right=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "right")
            ),
        )
    elif kind == "moveOf":
        return ExpressionMoveOf(
            mutability=json_optional(
                object_,
                "mutability",
                lambda value: destack._generated.dir.tree.node.from_json_mutability(
                    value
                ),
            ),
            variance=json_optional(
                object_, "variance", lambda value: from_json_variance_bound(value)
            ),
            right=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "right")
            ),
        )
    elif kind == "borrowOf":
        return ExpressionBorrowOf(
            mutability=json_optional(
                object_,
                "mutability",
                lambda value: destack._generated.dir.tree.node.from_json_mutability(
                    value
                ),
            ),
            variance=json_optional(
                object_, "variance", lambda value: from_json_variance_bound(value)
            ),
            right=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "right")
            ),
        )
    elif kind == "member":
        return ExpressionMember(
            left=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "left")
            ),
            name=json_optional(
                object_,
                "name",
                lambda value: destack._generated.core.string.from_json_string_id(value),
            ),
        )
    elif kind == "privateMember":
        return ExpressionPrivateMember(
            left=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "left")
            ),
            name=json_optional(
                object_,
                "name",
                lambda value: destack._generated.core.string.from_json_string_id(value),
            ),
        )
    elif kind == "index":
        return ExpressionIndex(
            position=from_json_postfix_position(json_field(object_, "position")),
            left=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "left")
            ),
            index=json_optional(
                object_,
                "index",
                lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                    value
                ),
            ),
        )
    elif kind == "instantiation":
        return ExpressionInstantiation(
            left=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "left")
            ),
            generic_arguments=[
                destack._generated.dir.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "genericArguments"))
            ],
        )
    elif kind == "call":
        return ExpressionCall(
            position=from_json_postfix_position(json_field(object_, "position")),
            left=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "left")
            ),
            generic_arguments=[
                destack._generated.dir.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "genericArguments"))
            ],
            arguments=[
                destack._generated.dir.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "arguments"))
            ],
        )
    elif kind == "new":
        return ExpressionNew(
            ty=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "ty")
            ),
            arguments=[
                destack._generated.dir.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "arguments"))
            ],
        )
    elif kind == "newMaybe":
        return ExpressionNewMaybe(
            ty=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "ty")
            ),
            arguments=[
                destack._generated.dir.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "arguments"))
            ],
        )
    elif kind == "maybe":
        return ExpressionMaybe(
            position=from_json_postfix_position(json_field(object_, "position")),
            left=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "left")
            ),
        )
    elif kind == "must":
        return ExpressionMust(
            position=from_json_postfix_position(json_field(object_, "position")),
            left=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "left")
            ),
        )
    elif kind == "binary":
        return ExpressionBinary(
            left=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "left")
            ),
            operator=destack._generated.dir.tree.operator.from_json_binary_operator(
                json_field(object_, "operator")
            ),
            right=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "right")
            ),
        )
    elif kind == "assign":
        return ExpressionAssign(
            left=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "left")
            ),
            operator=destack._generated.dir.tree.operator.from_json_assign_operator(
                json_field(object_, "operator")
            ),
            right=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "right")
            ),
        )
    elif kind == "debugger":
        return ExpressionDebugger()
    elif kind == "missing":
        return ExpressionMissing()
    elif kind == "stub":
        return ExpressionStub()
    elif kind == "error":
        return ExpressionError()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


"""The kind of a let or const binding."""
LetKind: typing.TypeAlias = typing.Literal["let"] | typing.Literal["const"]


def encode_let_kind(writer: BinaryWriter, value: LetKind) -> None:
    """Encode one LetKind."""
    if value == "let":
        writer.write_unsigned(0)
    elif value == "const":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_let_kind(reader: BinaryReader) -> LetKind:
    """Decode one LetKind."""
    variant = reader.read_number()

    if variant == 0:
        return "let"
    elif variant == 1:
        return "const"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_let_kind(value: LetKind) -> Json:
    """Return one JSON value for one LetKind."""
    return value


def from_json_let_kind(value: Json) -> LetKind:
    """Return one LetKind from one JSON value."""
    variant = json_string(value)

    if variant == "let":
        return "let"
    elif variant == "const":
        return "const"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""The style of if expression."""
IfForm: typing.TypeAlias = typing.Literal["if"] | typing.Literal["ternary"]


def encode_if_form(writer: BinaryWriter, value: IfForm) -> None:
    """Encode one IfForm."""
    if value == "if":
        writer.write_unsigned(0)
    elif value == "ternary":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_if_form(reader: BinaryReader) -> IfForm:
    """Decode one IfForm."""
    variant = reader.read_number()

    if variant == 0:
        return "if"
    elif variant == 1:
        return "ternary"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_if_form(value: IfForm) -> Json:
    """Return one JSON value for one IfForm."""
    return value


def from_json_if_form(value: Json) -> IfForm:
    """Return one IfForm from one JSON value."""
    variant = json_string(value)

    if variant == "if":
        return "if"
    elif variant == "ternary":
        return "ternary"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class Condition:
    """A left-to-right condition."""

    # the operands joined by short-circuiting `&&`
    operands: Sequence[ConditionOperand]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_condition(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Condition:
        """Decode one Condition."""
        return decode_condition(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_condition(self)

    @classmethod
    def from_json(cls, value: Json) -> Condition:
        """Return one Condition from one JSON value."""
        return from_json_condition(value)


def encode_condition(writer: BinaryWriter, value: Condition) -> None:
    """Encode one Condition."""
    writer.write_unsigned(len(value.operands))
    for item_value_operands_0 in value.operands:
        encode_condition_operand(writer, item_value_operands_0)


def decode_condition(reader: BinaryReader) -> Condition:
    """Decode one Condition."""
    operands = [decode_condition_operand(reader) for _ in range(reader.read_number())]

    return Condition(
        operands=operands,
    )


def to_json_condition(value: Condition) -> Json:
    """Return one JSON value for one Condition."""
    return {
        "operands": [to_json_condition_operand(item_0) for item_0 in value.operands],
    }


def from_json_condition(value: Json) -> Condition:
    """Return one Condition from one JSON value."""
    object_ = json_object(value)

    return Condition(
        operands=[
            from_json_condition_operand(item_0)
            for item_0 in json_array(json_field(object_, "operands"))
        ],
    )


@dataclass(frozen=True, slots=True)
class ConditionOperandExpression:
    """A regular condition expression."""

    condition: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["expression"] = "expression"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_condition_operand(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_condition_operand(self)


@dataclass(frozen=True, slots=True)
class ConditionOperandBinding:
    """A pattern binding condition."""

    # the keyword used for the binding
    kind_value: LetKind
    # the mutability derived from the binding keyword
    mutability: destack._generated.dir.tree.node.Mutability
    # the declarator for the binding
    declarator: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["binding"] = "binding"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_condition_operand(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_condition_operand(self)


"""One operand in a condition."""
ConditionOperand: typing.TypeAlias = (
    ConditionOperandExpression | ConditionOperandBinding
)


def encode_condition_operand(writer: BinaryWriter, value: ConditionOperand) -> None:
    """Encode one ConditionOperand."""
    if value.kind == "expression":
        writer.write_unsigned(0)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.condition)
    elif value.kind == "binding":
        writer.write_unsigned(1)
        encode_let_kind(writer, value.kind_value)
        destack._generated.dir.tree.node.encode_mutability(writer, value.mutability)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.declarator)
    else:
        raise SerdeError("unknown enum variant")


def decode_condition_operand(reader: BinaryReader) -> ConditionOperand:
    """Decode one ConditionOperand."""
    variant = reader.read_number()

    if variant == 0:
        condition = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return ConditionOperandExpression(
            condition=condition,
        )
    elif variant == 1:
        kind_value = decode_let_kind(reader)
        mutability = destack._generated.dir.tree.node.decode_mutability(reader)
        declarator = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return ConditionOperandBinding(
            kind_value=kind_value,
            mutability=mutability,
            declarator=declarator,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_condition_operand(value: ConditionOperand) -> Json:
    """Return one JSON value for one ConditionOperand."""
    if value.kind == "expression":
        return {
            "kind": "expression",
            "condition": destack._generated.dir.tree.node.to_json_local_node_id(
                value.condition
            ),
        }
    elif value.kind == "binding":
        return {
            "kind": "binding",
            "kind": to_json_let_kind(value.kind_value),
            "mutability": destack._generated.dir.tree.node.to_json_mutability(
                value.mutability
            ),
            "declarator": destack._generated.dir.tree.node.to_json_local_node_id(
                value.declarator
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_condition_operand(value: Json) -> ConditionOperand:
    """Return one ConditionOperand from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "expression":
        return ConditionOperandExpression(
            condition=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "condition")
            ),
        )
    elif kind == "binding":
        return ConditionOperandBinding(
            kind_value=from_json_let_kind(json_field(object_, "kind")),
            mutability=destack._generated.dir.tree.node.from_json_mutability(
                json_field(object_, "mutability")
            ),
            declarator=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "declarator")
            ),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


"""The kind of a while expression."""
WhileForm: typing.TypeAlias = typing.Literal["while"] | typing.Literal["doWhile"]


def encode_while_form(writer: BinaryWriter, value: WhileForm) -> None:
    """Encode one WhileForm."""
    if value == "while":
        writer.write_unsigned(0)
    elif value == "doWhile":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_while_form(reader: BinaryReader) -> WhileForm:
    """Decode one WhileForm."""
    variant = reader.read_number()

    if variant == 0:
        return "while"
    elif variant == 1:
        return "doWhile"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_while_form(value: WhileForm) -> Json:
    """Return one JSON value for one WhileForm."""
    return value


def from_json_while_form(value: Json) -> WhileForm:
    """Return one WhileForm from one JSON value."""
    variant = json_string(value)

    if variant == "while":
        return "while"
    elif variant == "doWhile":
        return "doWhile"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""The kind of a for each expression."""
ForEachOperator: typing.TypeAlias = typing.Literal["of"] | typing.Literal["in"]


def encode_for_each_operator(writer: BinaryWriter, value: ForEachOperator) -> None:
    """Encode one ForEachOperator."""
    if value == "of":
        writer.write_unsigned(0)
    elif value == "in":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_for_each_operator(reader: BinaryReader) -> ForEachOperator:
    """Decode one ForEachOperator."""
    variant = reader.read_number()

    if variant == 0:
        return "of"
    elif variant == 1:
        return "in"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_for_each_operator(value: ForEachOperator) -> Json:
    """Return one JSON value for one ForEachOperator."""
    return value


def from_json_for_each_operator(value: Json) -> ForEachOperator:
    """Return one ForEachOperator from one JSON value."""
    variant = json_string(value)

    if variant == "of":
        return "of"
    elif variant == "in":
        return "in"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class ForEachBindingPattern:
    """Regular pattern binding."""

    pattern: destack._generated.dir.tree.node.LocalNodeId
    keyword: BindingKeyword | None
    kind: typing.Literal["pattern"] = "pattern"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_for_each_binding(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_for_each_binding(self)


@dataclass(frozen=True, slots=True)
class ForEachBindingUsing:
    """Using binding with optional async disposal."""

    asynchrony: destack._generated.dir.tree.node.Asynchrony
    pattern: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["using"] = "using"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_for_each_binding(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_for_each_binding(self)


"""The binding in a for each expression."""
ForEachBinding: typing.TypeAlias = ForEachBindingPattern | ForEachBindingUsing


def encode_for_each_binding(writer: BinaryWriter, value: ForEachBinding) -> None:
    """Encode one ForEachBinding."""
    if value.kind == "pattern":
        writer.write_unsigned(0)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.pattern)
        if value.keyword is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            encode_binding_keyword(writer, value.keyword)
    elif value.kind == "using":
        writer.write_unsigned(1)
        destack._generated.dir.tree.node.encode_asynchrony(writer, value.asynchrony)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.pattern)
    else:
        raise SerdeError("unknown enum variant")


def decode_for_each_binding(reader: BinaryReader) -> ForEachBinding:
    """Decode one ForEachBinding."""
    variant = reader.read_number()

    if variant == 0:
        pattern = destack._generated.dir.tree.node.decode_local_node_id(reader)
        keyword = reader.read_option(lambda: decode_binding_keyword(reader))

        return ForEachBindingPattern(
            pattern=pattern,
            keyword=keyword,
        )
    elif variant == 1:
        asynchrony = destack._generated.dir.tree.node.decode_asynchrony(reader)
        pattern = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return ForEachBindingUsing(
            asynchrony=asynchrony,
            pattern=pattern,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_for_each_binding(value: ForEachBinding) -> Json:
    """Return one JSON value for one ForEachBinding."""
    if value.kind == "pattern":
        return {
            "kind": "pattern",
            "pattern": destack._generated.dir.tree.node.to_json_local_node_id(
                value.pattern
            ),
            **(
                {}
                if value.keyword is None
                else {"keyword": to_json_binding_keyword(value.keyword)}
            ),
        }
    elif value.kind == "using":
        return {
            "kind": "using",
            "asynchrony": destack._generated.dir.tree.node.to_json_asynchrony(
                value.asynchrony
            ),
            "pattern": destack._generated.dir.tree.node.to_json_local_node_id(
                value.pattern
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_for_each_binding(value: Json) -> ForEachBinding:
    """Return one ForEachBinding from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "pattern":
        return ForEachBindingPattern(
            pattern=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "pattern")
            ),
            keyword=json_optional(
                object_, "keyword", lambda value: from_json_binding_keyword(value)
            ),
        )
    elif kind == "using":
        return ForEachBindingUsing(
            asynchrony=destack._generated.dir.tree.node.from_json_asynchrony(
                json_field(object_, "asynchrony")
            ),
            pattern=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "pattern")
            ),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


"""The declaration keyword used by a for each pattern binding."""
BindingKeyword: typing.TypeAlias = typing.Literal["let"] | typing.Literal["const"]


def encode_binding_keyword(writer: BinaryWriter, value: BindingKeyword) -> None:
    """Encode one BindingKeyword."""
    if value == "let":
        writer.write_unsigned(0)
    elif value == "const":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_binding_keyword(reader: BinaryReader) -> BindingKeyword:
    """Decode one BindingKeyword."""
    variant = reader.read_number()

    if variant == 0:
        return "let"
    elif variant == 1:
        return "const"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_binding_keyword(value: BindingKeyword) -> Json:
    """Return one JSON value for one BindingKeyword."""
    return value


def from_json_binding_keyword(value: Json) -> BindingKeyword:
    """Return one BindingKeyword from one JSON value."""
    variant = json_string(value)

    if variant == "let":
        return "let"
    elif variant == "const":
        return "const"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""The cardinality of a yield expression."""
YieldCardinality: typing.TypeAlias = (
    typing.Literal["scalar"] | typing.Literal["generator"]
)


def encode_yield_cardinality(writer: BinaryWriter, value: YieldCardinality) -> None:
    """Encode one YieldCardinality."""
    if value == "scalar":
        writer.write_unsigned(0)
    elif value == "generator":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_yield_cardinality(reader: BinaryReader) -> YieldCardinality:
    """Decode one YieldCardinality."""
    variant = reader.read_number()

    if variant == 0:
        return "scalar"
    elif variant == 1:
        return "generator"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_yield_cardinality(value: YieldCardinality) -> Json:
    """Return one JSON value for one YieldCardinality."""
    return value


def from_json_yield_cardinality(value: Json) -> YieldCardinality:
    """Return one YieldCardinality from one JSON value."""
    variant = json_string(value)

    if variant == "scalar":
        return "scalar"
    elif variant == "generator":
        return "generator"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""A TypeBound is a type bound for a reference operation."""
VarianceBound: typing.TypeAlias = (
    typing.Literal["implements"] | typing.Literal["extends"] | typing.Literal["super"]
)


def encode_variance_bound(writer: BinaryWriter, value: VarianceBound) -> None:
    """Encode one VarianceBound."""
    if value == "implements":
        writer.write_unsigned(0)
    elif value == "extends":
        writer.write_unsigned(1)
    elif value == "super":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_variance_bound(reader: BinaryReader) -> VarianceBound:
    """Decode one VarianceBound."""
    variant = reader.read_number()

    if variant == 0:
        return "implements"
    elif variant == 1:
        return "extends"
    elif variant == 2:
        return "super"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_variance_bound(value: VarianceBound) -> Json:
    """Return one JSON value for one VarianceBound."""
    return value


def from_json_variance_bound(value: Json) -> VarianceBound:
    """Return one VarianceBound from one JSON value."""
    variant = json_string(value)

    if variant == "implements":
        return "implements"
    elif variant == "extends":
        return "extends"
    elif variant == "super":
        return "super"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


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
class Catch:
    """A catch branch."""

    # the optional catch pattern
    pattern: destack._generated.dir.tree.node.LocalNodeId | None
    # the optional catch pattern type
    ty: destack._generated.dir.tree.node.LocalNodeId | None
    # the catch body
    body: destack._generated.dir.tree.node.LocalNodeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_catch(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Catch:
        """Decode one Catch."""
        return decode_catch(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_catch(self)

    @classmethod
    def from_json(cls, value: Json) -> Catch:
        """Return one Catch from one JSON value."""
        return from_json_catch(value)


def encode_catch(writer: BinaryWriter, value: Catch) -> None:
    """Encode one Catch."""
    if value.pattern is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.pattern)
    if value.ty is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.ty)
    destack._generated.dir.tree.node.encode_local_node_id(writer, value.body)


def decode_catch(reader: BinaryReader) -> Catch:
    """Decode one Catch."""
    pattern = reader.read_option(
        lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
    )
    ty = reader.read_option(
        lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
    )
    body = destack._generated.dir.tree.node.decode_local_node_id(reader)

    return Catch(
        pattern=pattern,
        ty=ty,
        body=body,
    )


def to_json_catch(value: Catch) -> Json:
    """Return one JSON value for one Catch."""
    return {
        **(
            {}
            if value.pattern is None
            else {
                "pattern": destack._generated.dir.tree.node.to_json_local_node_id(
                    value.pattern
                )
            }
        ),
        **(
            {}
            if value.ty is None
            else {
                "ty": destack._generated.dir.tree.node.to_json_local_node_id(value.ty)
            }
        ),
        "body": destack._generated.dir.tree.node.to_json_local_node_id(value.body),
    }


def from_json_catch(value: Json) -> Catch:
    """Return one Catch from one JSON value."""
    object_ = json_object(value)

    return Catch(
        pattern=json_optional(
            object_,
            "pattern",
            lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                value
            ),
        ),
        ty=json_optional(
            object_,
            "ty",
            lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                value
            ),
        ),
        body=destack._generated.dir.tree.node.from_json_local_node_id(
            json_field(object_, "body")
        ),
    )


@dataclass(frozen=True, slots=True)
class WhereClause:
    """A WhereClause is a single clause in a where type declaration."""

    # the relation between the two operands
    relation: WhereRelation
    # the left relation operand, like `T` in `T: int32`
    left: destack._generated.dir.tree.node.LocalNodeId
    # the right relation operand, like `int32` in `T: int32`
    right: destack._generated.dir.tree.node.LocalNodeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_where_clause(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> WhereClause:
        """Decode one WhereClause."""
        return decode_where_clause(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_where_clause(self)

    @classmethod
    def from_json(cls, value: Json) -> WhereClause:
        """Return one WhereClause from one JSON value."""
        return from_json_where_clause(value)


def encode_where_clause(writer: BinaryWriter, value: WhereClause) -> None:
    """Encode one WhereClause."""
    encode_where_relation(writer, value.relation)
    destack._generated.dir.tree.node.encode_local_node_id(writer, value.left)
    destack._generated.dir.tree.node.encode_local_node_id(writer, value.right)


def decode_where_clause(reader: BinaryReader) -> WhereClause:
    """Decode one WhereClause."""
    relation = decode_where_relation(reader)
    left = destack._generated.dir.tree.node.decode_local_node_id(reader)
    right = destack._generated.dir.tree.node.decode_local_node_id(reader)

    return WhereClause(
        relation=relation,
        left=left,
        right=right,
    )


def to_json_where_clause(value: WhereClause) -> Json:
    """Return one JSON value for one WhereClause."""
    return {
        "relation": to_json_where_relation(value.relation),
        "left": destack._generated.dir.tree.node.to_json_local_node_id(value.left),
        "right": destack._generated.dir.tree.node.to_json_local_node_id(value.right),
    }


def from_json_where_clause(value: Json) -> WhereClause:
    """Return one WhereClause from one JSON value."""
    object_ = json_object(value)

    return WhereClause(
        relation=from_json_where_relation(json_field(object_, "relation")),
        left=destack._generated.dir.tree.node.from_json_local_node_id(
            json_field(object_, "left")
        ),
        right=destack._generated.dir.tree.node.from_json_local_node_id(
            json_field(object_, "right")
        ),
    )


"""A where-clause relation."""
WhereRelation: typing.TypeAlias = typing.Literal["satisfies"] | typing.Literal["equals"]


def encode_where_relation(writer: BinaryWriter, value: WhereRelation) -> None:
    """Encode one WhereRelation."""
    if value == "satisfies":
        writer.write_unsigned(0)
    elif value == "equals":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_where_relation(reader: BinaryReader) -> WhereRelation:
    """Decode one WhereRelation."""
    variant = reader.read_number()

    if variant == 0:
        return "satisfies"
    elif variant == 1:
        return "equals"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_where_relation(value: WhereRelation) -> Json:
    """Return one JSON value for one WhereRelation."""
    return value


def from_json_where_relation(value: Json) -> WhereRelation:
    """Return one WhereRelation from one JSON value."""
    variant = json_string(value)

    if variant == "satisfies":
        return "satisfies"
    elif variant == "equals":
        return "equals"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "Expression",
    "encode_expression",
    "decode_expression",
    "to_json_expression",
    "from_json_expression",
    "ExpressionDeclaration",
    "ExpressionBlock",
    "ExpressionLabel",
    "ExpressionImport",
    "ExpressionExport",
    "ExpressionLet",
    "ExpressionLetElse",
    "ExpressionUsing",
    "ExpressionIf",
    "ExpressionWhile",
    "ExpressionForEach",
    "ExpressionFor",
    "ExpressionLoop",
    "ExpressionTry",
    "ExpressionMatch",
    "ExpressionBreak",
    "ExpressionContinue",
    "ExpressionAwait",
    "ExpressionAwaitMaybe",
    "ExpressionAwaitMust",
    "ExpressionYield",
    "ExpressionThrow",
    "ExpressionReturn",
    "ExpressionIdentifier",
    "ExpressionPrivateIdentifier",
    "ExpressionThis",
    "ExpressionSuper",
    "ExpressionImportMeta",
    "ExpressionImportSource",
    "ExpressionScalarLiteral",
    "ExpressionRangeExpression",
    "ExpressionTemplateExpression",
    "ExpressionTaggedTemplateExpression",
    "ExpressionArrayExpression",
    "ExpressionFixedArrayExpression",
    "ExpressionTupleExpression",
    "ExpressionSequenceExpression",
    "ExpressionObjectExpression",
    "ExpressionStructExpression",
    "ExpressionTreeExpression",
    "ExpressionParenthesized",
    "ExpressionType",
    "ExpressionComptime",
    "ExpressionAs",
    "ExpressionSatisfies",
    "ExpressionIs",
    "ExpressionInstanceOf",
    "ExpressionUnary",
    "ExpressionMoveOf",
    "ExpressionBorrowOf",
    "ExpressionMember",
    "ExpressionPrivateMember",
    "ExpressionIndex",
    "ExpressionInstantiation",
    "ExpressionCall",
    "ExpressionNew",
    "ExpressionNewMaybe",
    "ExpressionMaybe",
    "ExpressionMust",
    "ExpressionBinary",
    "ExpressionAssign",
    "ExpressionDebugger",
    "ExpressionMissing",
    "ExpressionStub",
    "ExpressionError",
    "LetKind",
    "encode_let_kind",
    "decode_let_kind",
    "to_json_let_kind",
    "from_json_let_kind",
    "IfForm",
    "encode_if_form",
    "decode_if_form",
    "to_json_if_form",
    "from_json_if_form",
    "Condition",
    "encode_condition",
    "decode_condition",
    "to_json_condition",
    "from_json_condition",
    "ConditionOperand",
    "encode_condition_operand",
    "decode_condition_operand",
    "to_json_condition_operand",
    "from_json_condition_operand",
    "ConditionOperandExpression",
    "ConditionOperandBinding",
    "WhileForm",
    "encode_while_form",
    "decode_while_form",
    "to_json_while_form",
    "from_json_while_form",
    "ForEachOperator",
    "encode_for_each_operator",
    "decode_for_each_operator",
    "to_json_for_each_operator",
    "from_json_for_each_operator",
    "ForEachBinding",
    "encode_for_each_binding",
    "decode_for_each_binding",
    "to_json_for_each_binding",
    "from_json_for_each_binding",
    "ForEachBindingPattern",
    "ForEachBindingUsing",
    "BindingKeyword",
    "encode_binding_keyword",
    "decode_binding_keyword",
    "to_json_binding_keyword",
    "from_json_binding_keyword",
    "YieldCardinality",
    "encode_yield_cardinality",
    "decode_yield_cardinality",
    "to_json_yield_cardinality",
    "from_json_yield_cardinality",
    "VarianceBound",
    "encode_variance_bound",
    "decode_variance_bound",
    "to_json_variance_bound",
    "from_json_variance_bound",
    "PostfixPosition",
    "encode_postfix_position",
    "decode_postfix_position",
    "to_json_postfix_position",
    "from_json_postfix_position",
    "Catch",
    "encode_catch",
    "decode_catch",
    "to_json_catch",
    "from_json_catch",
    "WhereClause",
    "encode_where_clause",
    "decode_where_clause",
    "to_json_where_clause",
    "from_json_where_clause",
    "WhereRelation",
    "encode_where_relation",
    "decode_where_relation",
    "to_json_where_relation",
    "from_json_where_relation",
]
