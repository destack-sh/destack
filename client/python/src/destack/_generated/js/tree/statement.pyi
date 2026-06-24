# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.string
import destack._generated.js.tree.dependency
import destack._generated.js.tree.node
import destack._generated.js.tree.operator
import destack._generated.source.file.model.module

@dataclass(frozen=True, slots=True)
class StatementImport:
    """Import items (including type items)."""

    form: destack._generated.js.tree.dependency.DependencyForm
    target: destack._generated.core.string.StringId
    target_module: destack._generated.source.file.model.module.ModuleId | None
    items: Sequence[destack._generated.js.tree.node.LocalNodeId] | None
    attributes: DependencyAttributeClause | None
    kind: typing.Literal["import"] = "import"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class StatementExport:
    """Export items (including type items)."""

    form: destack._generated.js.tree.dependency.DependencyForm
    target: destack._generated.core.string.StringId | None
    target_module: destack._generated.source.file.model.module.ModuleId | None
    items: Sequence[destack._generated.js.tree.node.LocalNodeId]
    attributes: DependencyAttributeClause | None
    kind: typing.Literal["export"] = "export"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class StatementExportValue:
    """Export value."""

    value: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["exportValue"] = "exportValue"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class StatementDeclaration:
    """Declaration statement."""

    declaration: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["declaration"] = "declaration"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class StatementBlock:
    """Block of statements."""

    block: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["block"] = "block"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class StatementLabelled:
    """Labelled statement (like `label: stmt`)."""

    label: destack._generated.core.string.StringId
    body: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["labelled"] = "labelled"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class StatementLet:
    """Let binding."""

    export: destack._generated.js.tree.dependency.DependencyBinding | None
    is_ambient: bool
    mutability: destack._generated.js.tree.node.Mutability
    declarators: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["let"] = "let"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class StatementVar:
    """Var binding."""

    export: destack._generated.js.tree.dependency.DependencyBinding | None
    is_ambient: bool
    declarators: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["var"] = "var"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class StatementUsing:
    """Using binding."""

    asynchrony: destack._generated.js.tree.node.Asynchrony
    export: destack._generated.js.tree.dependency.DependencyBinding | None
    is_ambient: bool
    declarators: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["using"] = "using"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class StatementAssign:
    """Assignment operation."""

    left: destack._generated.js.tree.node.LocalNodeId
    operator: destack._generated.js.tree.operator.AssignOperator
    right: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["assign"] = "assign"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class StatementExpression:
    """Expression statement."""

    expression: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["expression"] = "expression"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class StatementIf:
    """If statement."""

    condition: destack._generated.js.tree.node.LocalNodeId
    then_block: destack._generated.js.tree.node.LocalNodeId
    else_block: destack._generated.js.tree.node.LocalNodeId | None
    kind: typing.Literal["if"] = "if"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class StatementWhile:
    """While statement."""

    condition: destack._generated.js.tree.node.LocalNodeId
    body: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["while"] = "while"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class StatementDoWhile:
    """Do while statement."""

    body: destack._generated.js.tree.node.LocalNodeId
    condition: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["doWhile"] = "doWhile"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class StatementFor:
    """For statement."""

    initialization: ForInitialization | None
    condition: destack._generated.js.tree.node.LocalNodeId | None
    increment: destack._generated.js.tree.node.LocalNodeId | None
    body: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["for"] = "for"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class StatementForIn:
    """For in statement."""

    keyword: BindingKeyword | None
    pattern: destack._generated.js.tree.node.LocalNodeId
    iterator: destack._generated.js.tree.node.LocalNodeId
    body: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["forIn"] = "forIn"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class StatementForOf:
    """For of statement."""

    asynchrony: destack._generated.js.tree.node.Asynchrony
    keyword: BindingKeyword | None
    pattern: destack._generated.js.tree.node.LocalNodeId
    iterator: destack._generated.js.tree.node.LocalNodeId
    body: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["forOf"] = "forOf"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class StatementSwitch:
    """Switch statement."""

    value: destack._generated.js.tree.node.LocalNodeId
    cases: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["switch"] = "switch"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class StatementTry:
    """Try statement."""

    try_block: destack._generated.js.tree.node.LocalNodeId
    catch_clause: destack._generated.js.tree.node.LocalNodeId | None
    finally_block: destack._generated.js.tree.node.LocalNodeId | None
    kind: typing.Literal["try"] = "try"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class StatementThrow:
    """Throw statement."""

    value: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["throw"] = "throw"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class StatementContinue:
    """Continue statement."""

    label: destack._generated.core.string.StringId | None
    kind: typing.Literal["continue"] = "continue"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class StatementBreak:
    """Break statement."""

    label: destack._generated.core.string.StringId | None
    kind: typing.Literal["break"] = "break"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class StatementReturn:
    """Return statement."""

    value: destack._generated.js.tree.node.LocalNodeId | None
    kind: typing.Literal["return"] = "return"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class StatementDebugger:
    """Debugger statement."""

    kind: typing.Literal["debugger"] = "debugger"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""A Statement is a JS/TS top-level statement in some container/block."""
Statement: typing.TypeAlias = (
    StatementImport
    | StatementExport
    | StatementExportValue
    | StatementDeclaration
    | StatementBlock
    | StatementLabelled
    | StatementLet
    | StatementVar
    | StatementUsing
    | StatementAssign
    | StatementExpression
    | StatementIf
    | StatementWhile
    | StatementDoWhile
    | StatementFor
    | StatementForIn
    | StatementForOf
    | StatementSwitch
    | StatementTry
    | StatementThrow
    | StatementContinue
    | StatementBreak
    | StatementReturn
    | StatementDebugger
)

def encode_statement(writer: BinaryWriter, value: Statement) -> None: ...
def decode_statement(reader: BinaryReader) -> Statement: ...
def to_json_statement(value: Statement) -> Json: ...
def from_json_statement(value: Json) -> Statement: ...

@dataclass(frozen=True, slots=True)
class DependencyAttributeClause:
    """One dependency attribute clause."""

    # the clause introducer
    kind: DependencyAttributeClauseKind
    # the attribute entries inside the clause body
    properties: Sequence[destack._generated.js.tree.node.LocalNodeId]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DependencyAttributeClause: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DependencyAttributeClause: ...

def encode_dependency_attribute_clause(
    writer: BinaryWriter, value: DependencyAttributeClause
) -> None: ...
def decode_dependency_attribute_clause(
    reader: BinaryReader,
) -> DependencyAttributeClause: ...
def to_json_dependency_attribute_clause(value: DependencyAttributeClause) -> Json: ...
def from_json_dependency_attribute_clause(value: Json) -> DependencyAttributeClause: ...

"""The kind of one dependency attribute clause."""
DependencyAttributeClauseKind: typing.TypeAlias = (
    typing.Literal["with"] | typing.Literal["assert"]
)

def encode_dependency_attribute_clause_kind(
    writer: BinaryWriter, value: DependencyAttributeClauseKind
) -> None: ...
def decode_dependency_attribute_clause_kind(
    reader: BinaryReader,
) -> DependencyAttributeClauseKind: ...
def to_json_dependency_attribute_clause_kind(
    value: DependencyAttributeClauseKind,
) -> Json: ...
def from_json_dependency_attribute_clause_kind(
    value: Json,
) -> DependencyAttributeClauseKind: ...

@dataclass(frozen=True, slots=True)
class ForInitializationExpression:
    """One expression initializer."""

    expression: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["expression"] = "expression"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ForInitializationDeclaration:
    """One declaration initializer."""

    keyword: BindingKeyword
    declarators: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["declaration"] = "declaration"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""The initializer of one for statement."""
