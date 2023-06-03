"""Renders language types to their source text, preserving available source information."""

from __future__ import annotations

import csv
import io
import json
from collections import defaultdict
from typing import Iterable, Optional, cast
from uuid import UUID

from bench.language import File, Statement
from bench.language.lex import IDENTIFIER_REGEX, INLINE_LITERAL_REGEX, KEYWORDS, LINE_COMMENT_REGEX
from bench.language.parse import get_reference_as_path
from bench.language.type import (
    PRIMITIVE_TYPES,
    BuildContent,
    CapabilityContent,
    CodeContent,
    DataContent,
    ExpectationContent,
    Field,
    RequirementContent,
    StatementPath,
    StatementType,
    SymbolContent,
    SymbolType,
    TaskContent,
    TypeContent,
    TypeFlag,
    TypeNode,
    TypeTag,
)

StmT = StatementType
SymT = SymbolType


def render(files: list[File]) -> str:
    lines = []
    for i, file in enumerate(files):
        if i != 0:
            lines.append("")
        lines.append(f"--- {file.path} ---")
        lines.append(render_file(file))
    return "\n".join(lines)


def render_file(file: File) -> str:
    lines = []

    # group by parent
    statements_by_parent: dict[UUID | None, list[Statement]] = defaultdict(list)
    for statement in file.statements:
        parent_id = statement.parent.id if statement.parent else None
        statements_by_parent[parent_id].append(statement)
    # sort by index
    for statements in statements_by_parent.values():
        statements.sort(key=lambda s: s.order_key)

    def walk_dfs(statement: Statement, indent: int) -> Iterable[Statement]:
        yield statement, indent
        for child in statements_by_parent.get(statement.id, []):
            yield from walk_dfs(child, indent + 1)

    root_statements = statements_by_parent.get(None, [])
    for i, root_statement in enumerate(root_statements):
        for statement, indent in walk_dfs(root_statement, 0):
            lines.append(render_statement_indented(statement, indent))

    return "\n".join(lines)


def render_statement_indented(statement: Statement, indent: int) -> str:
    content_str = render_statement(statement)
    if indent > 0:
        # if empty, add a single empty line
        content_lines = content_str.splitlines() if content_str else [""]
        indent_str = " " * 4 * indent  # use 4 spaces
        content_lines = [f"{indent_str}{line}" for line in content_lines]
        content_str = "\n".join(content_lines)
    return content_str


def render_statement(statement: Statement, include_content: bool = True) -> str:
    """Renders the statement itself without indentation"""
    if statement.type == StmT.BLANK:
        return ""
    elif statement.type == StmT.COMMENT:
        return render_comment(statement.text)
    elif statement.type == StmT.IMPORT:
        alias_name = escape_identifier(statement.name)
        alias_str = f" as {alias_name}" if statement.is_alias else ""
        reference_str = escape_identifier(get_reference_name(statement.reference))
        import_source = render_import_source(statement.reference, via=statement)
        return f"import {statement.symbol_type} {reference_str}{alias_str} from {import_source}"
    elif statement.type == StmT.DEFINITION:
        modifier_str = f"{statement.modifier} " if statement.modifier else ""
        identifier_str = escape_identifier(statement.name)

        # special case: inline type node "redefinitions" as definitions
        if statement.symbol_type == SymT.TYPE and cast(TypeNode, statement.content).tag not in (
            TypeTag.STRUCT,
            TypeTag.ENUM,
        ):
            type_str = render_type_node(cast(TypeNode, statement.content), statement)
            return f"{modifier_str}{statement.symbol_type} {identifier_str} = {type_str}"

        if isinstance(statement.content, TypeContent) and statement.content.tag == TypeTag.ENUM:
            symt_str = "enum"
        else:
            symt_str = statement.symbol_type
        if statement.symbol_type == SymT.REQUIREMENT:
            postfix = f"@{cast(RequirementContent, statement.content).version}"
        elif statement.symbol_type == SymT.TASK:
            type_str = render_type_func(cast(TaskContent, statement.content), statement)
            postfix = f" :: {type_str}:"
        elif statement.symbol_type == SymT.CODE:
            type_str = render_type_func(cast(CodeContent, statement.content), statement)
            postfix = f" :: {type_str}:"
        elif statement.symbol_type == SymT.DATA:
            content = cast(DataContent, statement.content)
            nodes = content.self_fields or content.fields or []
            type_str = render_type_struct(nodes, statement)
            postfix = f" :: {type_str}:"
        else:
            postfix = ":"
        def_str = f"{modifier_str}{symt_str} {identifier_str}{postfix}"
        if include_content:
            content_str = render_symbol_content(statement.content, statement)
        else:
            content_str = ""
        return f"{def_str}\n{content_str}" if content_str else def_str
    elif statement.type == StmT.REDEFINITION:
        modifier_str = f"{statement.modifier} " if statement.modifier else ""
        identifier_str = escape_identifier(statement.name)
        reference_str = render_reference(statement.reference, statement)
        return f"{modifier_str}{statement.symbol_type} {identifier_str} = {statement.symbol_type} {reference_str}"
    elif statement.type == StmT.REFERENCE:
        modifier_str = f"{statement.modifier} " if statement.modifier else ""
        reference_str = render_reference(statement.reference, statement)
        return f"{modifier_str}{statement.symbol_type} {reference_str}"
    else:
        raise ValueError(f"unexpected statement type: {statement}")


