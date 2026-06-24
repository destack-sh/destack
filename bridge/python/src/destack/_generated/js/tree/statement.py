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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_statement(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_statement(self)


@dataclass(frozen=True, slots=True)
class StatementExport:
    """Export items (including type items)."""

    form: destack._generated.js.tree.dependency.DependencyForm
    target: destack._generated.core.string.StringId | None
    target_module: destack._generated.source.file.model.module.ModuleId | None
    items: Sequence[destack._generated.js.tree.node.LocalNodeId]
    attributes: DependencyAttributeClause | None
    kind: typing.Literal["export"] = "export"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_statement(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_statement(self)


@dataclass(frozen=True, slots=True)
class StatementExportValue:
    """Export value."""

    value: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["exportValue"] = "exportValue"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_statement(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_statement(self)


@dataclass(frozen=True, slots=True)
class StatementDeclaration:
    """Declaration statement."""

    declaration: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["declaration"] = "declaration"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_statement(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_statement(self)


@dataclass(frozen=True, slots=True)
class StatementBlock:
    """Block of statements."""

    block: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["block"] = "block"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_statement(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_statement(self)


@dataclass(frozen=True, slots=True)
class StatementLabelled:
    """Labelled statement (like `label: stmt`)."""

    label: destack._generated.core.string.StringId
    body: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["labelled"] = "labelled"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_statement(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_statement(self)


@dataclass(frozen=True, slots=True)
class StatementLet:
    """Let binding."""

    export: destack._generated.js.tree.dependency.DependencyBinding | None
    is_ambient: bool
    mutability: destack._generated.js.tree.node.Mutability
    declarators: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["let"] = "let"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_statement(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_statement(self)


@dataclass(frozen=True, slots=True)
class StatementVar:
    """Var binding."""

    export: destack._generated.js.tree.dependency.DependencyBinding | None
    is_ambient: bool
    declarators: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["var"] = "var"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_statement(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_statement(self)


@dataclass(frozen=True, slots=True)
class StatementUsing:
    """Using binding."""

    asynchrony: destack._generated.js.tree.node.Asynchrony
    export: destack._generated.js.tree.dependency.DependencyBinding | None
    is_ambient: bool
    declarators: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["using"] = "using"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_statement(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_statement(self)


@dataclass(frozen=True, slots=True)
class StatementAssign:
    """Assignment operation."""

    left: destack._generated.js.tree.node.LocalNodeId
    operator: destack._generated.js.tree.operator.AssignOperator
    right: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["assign"] = "assign"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_statement(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_statement(self)


@dataclass(frozen=True, slots=True)
class StatementExpression:
    """Expression statement."""

    expression: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["expression"] = "expression"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_statement(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_statement(self)


@dataclass(frozen=True, slots=True)
class StatementIf:
    """If statement."""

    condition: destack._generated.js.tree.node.LocalNodeId
    then_block: destack._generated.js.tree.node.LocalNodeId
    else_block: destack._generated.js.tree.node.LocalNodeId | None
    kind: typing.Literal["if"] = "if"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_statement(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_statement(self)


@dataclass(frozen=True, slots=True)
class StatementWhile:
    """While statement."""

    condition: destack._generated.js.tree.node.LocalNodeId
    body: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["while"] = "while"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_statement(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_statement(self)


@dataclass(frozen=True, slots=True)
class StatementDoWhile:
    """Do while statement."""

    body: destack._generated.js.tree.node.LocalNodeId
    condition: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["doWhile"] = "doWhile"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_statement(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_statement(self)


@dataclass(frozen=True, slots=True)
class StatementFor:
    """For statement."""

    initialization: ForInitialization | None
    condition: destack._generated.js.tree.node.LocalNodeId | None
    increment: destack._generated.js.tree.node.LocalNodeId | None
    body: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["for"] = "for"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_statement(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_statement(self)


@dataclass(frozen=True, slots=True)
class StatementForIn:
    """For in statement."""

    keyword: BindingKeyword | None
    pattern: destack._generated.js.tree.node.LocalNodeId
    iterator: destack._generated.js.tree.node.LocalNodeId
    body: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["forIn"] = "forIn"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_statement(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_statement(self)


@dataclass(frozen=True, slots=True)
class StatementForOf:
    """For of statement."""

    asynchrony: destack._generated.js.tree.node.Asynchrony
    keyword: BindingKeyword | None
    pattern: destack._generated.js.tree.node.LocalNodeId
    iterator: destack._generated.js.tree.node.LocalNodeId
    body: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["forOf"] = "forOf"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_statement(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_statement(self)


@dataclass(frozen=True, slots=True)
class StatementSwitch:
    """Switch statement."""

    value: destack._generated.js.tree.node.LocalNodeId
    cases: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["switch"] = "switch"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_statement(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_statement(self)


@dataclass(frozen=True, slots=True)
class StatementTry:
    """Try statement."""

    try_block: destack._generated.js.tree.node.LocalNodeId
    catch_clause: destack._generated.js.tree.node.LocalNodeId | None
    finally_block: destack._generated.js.tree.node.LocalNodeId | None
    kind: typing.Literal["try"] = "try"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_statement(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_statement(self)


@dataclass(frozen=True, slots=True)
class StatementThrow:
    """Throw statement."""

    value: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["throw"] = "throw"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_statement(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_statement(self)


@dataclass(frozen=True, slots=True)
class StatementContinue:
    """Continue statement."""

    label: destack._generated.core.string.StringId | None
    kind: typing.Literal["continue"] = "continue"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_statement(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_statement(self)


@dataclass(frozen=True, slots=True)
class StatementBreak:
    """Break statement."""

    label: destack._generated.core.string.StringId | None
    kind: typing.Literal["break"] = "break"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_statement(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_statement(self)


@dataclass(frozen=True, slots=True)
class StatementReturn:
    """Return statement."""

    value: destack._generated.js.tree.node.LocalNodeId | None
    kind: typing.Literal["return"] = "return"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_statement(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_statement(self)


@dataclass(frozen=True, slots=True)
class StatementDebugger:
    """Debugger statement."""

    kind: typing.Literal["debugger"] = "debugger"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_statement(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_statement(self)


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


def encode_statement(writer: BinaryWriter, value: Statement) -> None:
    """Encode one Statement."""
    if value.kind == "import":
        writer.write_unsigned(0)
        destack._generated.js.tree.dependency.encode_dependency_form(writer, value.form)
        destack._generated.core.string.encode_string_id(writer, value.target)
        if value.target_module is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.source.file.model.module.encode_module_id(
                writer, value.target_module
            )
        if value.items is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            writer.write_unsigned(len(value.items))
            for item_value_items_1 in value.items:
                destack._generated.js.tree.node.encode_local_node_id(
                    writer, item_value_items_1
                )
        if value.attributes is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            encode_dependency_attribute_clause(writer, value.attributes)
    elif value.kind == "export":
        writer.write_unsigned(1)
        destack._generated.js.tree.dependency.encode_dependency_form(writer, value.form)
        if value.target is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.core.string.encode_string_id(writer, value.target)
        if value.target_module is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.source.file.model.module.encode_module_id(
                writer, value.target_module
            )
        writer.write_unsigned(len(value.items))
        for item_value_items_0 in value.items:
            destack._generated.js.tree.node.encode_local_node_id(
                writer, item_value_items_0
            )
        if value.attributes is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            encode_dependency_attribute_clause(writer, value.attributes)
    elif value.kind == "exportValue":
        writer.write_unsigned(2)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.value)
    elif value.kind == "declaration":
        writer.write_unsigned(3)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.declaration)
    elif value.kind == "block":
        writer.write_unsigned(4)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.block)
    elif value.kind == "labelled":
        writer.write_unsigned(5)
        destack._generated.core.string.encode_string_id(writer, value.label)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.body)
    elif value.kind == "let":
        writer.write_unsigned(6)
        if value.export is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.js.tree.dependency.encode_dependency_binding(
                writer, value.export
            )
        writer.write_bool(value.is_ambient)
        destack._generated.js.tree.node.encode_mutability(writer, value.mutability)
        writer.write_unsigned(len(value.declarators))
        for item_value_declarators_0 in value.declarators:
            destack._generated.js.tree.node.encode_local_node_id(
                writer, item_value_declarators_0
            )
    elif value.kind == "var":
        writer.write_unsigned(7)
        if value.export is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.js.tree.dependency.encode_dependency_binding(
                writer, value.export
            )
        writer.write_bool(value.is_ambient)
        writer.write_unsigned(len(value.declarators))
        for item_value_declarators_0 in value.declarators:
            destack._generated.js.tree.node.encode_local_node_id(
                writer, item_value_declarators_0
            )
    elif value.kind == "using":
        writer.write_unsigned(8)
        destack._generated.js.tree.node.encode_asynchrony(writer, value.asynchrony)
        if value.export is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.js.tree.dependency.encode_dependency_binding(
                writer, value.export
            )
        writer.write_bool(value.is_ambient)
        writer.write_unsigned(len(value.declarators))
        for item_value_declarators_0 in value.declarators:
            destack._generated.js.tree.node.encode_local_node_id(
                writer, item_value_declarators_0
            )
    elif value.kind == "assign":
        writer.write_unsigned(9)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.left)
        destack._generated.js.tree.operator.encode_assign_operator(
            writer, value.operator
        )
        destack._generated.js.tree.node.encode_local_node_id(writer, value.right)
    elif value.kind == "expression":
        writer.write_unsigned(10)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.expression)
    elif value.kind == "if":
        writer.write_unsigned(11)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.condition)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.then_block)
        if value.else_block is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.js.tree.node.encode_local_node_id(
                writer, value.else_block
            )
    elif value.kind == "while":
        writer.write_unsigned(12)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.condition)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.body)
    elif value.kind == "doWhile":
        writer.write_unsigned(13)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.body)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.condition)
    elif value.kind == "for":
        writer.write_unsigned(14)
        if value.initialization is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            encode_for_initialization(writer, value.initialization)
        if value.condition is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.js.tree.node.encode_local_node_id(
                writer, value.condition
            )
        if value.increment is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.js.tree.node.encode_local_node_id(
                writer, value.increment
            )
        destack._generated.js.tree.node.encode_local_node_id(writer, value.body)
    elif value.kind == "forIn":
        writer.write_unsigned(15)
        if value.keyword is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            encode_binding_keyword(writer, value.keyword)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.pattern)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.iterator)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.body)
    elif value.kind == "forOf":
        writer.write_unsigned(16)
        destack._generated.js.tree.node.encode_asynchrony(writer, value.asynchrony)
        if value.keyword is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            encode_binding_keyword(writer, value.keyword)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.pattern)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.iterator)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.body)
    elif value.kind == "switch":
        writer.write_unsigned(17)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.value)
        writer.write_unsigned(len(value.cases))
        for item_value_cases_0 in value.cases:
            destack._generated.js.tree.node.encode_local_node_id(
                writer, item_value_cases_0
            )
    elif value.kind == "try":
        writer.write_unsigned(18)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.try_block)
        if value.catch_clause is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.js.tree.node.encode_local_node_id(
                writer, value.catch_clause
            )
        if value.finally_block is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.js.tree.node.encode_local_node_id(
                writer, value.finally_block
            )
    elif value.kind == "throw":
        writer.write_unsigned(19)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.value)
    elif value.kind == "continue":
        writer.write_unsigned(20)
        if value.label is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.core.string.encode_string_id(writer, value.label)
    elif value.kind == "break":
        writer.write_unsigned(21)
        if value.label is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.core.string.encode_string_id(writer, value.label)
    elif value.kind == "return":
        writer.write_unsigned(22)
        if value.value is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.js.tree.node.encode_local_node_id(writer, value.value)
    elif value.kind == "debugger":
        writer.write_unsigned(23)
    else:
        raise SerdeError("unknown enum variant")


def decode_statement(reader: BinaryReader) -> Statement:
    """Decode one Statement."""
    variant = reader.read_number()

    if variant == 0:
        form = destack._generated.js.tree.dependency.decode_dependency_form(reader)
        target = destack._generated.core.string.decode_string_id(reader)
        target_module = reader.read_option(
            lambda: destack._generated.source.file.model.module.decode_module_id(reader)
        )
        items = reader.read_option(
            lambda: [
                destack._generated.js.tree.node.decode_local_node_id(reader)
                for _ in range(reader.read_number())
            ]
        )
        attributes = reader.read_option(
            lambda: decode_dependency_attribute_clause(reader)
        )

        return StatementImport(
            form=form,
            target=target,
            target_module=target_module,
            items=items,
            attributes=attributes,
        )
    elif variant == 1:
        form = destack._generated.js.tree.dependency.decode_dependency_form(reader)
        target = reader.read_option(
            lambda: destack._generated.core.string.decode_string_id(reader)
        )
        target_module = reader.read_option(
            lambda: destack._generated.source.file.model.module.decode_module_id(reader)
        )
        items = [
            destack._generated.js.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]
        attributes = reader.read_option(
            lambda: decode_dependency_attribute_clause(reader)
        )

        return StatementExport(
            form=form,
            target=target,
            target_module=target_module,
            items=items,
            attributes=attributes,
        )
    elif variant == 2:
        value_ = destack._generated.js.tree.node.decode_local_node_id(reader)

        return StatementExportValue(
            value=value_,
        )
    elif variant == 3:
        declaration = destack._generated.js.tree.node.decode_local_node_id(reader)

        return StatementDeclaration(
            declaration=declaration,
        )
    elif variant == 4:
        block = destack._generated.js.tree.node.decode_local_node_id(reader)

        return StatementBlock(
            block=block,
        )
    elif variant == 5:
        label = destack._generated.core.string.decode_string_id(reader)
        body = destack._generated.js.tree.node.decode_local_node_id(reader)

        return StatementLabelled(
            label=label,
            body=body,
        )
    elif variant == 6:
        export = reader.read_option(
            lambda: destack._generated.js.tree.dependency.decode_dependency_binding(
                reader
            )
        )
        is_ambient = reader.read_bool()
        mutability = destack._generated.js.tree.node.decode_mutability(reader)
        declarators = [
            destack._generated.js.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return StatementLet(
            export=export,
            is_ambient=is_ambient,
            mutability=mutability,
            declarators=declarators,
        )
    elif variant == 7:
        export = reader.read_option(
            lambda: destack._generated.js.tree.dependency.decode_dependency_binding(
                reader
            )
        )
        is_ambient = reader.read_bool()
        declarators = [
            destack._generated.js.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return StatementVar(
            export=export,
            is_ambient=is_ambient,
            declarators=declarators,
        )
    elif variant == 8:
        asynchrony = destack._generated.js.tree.node.decode_asynchrony(reader)
        export = reader.read_option(
            lambda: destack._generated.js.tree.dependency.decode_dependency_binding(
                reader
            )
        )
        is_ambient = reader.read_bool()
        declarators = [
            destack._generated.js.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return StatementUsing(
            asynchrony=asynchrony,
            export=export,
            is_ambient=is_ambient,
            declarators=declarators,
        )
    elif variant == 9:
        left = destack._generated.js.tree.node.decode_local_node_id(reader)
        operator = destack._generated.js.tree.operator.decode_assign_operator(reader)
        right = destack._generated.js.tree.node.decode_local_node_id(reader)

        return StatementAssign(
            left=left,
            operator=operator,
            right=right,
        )
    elif variant == 10:
        expression = destack._generated.js.tree.node.decode_local_node_id(reader)

        return StatementExpression(
            expression=expression,
        )
    elif variant == 11:
        condition = destack._generated.js.tree.node.decode_local_node_id(reader)
        then_block = destack._generated.js.tree.node.decode_local_node_id(reader)
        else_block = reader.read_option(
            lambda: destack._generated.js.tree.node.decode_local_node_id(reader)
        )

        return StatementIf(
            condition=condition,
            then_block=then_block,
            else_block=else_block,
        )
    elif variant == 12:
        condition = destack._generated.js.tree.node.decode_local_node_id(reader)
        body = destack._generated.js.tree.node.decode_local_node_id(reader)

        return StatementWhile(
            condition=condition,
            body=body,
        )
    elif variant == 13:
        body = destack._generated.js.tree.node.decode_local_node_id(reader)
        condition = destack._generated.js.tree.node.decode_local_node_id(reader)

        return StatementDoWhile(
            body=body,
            condition=condition,
        )
    elif variant == 14:
        initialization = reader.read_option(lambda: decode_for_initialization(reader))
        condition = reader.read_option(
            lambda: destack._generated.js.tree.node.decode_local_node_id(reader)
        )
        increment = reader.read_option(
            lambda: destack._generated.js.tree.node.decode_local_node_id(reader)
        )
        body = destack._generated.js.tree.node.decode_local_node_id(reader)

        return StatementFor(
            initialization=initialization,
            condition=condition,
            increment=increment,
            body=body,
        )
    elif variant == 15:
        keyword = reader.read_option(lambda: decode_binding_keyword(reader))
        pattern = destack._generated.js.tree.node.decode_local_node_id(reader)
        iterator = destack._generated.js.tree.node.decode_local_node_id(reader)
        body = destack._generated.js.tree.node.decode_local_node_id(reader)

        return StatementForIn(
            keyword=keyword,
            pattern=pattern,
            iterator=iterator,
            body=body,
        )
    elif variant == 16:
        asynchrony = destack._generated.js.tree.node.decode_asynchrony(reader)
        keyword = reader.read_option(lambda: decode_binding_keyword(reader))
        pattern = destack._generated.js.tree.node.decode_local_node_id(reader)
        iterator = destack._generated.js.tree.node.decode_local_node_id(reader)
        body = destack._generated.js.tree.node.decode_local_node_id(reader)

        return StatementForOf(
            asynchrony=asynchrony,
            keyword=keyword,
            pattern=pattern,
            iterator=iterator,
            body=body,
        )
    elif variant == 17:
        value_ = destack._generated.js.tree.node.decode_local_node_id(reader)
        cases = [
            destack._generated.js.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return StatementSwitch(
            value=value_,
            cases=cases,
        )
    elif variant == 18:
        try_block = destack._generated.js.tree.node.decode_local_node_id(reader)
        catch_clause = reader.read_option(
            lambda: destack._generated.js.tree.node.decode_local_node_id(reader)
        )
        finally_block = reader.read_option(
            lambda: destack._generated.js.tree.node.decode_local_node_id(reader)
        )

        return StatementTry(
            try_block=try_block,
            catch_clause=catch_clause,
            finally_block=finally_block,
        )
    elif variant == 19:
        value_ = destack._generated.js.tree.node.decode_local_node_id(reader)

        return StatementThrow(
            value=value_,
        )
    elif variant == 20:
        label = reader.read_option(
            lambda: destack._generated.core.string.decode_string_id(reader)
        )

        return StatementContinue(
            label=label,
        )
    elif variant == 21:
        label = reader.read_option(
            lambda: destack._generated.core.string.decode_string_id(reader)
        )

        return StatementBreak(
            label=label,
        )
    elif variant == 22:
        value_ = reader.read_option(
            lambda: destack._generated.js.tree.node.decode_local_node_id(reader)
        )

        return StatementReturn(
            value=value_,
        )
    elif variant == 23:
        return StatementDebugger()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_statement(value: Statement) -> Json:
    """Return one JSON value for one Statement."""
    if value.kind == "import":
        return {
            "kind": "import",
            "form": destack._generated.js.tree.dependency.to_json_dependency_form(
                value.form
            ),
            "target": destack._generated.core.string.to_json_string_id(value.target),
            **(
                {}
                if value.target_module is None
                else {
                    "targetModule": destack._generated.source.file.model.module.to_json_module_id(
                        value.target_module
                    )
                }
            ),
            **(
                {}
                if value.items is None
                else {
                    "items": [
                        destack._generated.js.tree.node.to_json_local_node_id(item_0)
                        for item_0 in value.items
                    ]
                }
            ),
            **(
                {}
                if value.attributes is None
                else {
                    "attributes": to_json_dependency_attribute_clause(value.attributes)
                }
            ),
        }
    elif value.kind == "export":
        return {
            "kind": "export",
            "form": destack._generated.js.tree.dependency.to_json_dependency_form(
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
            **(
                {}
                if value.target_module is None
                else {
                    "targetModule": destack._generated.source.file.model.module.to_json_module_id(
                        value.target_module
                    )
                }
            ),
            "items": [
                destack._generated.js.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.items
            ],
            **(
                {}
                if value.attributes is None
                else {
                    "attributes": to_json_dependency_attribute_clause(value.attributes)
                }
            ),
        }
    elif value.kind == "exportValue":
        return {
            "kind": "exportValue",
            "value": destack._generated.js.tree.node.to_json_local_node_id(value.value),
        }
    elif value.kind == "declaration":
        return {
            "kind": "declaration",
            "declaration": destack._generated.js.tree.node.to_json_local_node_id(
                value.declaration
            ),
        }
    elif value.kind == "block":
        return {
            "kind": "block",
            "block": destack._generated.js.tree.node.to_json_local_node_id(value.block),
        }
    elif value.kind == "labelled":
        return {
            "kind": "labelled",
            "label": destack._generated.core.string.to_json_string_id(value.label),
            "body": destack._generated.js.tree.node.to_json_local_node_id(value.body),
        }
    elif value.kind == "let":
        return {
            "kind": "let",
            **(
                {}
                if value.export is None
                else {
                    "export": destack._generated.js.tree.dependency.to_json_dependency_binding(
                        value.export
                    )
                }
            ),
            "isAmbient": value.is_ambient,
            "mutability": destack._generated.js.tree.node.to_json_mutability(
                value.mutability
            ),
            "declarators": [
                destack._generated.js.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.declarators
            ],
        }
    elif value.kind == "var":
        return {
            "kind": "var",
            **(
                {}
                if value.export is None
                else {
                    "export": destack._generated.js.tree.dependency.to_json_dependency_binding(
                        value.export
                    )
                }
            ),
            "isAmbient": value.is_ambient,
            "declarators": [
                destack._generated.js.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.declarators
            ],
        }
    elif value.kind == "using":
        return {
            "kind": "using",
            "asynchrony": destack._generated.js.tree.node.to_json_asynchrony(
                value.asynchrony
            ),
            **(
                {}
                if value.export is None
                else {
                    "export": destack._generated.js.tree.dependency.to_json_dependency_binding(
                        value.export
                    )
                }
            ),
            "isAmbient": value.is_ambient,
            "declarators": [
                destack._generated.js.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.declarators
            ],
        }
    elif value.kind == "assign":
        return {
            "kind": "assign",
            "left": destack._generated.js.tree.node.to_json_local_node_id(value.left),
            "operator": destack._generated.js.tree.operator.to_json_assign_operator(
                value.operator
            ),
            "right": destack._generated.js.tree.node.to_json_local_node_id(value.right),
        }
    elif value.kind == "expression":
        return {
            "kind": "expression",
            "expression": destack._generated.js.tree.node.to_json_local_node_id(
                value.expression
            ),
        }
    elif value.kind == "if":
        return {
            "kind": "if",
            "condition": destack._generated.js.tree.node.to_json_local_node_id(
                value.condition
            ),
            "thenBlock": destack._generated.js.tree.node.to_json_local_node_id(
                value.then_block
            ),
            **(
                {}
                if value.else_block is None
                else {
                    "elseBlock": destack._generated.js.tree.node.to_json_local_node_id(
                        value.else_block
                    )
                }
            ),
        }
    elif value.kind == "while":
        return {
            "kind": "while",
            "condition": destack._generated.js.tree.node.to_json_local_node_id(
                value.condition
            ),
            "body": destack._generated.js.tree.node.to_json_local_node_id(value.body),
        }
    elif value.kind == "doWhile":
        return {
            "kind": "doWhile",
            "body": destack._generated.js.tree.node.to_json_local_node_id(value.body),
            "condition": destack._generated.js.tree.node.to_json_local_node_id(
                value.condition
            ),
        }
    elif value.kind == "for":
        return {
            "kind": "for",
            **(
                {}
                if value.initialization is None
                else {
                    "initialization": to_json_for_initialization(value.initialization)
                }
            ),
            **(
                {}
                if value.condition is None
                else {
                    "condition": destack._generated.js.tree.node.to_json_local_node_id(
                        value.condition
                    )
                }
            ),
            **(
                {}
                if value.increment is None
                else {
                    "increment": destack._generated.js.tree.node.to_json_local_node_id(
                        value.increment
                    )
                }
            ),
            "body": destack._generated.js.tree.node.to_json_local_node_id(value.body),
        }
    elif value.kind == "forIn":
        return {
            "kind": "forIn",
            **(
                {}
                if value.keyword is None
                else {"keyword": to_json_binding_keyword(value.keyword)}
            ),
            "pattern": destack._generated.js.tree.node.to_json_local_node_id(
                value.pattern
            ),
            "iterator": destack._generated.js.tree.node.to_json_local_node_id(
                value.iterator
            ),
            "body": destack._generated.js.tree.node.to_json_local_node_id(value.body),
        }
    elif value.kind == "forOf":
        return {
            "kind": "forOf",
            "asynchrony": destack._generated.js.tree.node.to_json_asynchrony(
                value.asynchrony
            ),
            **(
                {}
                if value.keyword is None
                else {"keyword": to_json_binding_keyword(value.keyword)}
            ),
            "pattern": destack._generated.js.tree.node.to_json_local_node_id(
                value.pattern
            ),
            "iterator": destack._generated.js.tree.node.to_json_local_node_id(
                value.iterator
            ),
            "body": destack._generated.js.tree.node.to_json_local_node_id(value.body),
        }
    elif value.kind == "switch":
        return {
            "kind": "switch",
            "value": destack._generated.js.tree.node.to_json_local_node_id(value.value),
            "cases": [
                destack._generated.js.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.cases
            ],
        }
    elif value.kind == "try":
        return {
            "kind": "try",
            "tryBlock": destack._generated.js.tree.node.to_json_local_node_id(
                value.try_block
            ),
            **(
                {}
                if value.catch_clause is None
                else {
                    "catchClause": destack._generated.js.tree.node.to_json_local_node_id(
                        value.catch_clause
                    )
                }
            ),
            **(
                {}
                if value.finally_block is None
                else {
                    "finallyBlock": destack._generated.js.tree.node.to_json_local_node_id(
                        value.finally_block
                    )
                }
            ),
        }
    elif value.kind == "throw":
        return {
            "kind": "throw",
            "value": destack._generated.js.tree.node.to_json_local_node_id(value.value),
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
        }
    elif value.kind == "return":
        return {
            "kind": "return",
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
    elif value.kind == "debugger":
        return {
            "kind": "debugger",
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_statement(value: Json) -> Statement:
    """Return one Statement from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "import":
        return StatementImport(
            form=destack._generated.js.tree.dependency.from_json_dependency_form(
                json_field(object_, "form")
            ),
            target=destack._generated.core.string.from_json_string_id(
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
            items=json_optional(
                object_,
                "items",
                lambda value: [
                    destack._generated.js.tree.node.from_json_local_node_id(item_0)
                    for item_0 in json_array(value)
                ],
            ),
            attributes=json_optional(
                object_,
                "attributes",
                lambda value: from_json_dependency_attribute_clause(value),
            ),
        )
    elif kind == "export":
        return StatementExport(
            form=destack._generated.js.tree.dependency.from_json_dependency_form(
                json_field(object_, "form")
            ),
            target=json_optional(
                object_,
                "target",
                lambda value: destack._generated.core.string.from_json_string_id(value),
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
            items=[
                destack._generated.js.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "items"))
            ],
            attributes=json_optional(
                object_,
                "attributes",
                lambda value: from_json_dependency_attribute_clause(value),
            ),
        )
    elif kind == "exportValue":
        return StatementExportValue(
            value=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "value")
            ),
        )
    elif kind == "declaration":
        return StatementDeclaration(
            declaration=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "declaration")
            ),
        )
    elif kind == "block":
        return StatementBlock(
            block=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "block")
            ),
        )
    elif kind == "labelled":
        return StatementLabelled(
            label=destack._generated.core.string.from_json_string_id(
                json_field(object_, "label")
            ),
            body=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "body")
            ),
        )
    elif kind == "let":
        return StatementLet(
            export=json_optional(
                object_,
                "export",
                lambda value: (
                    destack._generated.js.tree.dependency.from_json_dependency_binding(
                        value
                    )
                ),
            ),
            is_ambient=json_bool(json_field(object_, "isAmbient")),
            mutability=destack._generated.js.tree.node.from_json_mutability(
                json_field(object_, "mutability")
            ),
            declarators=[
                destack._generated.js.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "declarators"))
            ],
        )
    elif kind == "var":
        return StatementVar(
            export=json_optional(
                object_,
                "export",
                lambda value: (
                    destack._generated.js.tree.dependency.from_json_dependency_binding(
                        value
                    )
                ),
            ),
            is_ambient=json_bool(json_field(object_, "isAmbient")),
            declarators=[
                destack._generated.js.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "declarators"))
            ],
        )
    elif kind == "using":
        return StatementUsing(
            asynchrony=destack._generated.js.tree.node.from_json_asynchrony(
                json_field(object_, "asynchrony")
            ),
            export=json_optional(
                object_,
                "export",
                lambda value: (
                    destack._generated.js.tree.dependency.from_json_dependency_binding(
                        value
                    )
                ),
            ),
            is_ambient=json_bool(json_field(object_, "isAmbient")),
            declarators=[
                destack._generated.js.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "declarators"))
            ],
        )
    elif kind == "assign":
        return StatementAssign(
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
    elif kind == "expression":
        return StatementExpression(
            expression=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "expression")
            ),
        )
    elif kind == "if":
        return StatementIf(
            condition=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "condition")
            ),
            then_block=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "thenBlock")
            ),
            else_block=json_optional(
                object_,
                "elseBlock",
                lambda value: destack._generated.js.tree.node.from_json_local_node_id(
                    value
                ),
            ),
        )
    elif kind == "while":
        return StatementWhile(
            condition=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "condition")
            ),
            body=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "body")
            ),
        )
    elif kind == "doWhile":
        return StatementDoWhile(
            body=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "body")
            ),
            condition=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "condition")
            ),
        )
    elif kind == "for":
        return StatementFor(
            initialization=json_optional(
                object_,
                "initialization",
                lambda value: from_json_for_initialization(value),
            ),
            condition=json_optional(
                object_,
                "condition",
                lambda value: destack._generated.js.tree.node.from_json_local_node_id(
                    value
                ),
            ),
            increment=json_optional(
                object_,
                "increment",
                lambda value: destack._generated.js.tree.node.from_json_local_node_id(
                    value
                ),
            ),
            body=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "body")
            ),
        )
    elif kind == "forIn":
        return StatementForIn(
            keyword=json_optional(
                object_, "keyword", lambda value: from_json_binding_keyword(value)
            ),
            pattern=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "pattern")
            ),
            iterator=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "iterator")
            ),
            body=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "body")
            ),
        )
    elif kind == "forOf":
        return StatementForOf(
            asynchrony=destack._generated.js.tree.node.from_json_asynchrony(
                json_field(object_, "asynchrony")
            ),
            keyword=json_optional(
                object_, "keyword", lambda value: from_json_binding_keyword(value)
            ),
            pattern=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "pattern")
            ),
            iterator=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "iterator")
            ),
            body=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "body")
            ),
        )
    elif kind == "switch":
        return StatementSwitch(
            value=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "value")
            ),
            cases=[
                destack._generated.js.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "cases"))
            ],
        )
    elif kind == "try":
        return StatementTry(
            try_block=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "tryBlock")
            ),
            catch_clause=json_optional(
                object_,
                "catchClause",
                lambda value: destack._generated.js.tree.node.from_json_local_node_id(
                    value
                ),
            ),
            finally_block=json_optional(
                object_,
                "finallyBlock",
                lambda value: destack._generated.js.tree.node.from_json_local_node_id(
                    value
                ),
            ),
        )
    elif kind == "throw":
        return StatementThrow(
            value=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "value")
            ),
        )
    elif kind == "continue":
        return StatementContinue(
            label=json_optional(
                object_,
                "label",
                lambda value: destack._generated.core.string.from_json_string_id(value),
            ),
        )
    elif kind == "break":
        return StatementBreak(
            label=json_optional(
                object_,
                "label",
                lambda value: destack._generated.core.string.from_json_string_id(value),
            ),
        )
    elif kind == "return":
        return StatementReturn(
            value=json_optional(
                object_,
                "value",
                lambda value: destack._generated.js.tree.node.from_json_local_node_id(
                    value
                ),
            ),
        )
    elif kind == "debugger":
        return StatementDebugger()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class DependencyAttributeClause:
    """One dependency attribute clause."""

    # the clause introducer
    kind: DependencyAttributeClauseKind
    # the attribute entries inside the clause body
    properties: Sequence[destack._generated.js.tree.node.LocalNodeId]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_dependency_attribute_clause(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DependencyAttributeClause:
        """Decode one DependencyAttributeClause."""
        return decode_dependency_attribute_clause(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_dependency_attribute_clause(self)

    @classmethod
    def from_json(cls, value: Json) -> DependencyAttributeClause:
        """Return one DependencyAttributeClause from one JSON value."""
        return from_json_dependency_attribute_clause(value)


def encode_dependency_attribute_clause(
    writer: BinaryWriter, value: DependencyAttributeClause
) -> None:
    """Encode one DependencyAttributeClause."""
    encode_dependency_attribute_clause_kind(writer, value.kind)
    writer.write_unsigned(len(value.properties))
    for item_value_properties_0 in value.properties:
        destack._generated.js.tree.node.encode_local_node_id(
            writer, item_value_properties_0
        )


def decode_dependency_attribute_clause(
    reader: BinaryReader,
) -> DependencyAttributeClause:
    """Decode one DependencyAttributeClause."""
    kind = decode_dependency_attribute_clause_kind(reader)
    properties = [
        destack._generated.js.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]

    return DependencyAttributeClause(
        kind=kind,
        properties=properties,
    )


def to_json_dependency_attribute_clause(value: DependencyAttributeClause) -> Json:
    """Return one JSON value for one DependencyAttributeClause."""
    return {
        "kind": to_json_dependency_attribute_clause_kind(value.kind),
        "properties": [
            destack._generated.js.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.properties
        ],
    }


def from_json_dependency_attribute_clause(value: Json) -> DependencyAttributeClause:
    """Return one DependencyAttributeClause from one JSON value."""
    object_ = json_object(value)

    return DependencyAttributeClause(
        kind=from_json_dependency_attribute_clause_kind(json_field(object_, "kind")),
        properties=[
            destack._generated.js.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "properties"))
        ],
    )


"""The kind of one dependency attribute clause."""
DependencyAttributeClauseKind: typing.TypeAlias = (
    typing.Literal["with"] | typing.Literal["assert"]
)


def encode_dependency_attribute_clause_kind(
    writer: BinaryWriter, value: DependencyAttributeClauseKind
) -> None:
    """Encode one DependencyAttributeClauseKind."""
    if value == "with":
        writer.write_unsigned(0)
    elif value == "assert":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_dependency_attribute_clause_kind(
    reader: BinaryReader,
) -> DependencyAttributeClauseKind:
    """Decode one DependencyAttributeClauseKind."""
    variant = reader.read_number()

    if variant == 0:
        return "with"
    elif variant == 1:
        return "assert"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_dependency_attribute_clause_kind(
    value: DependencyAttributeClauseKind,
) -> Json:
    """Return one JSON value for one DependencyAttributeClauseKind."""
    return value


def from_json_dependency_attribute_clause_kind(
    value: Json,
) -> DependencyAttributeClauseKind:
    """Return one DependencyAttributeClauseKind from one JSON value."""
    variant = json_string(value)

    if variant == "with":
        return "with"
    elif variant == "assert":
        return "assert"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class ForInitializationExpression:
    """One expression initializer."""

    expression: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["expression"] = "expression"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_for_initialization(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_for_initialization(self)


@dataclass(frozen=True, slots=True)
class ForInitializationDeclaration:
    """One declaration initializer."""

    keyword: BindingKeyword
    declarators: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["declaration"] = "declaration"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_for_initialization(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_for_initialization(self)


"""The initializer of one for statement."""
ForInitialization: typing.TypeAlias = (
    ForInitializationExpression | ForInitializationDeclaration
)


def encode_for_initialization(writer: BinaryWriter, value: ForInitialization) -> None:
    """Encode one ForInitialization."""
    if value.kind == "expression":
        writer.write_unsigned(0)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.expression)
    elif value.kind == "declaration":
        writer.write_unsigned(1)
        encode_binding_keyword(writer, value.keyword)
        writer.write_unsigned(len(value.declarators))
        for item_value_declarators_0 in value.declarators:
            destack._generated.js.tree.node.encode_local_node_id(
                writer, item_value_declarators_0
            )
    else:
        raise SerdeError("unknown enum variant")


def decode_for_initialization(reader: BinaryReader) -> ForInitialization:
    """Decode one ForInitialization."""
    variant = reader.read_number()

    if variant == 0:
        expression = destack._generated.js.tree.node.decode_local_node_id(reader)

        return ForInitializationExpression(expression=expression)
    elif variant == 1:
        keyword = decode_binding_keyword(reader)
        declarators = [
            destack._generated.js.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return ForInitializationDeclaration(
            keyword=keyword,
            declarators=declarators,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_for_initialization(value: ForInitialization) -> Json:
    """Return one JSON value for one ForInitialization."""
    if value.kind == "expression":
        return {
            "kind": "expression",
            "expression": destack._generated.js.tree.node.to_json_local_node_id(
                value.expression
            ),
        }
    elif value.kind == "declaration":
        return {
            "kind": "declaration",
            "keyword": to_json_binding_keyword(value.keyword),
            "declarators": [
                destack._generated.js.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.declarators
            ],
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_for_initialization(value: Json) -> ForInitialization:
    """Return one ForInitialization from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "expression":
        return ForInitializationExpression(
            expression=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "expression")
            )
        )
    elif kind == "declaration":
        return ForInitializationDeclaration(
            keyword=from_json_binding_keyword(json_field(object_, "keyword")),
            declarators=[
                destack._generated.js.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "declarators"))
            ],
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


"""The binding keyword used by a for each binding."""
BindingKeyword: typing.TypeAlias = (
    typing.Literal["var"] | typing.Literal["let"] | typing.Literal["const"]
)


def encode_binding_keyword(writer: BinaryWriter, value: BindingKeyword) -> None:
    """Encode one BindingKeyword."""
    if value == "var":
        writer.write_unsigned(0)
    elif value == "let":
        writer.write_unsigned(1)
    elif value == "const":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_binding_keyword(reader: BinaryReader) -> BindingKeyword:
    """Decode one BindingKeyword."""
    variant = reader.read_number()

    if variant == 0:
        return "var"
    elif variant == 1:
        return "let"
    elif variant == 2:
        return "const"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_binding_keyword(value: BindingKeyword) -> Json:
    """Return one JSON value for one BindingKeyword."""
    return value


def from_json_binding_keyword(value: Json) -> BindingKeyword:
    """Return one BindingKeyword from one JSON value."""
    variant = json_string(value)

    if variant == "var":
        return "var"
    elif variant == "let":
        return "let"
    elif variant == "const":
        return "const"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


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
