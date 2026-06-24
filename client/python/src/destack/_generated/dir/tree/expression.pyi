# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

from destack._impl.dir.tree.expression import (
    ExpressionImpl,
)

import destack._generated.core.string
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

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionBlock(ExpressionImpl):
    """Block of Expressions."""

    block: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["block"] = "block"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionLabel(ExpressionImpl):
    """Label statement (like `label: stmt` in JavaScript)."""

    label: destack._generated.core.string.StringId
    body: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["label"] = "label"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionImport(ExpressionImpl):
    """An Import is an import declaration for dependency management."""

    form: destack._generated.dir.tree.dependency.DependencyForm
    target: destack._generated.core.string.StringId
    items: Sequence[destack._generated.dir.tree.node.LocalNodeId] | None
    attributes: destack._generated.dir.tree.import_.ImportAttributeClause | None
    kind: typing.Literal["import"] = "import"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionExport(ExpressionImpl):
    """An Export is an explicit export declaration for dependency management."""

    form: destack._generated.dir.tree.dependency.DependencyForm
    target: destack._generated.core.string.StringId | None
    items: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    attributes: destack._generated.dir.tree.import_.ImportAttributeClause | None
    kind: typing.Literal["export"] = "export"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionLet(ExpressionImpl):
    """Let binding for mutable and immutable variables."""

    kind_value: LetKind
    export: destack._generated.dir.tree.dependency.ExportKind | None
    mutability: destack._generated.dir.tree.node.Mutability
    declarators: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    is_ambient: bool
    is_shared: bool
    kind: typing.Literal["let"] = "let"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionLetElse(ExpressionImpl):
    """Let-else binding with an early-exit branch."""

    kind_value: LetKind
    mutability: destack._generated.dir.tree.node.Mutability
    declarator: destack._generated.dir.tree.node.LocalNodeId
    else_branch: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["letElse"] = "letElse"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionUsing(ExpressionImpl):
    """Using binding for resources with deterministic disposal."""

    asynchrony: destack._generated.dir.tree.node.Asynchrony
    export: destack._generated.dir.tree.dependency.ExportKind | None
    declarators: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    is_ambient: bool
    kind: typing.Literal["using"] = "using"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionIf(ExpressionImpl):
    """If/then/else expression."""

    form: IfForm
    condition: Condition
    then_expression: destack._generated.dir.tree.node.LocalNodeId
    else_expression: destack._generated.dir.tree.node.LocalNodeId | None
    kind: typing.Literal["if"] = "if"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionWhile(ExpressionImpl):
    """A While is while or do-while loop."""

    form: WhileForm
    condition: destack._generated.dir.tree.node.LocalNodeId
    body: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["while"] = "while"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionForEach(ExpressionImpl):
    """A ForEach is a for loop over an iterator with a binding."""

    asynchrony: destack._generated.dir.tree.node.Asynchrony
    operator: ForEachOperator
    binding: ForEachBinding
    iterator: destack._generated.dir.tree.node.LocalNodeId
    body: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["forEach"] = "forEach"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionFor(ExpressionImpl):
    """A For is a for loop with the traditional three-part (initialization, condition, increment)."""

    initialization: destack._generated.dir.tree.node.LocalNodeId | None
    condition: destack._generated.dir.tree.node.LocalNodeId | None
    increment: destack._generated.dir.tree.node.LocalNodeId | None
    body: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["for"] = "for"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionLoop(ExpressionImpl):
    """A Loop is an unconditional loop."""

    body: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["loop"] = "loop"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionTry(ExpressionImpl):
    """A Try is a try/catch/finally expression."""

    body: destack._generated.dir.tree.node.LocalNodeId
    catch: destack._generated.dir.tree.node.LocalNodeId | None
    finally_: destack._generated.dir.tree.node.LocalNodeId | None
    kind: typing.Literal["try"] = "try"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionMatch(ExpressionImpl):
    """A Match is a match expression with case patterns."""

    form: destack._generated.dir.tree.match.MatchForm
    value: destack._generated.dir.tree.node.LocalNodeId
    cases: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["match"] = "match"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionBreak(ExpressionImpl):
    """A break statement."""

    label: destack._generated.core.string.StringId | None
    value: destack._generated.dir.tree.node.LocalNodeId | None
    kind: typing.Literal["break"] = "break"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionContinue(ExpressionImpl):
    """A continue statement."""

    label: destack._generated.core.string.StringId | None
    kind: typing.Literal["continue"] = "continue"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionAwait(ExpressionImpl):
    """Await an expression."""

    expression: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["await"] = "await"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionAwaitMaybe(ExpressionImpl):
    """Await an expression with immediate error propagation (`await? expr`)."""

    expression: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["awaitMaybe"] = "awaitMaybe"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionAwaitMust(ExpressionImpl):
    """Await an expression with immediate trapping error propagation (`await! expr`)."""

    expression: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["awaitMust"] = "awaitMust"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionYield(ExpressionImpl):
    """Yield an expression."""

    cardinality: YieldCardinality
    value: destack._generated.dir.tree.node.LocalNodeId | None
    kind: typing.Literal["yield"] = "yield"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionThrow(ExpressionImpl):
    """Throw an expression."""

    value: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["throw"] = "throw"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionReturn(ExpressionImpl):
    """Return an expression."""

    value: destack._generated.dir.tree.node.LocalNodeId | None
    kind: typing.Literal["return"] = "return"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionIdentifier(ExpressionImpl):
    """Bare identifier reference."""

    name: destack._generated.core.string.StringId
    kind: typing.Literal["identifier"] = "identifier"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionPrivateIdentifier(ExpressionImpl):
    """Private identifier (JavaScript/TypeScript)."""

    name: destack._generated.core.string.StringId
    kind: typing.Literal["privateIdentifier"] = "privateIdentifier"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionThis(ExpressionImpl):
    """This reference (value or type context)."""

    kind: typing.Literal["this"] = "this"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionSuper(ExpressionImpl):
    """Super reference (value context)."""

    kind: typing.Literal["super"] = "super"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionImportMeta(ExpressionImpl):
    """Import meta intrinsic value."""

    kind: typing.Literal["importMeta"] = "importMeta"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionImportSource(ExpressionImpl):
    """Import source intrinsic value."""

    kind: typing.Literal["importSource"] = "importSource"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionScalarLiteral(ExpressionImpl):
    """Literal scalar value."""

    scalar_literal: destack._generated.dir.tree.literal.ScalarLiteral
    kind: typing.Literal["scalarLiteral"] = "scalarLiteral"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionRangeExpression(ExpressionImpl):
    """Range expression."""

    start: destack._generated.dir.tree.node.LocalNodeId | None
    end: destack._generated.dir.tree.node.LocalNodeId | None
    end_kind: destack._generated.dir.tree.operator.RangeEnd
    kind: typing.Literal["rangeExpression"] = "rangeExpression"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionTemplateExpression(ExpressionImpl):
    """Template expression. May include interpolation arguments."""

    value: destack._generated.dir.tree.literal.TemplateLiteral
    kind: typing.Literal["templateExpression"] = "templateExpression"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionTaggedTemplateExpression(ExpressionImpl):
    """Tagged template expression. May include interpolation arguments."""

    tag: destack._generated.dir.tree.node.LocalNodeId
    generic_arguments: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    value: destack._generated.dir.tree.literal.TemplateLiteral
    kind: typing.Literal["taggedTemplateExpression"] = "taggedTemplateExpression"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionArrayExpression(ExpressionImpl):
    """An ArrayExpression constructs an array of homogeneous elements."""

    elements: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["arrayExpression"] = "arrayExpression"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionFixedArrayExpression(ExpressionImpl):
    """A FixedArrayExpression constructs a fixed-length array by repeating one value."""

    # the repeated value expression
    value: destack._generated.dir.tree.node.LocalNodeId
    # the fixed array length expression
    length: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["fixedArrayExpression"] = "fixedArrayExpression"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionTupleExpression(ExpressionImpl):
    """A TupleExpression constructs an anonymous tuple of heterogeneous elements."""

    elements: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["tupleExpression"] = "tupleExpression"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionSequenceExpression(ExpressionImpl):
    """A SequenceExpression is the JavaScript/TypeScript comma operator."""

    expressions: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["sequenceExpression"] = "sequenceExpression"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionObjectExpression(ExpressionImpl):
    """An ObjectExpression constructs an object with heterogeneous fields."""

    properties: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["objectExpression"] = "objectExpression"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionStructExpression(ExpressionImpl):
    """A StructExpression constructs a nominal value with named fields."""

    ty: destack._generated.dir.tree.node.LocalNodeId
    properties: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["structExpression"] = "structExpression"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionTreeExpression(ExpressionImpl):
    """A TreeExpression constructs a tree fragment with arguments and children."""

    left: destack._generated.dir.tree.node.LocalNodeId | None
    generic_arguments: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    arguments: Sequence[destack._generated.dir.tree.node.LocalNodeId] | None
    elements: Sequence[destack._generated.dir.tree.node.LocalNodeId] | None
    kind: typing.Literal["treeExpression"] = "treeExpression"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionParenthesized(ExpressionImpl):
    """Parenthesized expression."""

    expression: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["parenthesized"] = "parenthesized"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionType(ExpressionImpl):
    """Type expression used as a runtime type value."""

    value: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["type"] = "type"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionComptime(ExpressionImpl):
    """Compile time evaluated expression."""

    body: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["comptime"] = "comptime"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionAs(ExpressionImpl):
    """TypeScript-style `as` assertion."""

    expression: destack._generated.dir.tree.node.LocalNodeId
    target_type: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["as"] = "as"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionSatisfies(ExpressionImpl):
    """TypeScript-style `satisfies` expression."""

    expression: destack._generated.dir.tree.node.LocalNodeId
    target_type: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["satisfies"] = "satisfies"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionIs(ExpressionImpl):
    """Runtime type guard."""

    value: destack._generated.dir.tree.node.LocalNodeId
    target_type: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["is"] = "is"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionInstanceOf(ExpressionImpl):
    """Runtime constructor guard."""

    value: destack._generated.dir.tree.node.LocalNodeId
    target: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["instanceOf"] = "instanceOf"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionUnary(ExpressionImpl):
    """Unary operation (prefix or postfix)."""

    operator: destack._generated.dir.tree.operator.UnaryOperator
    right: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["unary"] = "unary"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionMoveOf(ExpressionImpl):
    """Move operation (e.g., `^x`)."""

    mutability: destack._generated.dir.tree.node.Mutability | None
    variance: VarianceBound | None
    right: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["moveOf"] = "moveOf"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionBorrowOf(ExpressionImpl):
    """Borrow operation (e.g., `&x`)."""

    mutability: destack._generated.dir.tree.node.Mutability | None
    variance: VarianceBound | None
    right: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["borrowOf"] = "borrowOf"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionMember(ExpressionImpl):
    """Member access."""

    left: destack._generated.dir.tree.node.LocalNodeId
    name: destack._generated.core.string.StringId | None
    kind: typing.Literal["member"] = "member"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionPrivateMember(ExpressionImpl):
    """Private member access."""

    left: destack._generated.dir.tree.node.LocalNodeId
    name: destack._generated.core.string.StringId | None
    kind: typing.Literal["privateMember"] = "privateMember"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionIndex(ExpressionImpl):
    """Index into a receiver expression."""

    position: PostfixPosition
    left: destack._generated.dir.tree.node.LocalNodeId
    index: destack._generated.dir.tree.node.LocalNodeId | None
    kind: typing.Literal["index"] = "index"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionInstantiation(ExpressionImpl):
    """Instantiation expression (TypeScript)."""

    left: destack._generated.dir.tree.node.LocalNodeId
    generic_arguments: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["instantiation"] = "instantiation"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionCall(ExpressionImpl):
    """A Call is call to a function OR an instantiation of a tuple type."""

    position: PostfixPosition
    left: destack._generated.dir.tree.node.LocalNodeId
    generic_arguments: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    arguments: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["call"] = "call"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionNew(ExpressionImpl):
    """New constructor call."""

    ty: destack._generated.dir.tree.node.LocalNodeId
    arguments: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["new"] = "new"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionNewMaybe(ExpressionImpl):
    """Fallible new constructor call with immediate allocation failure propagation."""

    ty: destack._generated.dir.tree.node.LocalNodeId
    arguments: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["newMaybe"] = "newMaybe"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionMaybe(ExpressionImpl):
    """Maybe unwrap an expression with `?` and propagate."""

    position: PostfixPosition
    left: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["maybe"] = "maybe"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionMust(ExpressionImpl):
    """Force unwrap an expression with `!` and propagate."""

    position: PostfixPosition
    left: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["must"] = "must"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionBinary(ExpressionImpl):
    """Binary operation."""

    left: destack._generated.dir.tree.node.LocalNodeId
    operator: destack._generated.dir.tree.operator.BinaryOperator
    right: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["binary"] = "binary"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionAssign(ExpressionImpl):
    """Assignment operation."""

    left: destack._generated.dir.tree.node.LocalNodeId
    operator: destack._generated.dir.tree.operator.AssignOperator
    right: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["assign"] = "assign"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionDebugger(ExpressionImpl):
    """Debugger statement."""

    kind: typing.Literal["debugger"] = "debugger"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionMissing(ExpressionImpl):
    """Missing expression child."""

    kind: typing.Literal["missing"] = "missing"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionStub(ExpressionImpl):
    """Stub placeholder."""

    kind: typing.Literal["stub"] = "stub"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExpressionError(ExpressionImpl):
    """Error placeholder."""

    kind: typing.Literal["error"] = "error"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

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

