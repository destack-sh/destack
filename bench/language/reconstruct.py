"""Renders language types to their source text, preserving available source information."""

from __future__ import annotations

import json
from collections import defaultdict
from typing import Iterable, Optional
from uuid import UUID

from bench.api.symbol import StatementType
from bench.language import File, Statement
from bench.language.lex import IDENTIFIER_REGEX, INLINE_LITERAL_REGEX
from bench.language.parse import UNGROUPED_STATEMENT_TYPES
from bench.language.schema import render_bsl
from bench.language.type import (
    Code,
    Compilation,
    Dataset,
    Expectation,
    Requirement,
    Runconfig,
    Schema,
    StatementPath,
    SymbolContent,
    Task,
    Value,
)


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
            lines.append(render_statement(statement, indent, render_indent=True))
        # add extra blank line between ungrouped root statements
        if root_statement.type in UNGROUPED_STATEMENT_TYPES and i != (len(root_statements) - 1):
            lines.append("")

    return "\n".join(lines)


def render_statement(statement: Statement, indent: int, render_indent: bool) -> str:
    content_str = render_statement_content(statement)

    if render_indent:
        content_lines = content_str.splitlines()
        indent_str = " " * 4 * indent  # use 4 spaces
        content_lines = [f"{indent_str}{line}" for line in content_lines]
        content_str = "\n".join(content_lines)

    return content_str


def render_statement_content(statement: Statement) -> str:
    if statement.type == StatementType.BLANK:
        return ""
    elif statement.type == StatementType.COMMENT:
        return f"# {statement.text}"
    elif statement.type == StatementType.IMPORT:
        alias_name = escape_identifier(statement.name)
        alias_str = f" as {alias_name}" if statement.is_alias else ""
        reference_name = escape_identifier(get_reference_name(statement.reference))
        import_source = render_import_source(statement.reference, via=statement)
        return f"import {statement.symbol_type} {reference_name}{alias_str} from {import_source}"
    elif statement.type == StatementType.DEFINITION:
        content_str = render_symbol_content(statement.content)
        modifier_str = f"{statement.modifier} " if statement.modifier else ""
        identifier_str = escape_identifier(statement.name)
        def_str = f"{modifier_str}{statement.symbol_type} {identifier_str}:"
        return f"{def_str}\n{content_str}" if content_str else def_str
    elif statement.type == StatementType.REDEFINITION:
        modifier_str = f"{statement.modifier} " if statement.modifier else ""
        identifier_str = escape_identifier(statement.name)
        reference_name = escape_identifier(get_reference_name(statement.reference))
        return f"{modifier_str}{statement.symbol_type} {identifier_str} = {statement.symbol_type} {reference_name}"
    elif statement.type == StatementType.REFERENCE:
        modifier_str = f"{statement.modifier} " if statement.modifier else ""
        identifier_str = escape_identifier(statement.name)
        return f"{modifier_str}{statement.symbol_type} {identifier_str}"
    else:
        raise ValueError(f"unexpected statement type: {statement}")


def render_symbol_content(content: SymbolContent) -> Optional[str]:
    if isinstance(content, Schema):
        bsl = render_bsl(content.element)
        return render_literal(bsl)
    elif isinstance(content, Task):
        return render_literal(content.description)
    elif isinstance(content, Expectation):
        return render_literal(content.description)
    elif isinstance(content, Code):
        return render_literal(content.code, lang=content.language)
    elif isinstance(content, Dataset):
        records_as_jsonl = "\n".join(json.dumps(record) for record in content.records)
        return render_literal(records_as_jsonl, lang="jsonl")
    elif isinstance(content, Value):
        value_as_json = json.dumps(content.value)
        return render_literal(value_as_json)
    elif isinstance(content, (Compilation, Runconfig, Requirement)):
        return None
    else:
        raise ValueError(f"unexpected symbol content type: {content}")


def escape_identifier(identifier: str) -> str:
    """Wraps an identifier in single quotes if it contains special characters."""
    if IDENTIFIER_REGEX.fullmatch(identifier):
        return identifier
    else:
        return f"'{identifier}'"


def render_literal(value: str, lang: Optional[str] = None) -> str:
    if INLINE_LITERAL_REGEX.fullmatch(f"`{value}`"):
        if lang is None:
            return f"`{value}`"
        else:
            return f"`{value}`{{.{lang}}}"
    else:  # multiline
        if lang is None:
            return f"```\n{value}\n```"
        else:
            return f"```{lang}\n{value}\n```"


def get_reference_name(reference: Statement | StatementPath) -> str:
    if isinstance(reference, StatementPath):
        return reference[1]
    elif isinstance(reference, Statement):
        return reference.name
    else:
        raise ValueError(f"unexpected reference type: {reference}")


def render_import_source(reference: Statement | StatementPath, via: Statement) -> str:
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