ForInitialization: typing.TypeAlias = (
    ForInitializationExpression | ForInitializationDeclaration
)

def encode_for_initialization(
    writer: BinaryWriter, value: ForInitialization
) -> None: ...
def decode_for_initialization(reader: BinaryReader) -> ForInitialization: ...
def to_json_for_initialization(value: ForInitialization) -> Json: ...
def from_json_for_initialization(value: Json) -> ForInitialization: ...

"""The binding keyword used by a for each binding."""
BindingKeyword: typing.TypeAlias = (
    typing.Literal["var"] | typing.Literal["let"] | typing.Literal["const"]
)

def encode_binding_keyword(writer: BinaryWriter, value: BindingKeyword) -> None: ...
def decode_binding_keyword(reader: BinaryReader) -> BindingKeyword: ...
def to_json_binding_keyword(value: BindingKeyword) -> Json: ...
def from_json_binding_keyword(value: Json) -> BindingKeyword: ...

__all__ = [
    "Statement",
    "encode_statement",
    "decode_statement",
    "to_json_statement",
    "from_json_statement",
    "StatementImport",
    "StatementExport",
    "StatementExportValue",
    "StatementDeclaration",
    "StatementBlock",
    "StatementLabelled",
    "StatementLet",
    "StatementVar",
    "StatementUsing",
    "StatementAssign",
    "StatementExpression",
    "StatementIf",
    "StatementWhile",
    "StatementDoWhile",
    "StatementFor",
    "StatementForIn",
    "StatementForOf",
    "StatementSwitch",
    "StatementTry",
    "StatementThrow",
    "StatementContinue",
    "StatementBreak",
    "StatementReturn",
    "StatementDebugger",
    "DependencyAttributeClause",
    "encode_dependency_attribute_clause",
    "decode_dependency_attribute_clause",
    "to_json_dependency_attribute_clause",
    "from_json_dependency_attribute_clause",
    "DependencyAttributeClauseKind",
    "encode_dependency_attribute_clause_kind",
    "decode_dependency_attribute_clause_kind",
    "to_json_dependency_attribute_clause_kind",
    "from_json_dependency_attribute_clause_kind",
    "ForInitialization",
    "encode_for_initialization",
    "decode_for_initialization",
    "to_json_for_initialization",
    "from_json_for_initialization",
    "ForInitializationExpression",
    "ForInitializationDeclaration",
    "BindingKeyword",
    "encode_binding_keyword",
    "decode_binding_keyword",
    "to_json_binding_keyword",
    "from_json_binding_keyword",
]