def encode_expression(writer: BinaryWriter, value: Expression) -> None: ...
def decode_expression(reader: BinaryReader) -> Expression: ...
def to_json_expression(value: Expression) -> Json: ...
def from_json_expression(value: Json) -> Expression: ...

"""The kind of a let or const binding."""
LetKind: typing.TypeAlias = typing.Literal["let"] | typing.Literal["const"]

def encode_let_kind(writer: BinaryWriter, value: LetKind) -> None: ...
def decode_let_kind(reader: BinaryReader) -> LetKind: ...
def to_json_let_kind(value: LetKind) -> Json: ...
def from_json_let_kind(value: Json) -> LetKind: ...

"""The style of if expression."""
IfForm: typing.TypeAlias = typing.Literal["if"] | typing.Literal["ternary"]

def encode_if_form(writer: BinaryWriter, value: IfForm) -> None: ...
def decode_if_form(reader: BinaryReader) -> IfForm: ...
def to_json_if_form(value: IfForm) -> Json: ...
def from_json_if_form(value: Json) -> IfForm: ...

@dataclass(frozen=True, slots=True)
class Condition:
    """A left-to-right condition."""

    # the operands joined by short-circuiting `&&`
    operands: Sequence[ConditionOperand]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Condition: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Condition: ...

def encode_condition(writer: BinaryWriter, value: Condition) -> None: ...
def decode_condition(reader: BinaryReader) -> Condition: ...
def to_json_condition(value: Condition) -> Json: ...
def from_json_condition(value: Json) -> Condition: ...

