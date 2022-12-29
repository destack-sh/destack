"""Renders language types to their source text, preserving available source information."""

from __future__ import annotations

import json
from collections import defaultdict
from itertools import chain
from typing import Iterable
from uuid import UUID

from bench.api.symbol import StatementType
from bench.language import File, Statement
from bench.language.schema import render_bql
from bench.language.types import (
    Code,
    Dataset,
    Expectation,
    Model,
    Schema,
    SymbolContent,
    Task,
    UnresolvedStatement,
    Value,
)


def render(files: list[File]) -> str:
    lines = []
    for file in files:
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
    for statement, indent in chain(*(walk_dfs(root, 0) for root in root_statements)):
        lines.append(render_statement(statement, indent, render_indent=True))

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
    if statement.type == StatementType.COMMENT:
        return f"# {statement.text}"
    elif statement.type == StatementType.REQUIREMENT:
        return f"require {statement.requirement.name}@{statement.requirement.version}"
    elif statement.type == StatementType.COMPILATION:
        return f"compile {statement.name} = {statement.symbol_type} {statement.reference}"
    elif statement.type == StatementType.RUNCONFIG:
        raise NotImplementedError
    elif statement.type == StatementType.IMPORT:
        alias_str = f" as {statement.name}" if statement.is_alias else ""
        reference_name = _get_reference_name(statement.reference)
        import_path = _get_reference_path(statement.reference, via=statement)
        return f"import {reference_name}{alias_str} from {import_path}"
    elif statement.type == StatementType.DEFINITION:
        content_str = render_symbol_content(statement.content)
        return f"{statement.symbol_type} {statement.name}:\n{content_str}"
    elif statement.type == StatementType.REFERENCE:
        return "reference"  # nocheckin
    else:
        raise ValueError(f"unexpected statement type: {statement}")


def render_symbol_content(content: SymbolContent) -> str:
    if isinstance(content, Schema):
        return f"`{render_bql(content.element)}`{{bql}}"
    elif isinstance(content, Task):
        return f"`{content.description}`"
    elif isinstance(content, Expectation):
        return f"`{content.description}`"
    elif isinstance(content, Code):
        return f"```python\n{content.code}\n```"
    elif isinstance(content, Dataset):
        records_as_jsonl = "\n".join(json.dumps(record) for record in content.records)
        return f"```jsonl\n{records_as_jsonl}\n```"
    elif isinstance(content, Value):
        value_as_json = json.dumps(content.value)
        return f"`{value_as_json}`"
    else:
        raise ValueError(f"unexpected symbol content type: {content}")


def _get_reference_name(reference: Statement | UnresolvedStatement) -> str:
    if isinstance(reference, UnresolvedStatement):
        return reference[1]
    elif isinstance(reference, Statement):
        return reference.name
    else:
        raise ValueError(f"unexpected reference type: {reference}")


def _get_reference_path(reference: Statement | UnresolvedStatement, via: Statement) -> str:
    if isinstance(reference, UnresolvedStatement):
        return reference[0]
    elif isinstance(reference, Statement):
        return reference.file.path
    else:
        raise ValueError(f"unexpected reference type: {reference}")
