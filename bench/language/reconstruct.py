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
from bench.language.type import (
    PRIMITIVE_TYPES,
    Capability,
    Code,
    Compilation,
    Dataset,
    Expectation,
    Requirement,
    Runconfig,
    StatementPath,
    StatementType,
    SymbolContent,
    SymbolType,
    Task,
    Type,
    TypeNode,
    TypeTag,
    Value,
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
        statements.sort(key=lambda s: s.index)

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
        content_lines = content_str.splitlines()
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
        reference_name = escape_identifier(get_reference_name(statement.reference))
        import_source = render_import_source(statement.reference, via=statement)
        return f"import {statement.symbol_type} {reference_name}{alias_str} from {import_source}"
    elif statement.type == StmT.DEFINITION:
        modifier_str = f"{statement.modifier} " if statement.modifier else ""
        identifier_str = escape_identifier(statement.name)

        # special case: inline type node "redefinitions" as definitions
        if (
            statement.symbol_type == SymT.TYPE
            and cast(Type, statement.content).type_node.type != TypeTag.STRUCT
        ):
            type_str = render_type_node(cast(Type, statement.content).type_node)
            return f"{modifier_str}{statement.symbol_type} {identifier_str} = {type_str}"

        if statement.symbol_type == SymT.REQUIREMENT:
            postfix = f"@{cast(Requirement, statement.content).version}"
        elif statement.symbol_type == SymT.TASK:
            type_str = render_type_node(cast(Task, statement.content).type_node)
            postfix = f" :: {type_str}:"
        elif statement.symbol_type == SymT.CODE:
            type_str = render_type_node(cast(Code, statement.content).type_node)
            postfix = f" :: {type_str}:"
        elif statement.symbol_type == SymT.DATASET:
            content = cast(Dataset, statement.content)
            type_str = render_type_node_struct(content.type_node, ", ")
            postfix = f" :: ({type_str}):"
        else:
            postfix = ":"
        def_str = f"{modifier_str}{statement.symbol_type} {identifier_str}{postfix}"
        if include_content:
            content_str = render_symbol_content(statement.content)
        else:
            content_str = ""
        return f"{def_str}\n{content_str}" if content_str else def_str
    elif statement.type == StmT.REDEFINITION:
        modifier_str = f"{statement.modifier} " if statement.modifier else ""
        identifier_str = escape_identifier(statement.name)
        reference_name = escape_identifier(get_reference_name(statement.reference))
        return f"{modifier_str}{statement.symbol_type} {identifier_str} = {statement.symbol_type} {reference_name}"
    elif statement.type == StmT.REFERENCE:
        modifier_str = f"{statement.modifier} " if statement.modifier else ""
        identifier_str = escape_identifier(statement.name)
        return f"{modifier_str}{statement.symbol_type} {identifier_str}"
    else:
        raise ValueError(f"unexpected statement type: {statement}")


def render_symbol_content(content: SymbolContent) -> Optional[str]:
    if isinstance(content, Type):
        rendered_type = render_type_node(content.type_node)
        if content.description is not None:
            return f"{render_description(content.description)}\n{rendered_type}"
        else:
            return rendered_type
    elif isinstance(content, Capability):
        return render_description(content.description)
    elif isinstance(content, Task):
        return render_description(content.description)
    elif isinstance(content, Expectation):
        return render_description(content.description)
    elif isinstance(content, Code):
        rendered_code = render_literal(content.code, lang=content.language)
        if content.description is not None:
            return f"{render_description(content.description)}\n{rendered_code}"
        else:
            return rendered_code
    elif isinstance(content, Dataset):
        if content.language == "jsonl":
            records_as_jsonl = "\n".join(json.dumps(record) for record in content.records)
            rendered_data = render_literal(records_as_jsonl, lang="jsonl")
        elif content.language == "json":
            rendered_data = render_literal(json.dumps(content.records), lang="json")
        elif content.language == "csv":
            field_names = [field.name for field in content.type_node.children]
            csv_output = io.StringIO()
            csv_writer = csv.DictWriter(
                csv_output, quoting=csv.QUOTE_NONNUMERIC, fieldnames=field_names
            )
            csv_writer.writerows(content.records)
            rendered_data = render_literal(csv_output.getvalue().strip(), lang="csv")
        else:
            raise ValueError(f"unexpected data language: {content.language}")
        if content.description is not None:
            return f"{render_description(content.description)}\n{rendered_data}"
        else:
            return rendered_data
    elif isinstance(content, Value):
        rendered_value = render_literal(json.dumps(content.value))
        if content.description is not None:
            return f"{render_description(content.description)}\n{rendered_value}"
        else:
            return rendered_value
    elif isinstance(content, (Compilation, Runconfig, Requirement)):
        return None
    else:
        raise ValueError(f"unexpected symbol content type: {type(content)} {content}")


def escape_identifier(identifier: str) -> str:
    """Wraps an identifier in single quotes if it contains special characters or is a keyword."""
    if IDENTIFIER_REGEX.fullmatch(identifier) and identifier not in KEYWORDS:
        return identifier
    else:
        return f"'{identifier}'"


def render_type_node(node: TypeNode, ignore_name: bool = False) -> str:
    if node.name and not ignore_name:
        identifier_str = escape_identifier(node.name) + ": "
    else:
        identifier_str = ""
    description_str = f' "{node.description}"' if node.description else ""
    if node.type == TypeTag.TYPE_REFERENCE or node.source_reference is not None:
        # if it's a reference _or_ used to be a reference, keep the type reference
        reference_str = escape_identifier(node.source_reference)
        return f"{identifier_str}{reference_str}{description_str}"
    elif node.type == TypeTag.FUNCTION:
        return render_type_node_func(node)
    elif node.type == TypeTag.STRUCT:
        return render_type_node_struct(node, seperator="\n")
    elif node.type == TypeTag.ARRAY:
        type_str = f"[{render_type_node(node.children[0])}]"
        return f"{identifier_str}{type_str}{description_str}"
    elif node.type == TypeTag.UNION:
        type_str = " | ".join(render_type_node(e) for e in node.children)
        return f"{identifier_str}{type_str}{description_str}"
    elif node.type == TypeTag.INTERSECTION:
        type_str = " & ".join(render_type_node(e) for e in node.children)
        return f"{identifier_str}{type_str}{description_str}"
    elif node.type in PRIMITIVE_TYPES:
        return f"{identifier_str}{node.type.value}{description_str}"
    else:
        raise ValueError(f"unexpected type: {node.type}")


def render_type_node_func(node: TypeNode) -> str:
    input_str = render_type_node_struct(node.input, seperator=", ")
    if node.output.type != TypeTag.NULL:
        output_str = render_type_node(node.output, ignore_name=True)
        return f"({input_str}) -> {output_str}"
    else:
        return f"({input_str})"


def render_type_node_struct(node: TypeNode, seperator: str) -> str:
    if node.children is None:
        raise ValueError(f"expected type with elements: {node}")
    field_strs = [render_type_node(field) for field in node.children]
    return seperator.join(field_strs)


def render_description(value: str) -> str:
    return f'"{value}"'


def render_literal(value: str, lang: Optional[str] = None) -> str:
    prefer_multiline = lang == "python" or lang == "jsonl"
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
