from __future__ import annotations

from itertools import chain

import structlog

from bench.language import (
    Code,
    CodeContent,
    Dataset,
    DatasetContent,
    File,
    Module,
    Statement,
    StatementType,
)
from bench.language.type import Build, BuildContent, InterpSymbol
from bench.utils.fractional import generate_n_keys_between

logger = structlog.get_logger(__name__)


def map_to_file(
    symbols: list[InterpSymbol], weak_references: list[InterpSymbol], file: File | None = None
) -> File:
    """Map high-level interpreted symbols back to lower level statements."""

    if file is None:
        file = File(module=Module(name="<generated>"), path="<generated>")
        start_ok = None
    else:
        start_ok = max((s.order_key for s in file.statements if s.parent is None), default=None)

    order_keys = generate_n_keys_between(start_ok, None, len(symbols) + len(weak_references))

    # render weak references :WeakReferences
    for order_key, symbol in zip(order_keys, weak_references):
        if symbol.source is None:
            raise RuntimeError(f"weak reference {symbol} has no source")
        statement = Statement(
            type=StatementType.IMPORT,
            symbol_type=symbol.symbol_type,
            modifier=symbol.modifier,
            name=symbol.name,
            # point directly to underling definition, won't work with :Variables
            reference=symbol.source.underlying_definition,
            content=None,
            file=file,
            parent=None,
            order_key=order_key,
            generated=True,
        )
        file.statements.append(statement)

    # render symbols themselves
    for order_key, symbol in zip(order_keys, symbols):
        statement = _map_statement(file, symbol, order_key)
        children = _map_statement_children(symbol, statement)
        file.statements.append(statement)
        file.statements.extend(children)

    return file


def _map_statement(file: File, symbol: InterpSymbol, order_key: str) -> Statement:
    """Map a single symbol to a statement."""
    if isinstance(symbol, Dataset):
        content = map_dataset_content(symbol)
    elif isinstance(symbol, Code):
        content = map_code_content(symbol)
    elif isinstance(symbol, Build):
        content = map_build_content(symbol)
    else:
        raise RuntimeError(f"unexpected symbol {symbol}")
    statement = Statement(
        id=symbol.id,
        type=StatementType.DEFINITION,
        symbol_type=symbol.symbol_type,
        modifier=symbol.modifier,
        name=symbol.name,
        content=content,
        file=file,
        parent=None,
        order_key=order_key,
        generated=True,
    )
    return statement


def _map_statement_children(symbol: InterpSymbol, statement: Statement) -> list[Statement]:
    """Maps the nested symbols of a symbol to statements."""
    if isinstance(symbol, Build):
        order_keys = generate_n_keys_between(None, None, len(symbol.tasks) + len(symbol.models))
        children = []
        for ok, child_symbol in zip(order_keys, chain(symbol.tasks, symbol.models)):
            child = _make_ref_or_def(child_symbol, ok, statement)
            children.append(child)
        return children
    elif isinstance(symbol, (Code, Dataset)):
        return []
    else:
        raise RuntimeError(f"unexpected symbol {symbol}")


def _make_ref_or_def(symbol: InterpSymbol, order_key: str, parent: Statement):
    if symbol.source is not None:
        return _make_reference(symbol, order_key, parent)
    else:
        return _make_definition(symbol, order_key, parent)


def _make_reference(symbol: InterpSymbol, order_key: str, parent: Statement):
    if symbol.definition.source is None:
        raise RuntimeError(f"symbol has no source {parent}->{symbol}")
    child = Statement(
        id=symbol.id,
        type=StatementType.REFERENCE,
        symbol_type=symbol.symbol_type,
        modifier=symbol.modifier,
        name=symbol.name,
        content=None,
        file=parent.file,
        parent=parent,
        reference=symbol.definition.source,
        order_key=order_key,
        generated=True,
    )
    return child


def _make_definition(symbol: InterpSymbol, order_key: str, parent: Statement):
    if symbol.source is not None:
        raise RuntimeError(f"symbol has a source {parent}->{symbol}")
    child = Statement(
        id=symbol.id,
        type=StatementType.DEFINITION,
        symbol_type=symbol.symbol_type,
        modifier=symbol.modifier,
        name=symbol.name,
        content=symbol,
        file=parent.file,
        parent=parent,
        reference=None,
        order_key=order_key,
        generated=True,
    )
    return child


def map_dataset_content(dataset: Dataset) -> DatasetContent:
    return DatasetContent(
        description=dataset.description,
        language=dataset.language,
        type_node=dataset.type_node,
        records=dataset.records,
    )


def map_code_content(code: Code) -> CodeContent:
    return CodeContent(
        description=code.description,
        language=code.language,
        type_node=code.type_node,
        code=code.code,
        xblocks=code.xblocks,
    )


def map_build_content(build: Build) -> BuildContent:
    return BuildContent(
        source_mappings=build.source_mappings,
        settings=build.settings,
        evaluate_settings=build.evaluate_settings,  # :BuildEvaluationSettings
    )