def render_symbol_content(content: SymbolContent, statement: Statement) -> Optional[str]:
    if isinstance(content, CapabilityContent):
        return render_description(content.description)
    elif isinstance(content, TaskContent):
        return render_description(content.description)
    elif isinstance(content, ExpectationContent):
        return render_description(content.description)
    elif isinstance(content, CodeContent):
        rendered_code = render_literal(content.code, lang=content.language)
        if content.description is not None:
            return f"{render_description(content.description)}\n{rendered_code}"
        else:
            return rendered_code
    elif isinstance(content, DataContent):
        records_data = [record.data for record in content.records]
        if content.language == "jsonl":
            records_as_jsonl = "\n".join(json.dumps(data) for data in records_data)
            rendered_data = render_literal(records_as_jsonl, lang="jsonl")
        elif content.language == "json":
            rendered_data = render_literal(json.dumps(records_data), lang="json")
        elif content.language == "csv":
            field_names = [field.name for field in content.fields]
            csv_output = io.StringIO()
            csv_writer = csv.DictWriter(
                csv_output,
                quoting=csv.QUOTE_NONNUMERIC,
                fieldnames=field_names,
                lineterminator="\n",
            )
            csv_writer.writerows(records_data)
            rendered_data = render_literal(csv_output.getvalue().strip(), lang="csv")
        else:
            raise ValueError(f"unexpected data language: {content.language}")
        if content.description is not None:
            return f"{render_description(content.description)}\n{rendered_data}"
        else:
            return rendered_data
    elif isinstance(content, TypeContent):
        if content.tag == TypeTag.STRUCT:
            rendered_type = render_type_struct_inner(
                content.self_fields or content.fields, statement, "\n", inline=False
            )
        elif content.tag == TypeTag.ENUM:
            rendered_type = render_type_enum(content.self_fields or content.fields)
        elif content.tag == TypeTag.FUNCTION:
            rendered_type = render_type_func(content, statement)
        else:
            rendered_type = render_type_node(content, statement)
        if content.description is not None:
            return f"{render_description(content.description)}\n{rendered_type}"
        else:
            return rendered_type
    elif isinstance(content, (BuildContent, RequirementContent)):
        return None
    else:
        raise ValueError(f"unexpected symbol content type: {type(content)} {content}")


def escape_identifier(identifier: str) -> str:
    """Wraps an identifier in single quotes if it contains special characters or is a keyword."""
    identifier = identifier.replace("\n", " ")  # strip newlines
    if IDENTIFIER_REGEX.fullmatch(identifier) and identifier not in KEYWORDS:
        return identifier
    else:
        return f"'{identifier}'"


def render_type_node(
    node: TypeNode,
    statement: Statement | None,
    ignore_name: bool = False,
    ignore_reference: bool = False,
    ignore_description: bool = False,
) -> str:
    if node.name and not ignore_name:
        identifier_str = escape_identifier(node.name) + ": "
    else:
        identifier_str = ""
    description_str = (
        f' "{node.description}"' if node.description and not ignore_description else ""
    )
    if node.tag == TypeTag.TYPE_REFERENCE or (node.reference is not None and not ignore_reference):
        type_str = render_reference(node.reference, statement)
    elif node.tag == TypeTag.UNION:
        nodes = node.self_fields or node.fields or []
        type_str = " | ".join(render_type_node(e, statement) for e in nodes)
    elif node.hint is not None:
        type_str = node.hint
    elif node.tag in PRIMITIVE_TYPES or node.tag == TypeTag.ANY:
        type_str = node.tag.value
    elif node.tag == TypeTag.LITERAL:
        type_str = render_literal(json.dumps(node.value))
    else:
        raise ValueError(f"unexpected type: {node.tag}")

    if node.flags & TypeFlag.IsArray:
        type_str = f"[{type_str}]"
    if node.flags & TypeFlag.IsNullable:
        type_str = f"{type_str}?"

    return f"{identifier_str}{type_str}{description_str}"