@dataclass(frozen=True, slots=True)
class ConditionOperandExpression:
    """A regular condition expression."""

    condition: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["expression"] = "expression"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

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

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""One operand in a condition."""
ConditionOperand: typing.TypeAlias = (
    ConditionOperandExpression | ConditionOperandBinding
)

def encode_condition_operand(writer: BinaryWriter, value: ConditionOperand) -> None: ...
def decode_condition_operand(reader: BinaryReader) -> ConditionOperand: ...
def to_json_condition_operand(value: ConditionOperand) -> Json: ...
def from_json_condition_operand(value: Json) -> ConditionOperand: ...

"""The kind of a while expression."""
WhileForm: typing.TypeAlias = typing.Literal["while"] | typing.Literal["doWhile"]

def encode_while_form(writer: BinaryWriter, value: WhileForm) -> None: ...
def decode_while_form(reader: BinaryReader) -> WhileForm: ...
def to_json_while_form(value: WhileForm) -> Json: ...
def from_json_while_form(value: Json) -> WhileForm: ...

"""The kind of a for each expression."""
ForEachOperator: typing.TypeAlias = typing.Literal["of"] | typing.Literal["in"]

def encode_for_each_operator(writer: BinaryWriter, value: ForEachOperator) -> None: ...
def decode_for_each_operator(reader: BinaryReader) -> ForEachOperator: ...
def to_json_for_each_operator(value: ForEachOperator) -> Json: ...
def from_json_for_each_operator(value: Json) -> ForEachOperator: ...

@dataclass(frozen=True, slots=True)
class ForEachBindingPattern:
    """Regular pattern binding."""

    pattern: destack._generated.dir.tree.node.LocalNodeId
    keyword: BindingKeyword | None
    kind: typing.Literal["pattern"] = "pattern"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ForEachBindingUsing:
    """Using binding with optional async disposal."""

    asynchrony: destack._generated.dir.tree.node.Asynchrony
    pattern: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["using"] = "using"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""The binding in a for each expression."""