def render_type_func(node: TypeContent, statement: Statement) -> str:
    nodes = node.self_fields or node.fields or []
    inputs = [n for n in nodes if not (n.flags & TypeFlag.IsOutput)]
    outputs = [n for n in nodes if n.flags & TypeFlag.IsOutput]
    input_str = render_type_struct(inputs, statement)
    if node.outputs:
        output_str = render_type_struct(outputs, statement)
        return f"{input_str} -> {output_str}"
    else:
        return input_str


def render_type_struct(nodes: list[TypeNode], statement: Statement | None) -> str:
    if not nodes:
        return "()"
    unioned_nodes = [node for node in nodes if node.flags & TypeFlag.IsUnionWith]
    normal_nodes = [node for node in nodes if not node.flags & TypeFlag.IsUnionWith]

    parts = []
    if normal_nodes:
        normal_str = f"({render_type_struct_inner(normal_nodes, statement, ', ')})"
        parts.append(normal_str)
    for node in unioned_nodes:
        parts.append(render_reference(node.reference, statement))
    return " & ".join(parts)


def render_type_struct_inner(
    nodes: list[TypeNode], statement: Statement | None, seperator: str, inline: bool = True
) -> str:
    if not nodes:
        return ""
    field_strs = []
    for node in nodes:
        field_str = render_type_node(node, statement)
        if not inline:
            if node.flags & TypeFlag.IsUnionWith:
                field_str = f"& {field_str}"
            else:
                field_str = f"- {field_str}"
        field_strs.append(field_str)
    return seperator.join(field_strs)


def render_type_enum(nodes: list[Field]) -> str:
    members_strs = []
    for m in nodes:
        member_str = f"- {escape_identifier(m.name)}"
        if m.description:
            member_str += f' "{m.description}"'
        members_strs.append(member_str)
    return "\n".join(members_strs)


def render_description(value: str) -> str:
    return f'"{value or ""}"'


def render_literal(value: str, lang: Optional[str] = None) -> str:
    prefer_multiline = lang in ("python", "jsonl", "x")
    if INLINE_LITERAL_REGEX.fullmatch(f"`{value}`") and not prefer_multiline:
        if lang is None:
            return f"`{value}`"
        else:
            return f"`{value}`{{.{lang}}}"
    else:  # multiline
        if lang is None:
            return f"```\n{value}\n```"
        else:
            return f"```{lang}\n{value}\n```"


def render_comment(text: str) -> str:
    if LINE_COMMENT_REGEX.fullmatch(f"# {text}"):
        return f"# {text}"
    else:
        return f"###\n{text}\n###"


def get_reference_name(reference: Statement | StatementPath) -> str:
    if isinstance(reference, StatementPath):
        return reference[1]
    elif isinstance(reference, Statement):
        return reference.name
    else:
        raise ValueError(f"unexpected reference type: {reference}")


def render_reference(reference: Statement | StatementPath | UUID, via: Statement | None) -> str:
    if isinstance(reference, UUID):
        return escape_identifier(str(reference))
    elif not isinstance(reference, StatementPath):
        reference = get_reference_as_path(reference, via)
    if reference[0] == ".":
        return escape_identifier(reference[1])
    return escape_identifier(f"{reference[0]}.{reference[1]}")


def render_import_source(reference: Statement | StatementPath, via: Statement) -> str:
    # :StatementReferencePath
    if isinstance(reference, StatementPath):
        return reference[0]
    elif isinstance(reference, Statement):
        # if same module, use local reference
        if reference.file.module.name == via.file.module.name:
            return f".{reference.file.path_without_extension}"
        else:  # otherwise use absolute reference
            return f"{reference.file.module.name}.{reference.file.path_without_extension}"
    else:
        raise ValueError(f"unexpected reference type: {reference}")