ForEachBinding: typing.TypeAlias = ForEachBindingPattern | ForEachBindingUsing

def encode_for_each_binding(writer: BinaryWriter, value: ForEachBinding) -> None: ...
def decode_for_each_binding(reader: BinaryReader) -> ForEachBinding: ...
def to_json_for_each_binding(value: ForEachBinding) -> Json: ...
def from_json_for_each_binding(value: Json) -> ForEachBinding: ...

"""The declaration keyword used by a for each pattern binding."""
BindingKeyword: typing.TypeAlias = typing.Literal["let"] | typing.Literal["const"]

def encode_binding_keyword(writer: BinaryWriter, value: BindingKeyword) -> None: ...
def decode_binding_keyword(reader: BinaryReader) -> BindingKeyword: ...
def to_json_binding_keyword(value: BindingKeyword) -> Json: ...
def from_json_binding_keyword(value: Json) -> BindingKeyword: ...

"""The cardinality of a yield expression."""
YieldCardinality: typing.TypeAlias = (
    typing.Literal["scalar"] | typing.Literal["generator"]
)

def encode_yield_cardinality(writer: BinaryWriter, value: YieldCardinality) -> None: ...
def decode_yield_cardinality(reader: BinaryReader) -> YieldCardinality: ...
def to_json_yield_cardinality(value: YieldCardinality) -> Json: ...
def from_json_yield_cardinality(value: Json) -> YieldCardinality: ...

"""A TypeBound is a type bound for a reference operation."""
VarianceBound: typing.TypeAlias = (
    typing.Literal["implements"] | typing.Literal["extends"] | typing.Literal["super"]
)

def encode_variance_bound(writer: BinaryWriter, value: VarianceBound) -> None: ...
def decode_variance_bound(reader: BinaryReader) -> VarianceBound: ...
def to_json_variance_bound(value: VarianceBound) -> Json: ...
def from_json_variance_bound(value: Json) -> VarianceBound: ...

"""The position of a postfix expression."""
PostfixPosition: typing.TypeAlias = (
    typing.Literal["direct"] | typing.Literal["indirect"]
)

def encode_postfix_position(writer: BinaryWriter, value: PostfixPosition) -> None: ...
def decode_postfix_position(reader: BinaryReader) -> PostfixPosition: ...
def to_json_postfix_position(value: PostfixPosition) -> Json: ...
def from_json_postfix_position(value: Json) -> PostfixPosition: ...

@dataclass(frozen=True, slots=True)
class Catch:
    """A catch branch."""

    # the optional catch pattern
    pattern: destack._generated.dir.tree.node.LocalNodeId | None
    # the optional catch pattern type
    ty: destack._generated.dir.tree.node.LocalNodeId | None
    # the catch body
    body: destack._generated.dir.tree.node.LocalNodeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Catch: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Catch: ...

def encode_catch(writer: BinaryWriter, value: Catch) -> None: ...
def decode_catch(reader: BinaryReader) -> Catch: ...
def to_json_catch(value: Catch) -> Json: ...
def from_json_catch(value: Json) -> Catch: ...

@dataclass(frozen=True, slots=True)
class WhereClause:
    """A WhereClause is a single clause in a where type declaration."""

    # the target type to constrain (like `T` in `T: int32`)
    left: destack._generated.dir.tree.node.LocalNodeId
    # the constraint type (like `int32` in `T: int32`)
    right: destack._generated.dir.tree.node.LocalNodeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> WhereClause: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> WhereClause: ...

def encode_where_clause(writer: BinaryWriter, value: WhereClause) -> None: ...
def decode_where_clause(reader: BinaryReader) -> WhereClause: ...
def to_json_where_clause(value: WhereClause) -> Json: ...
def from_json_where_clause(value: Json) -> WhereClause: ...

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
]
