from __future__ import annotations

import functools
import glob
from pathlib import Path
from typing import Callable

import pytest

from bench.language import Type, TypeTag, parse
from bench.language.lex import SourceFile, lex
from bench.language.parse import (
    ParseError,
    SemanticError,
    SemanticErrorType,
    index_module,
    parse_string,
)
from bench.language.reconstruct import render

# all .instruct files in bench/demo
demo_paths = glob.glob("../demo/*.bench")
if len(demo_paths) == 0:
    raise RuntimeError(f"no demo files found at bench/demo (cwd={Path.cwd()})")


def _raise_if_error(error: ParseError | SemanticError, test: Callable):
    if test(error):
        raise error


def _raise_if(test: Callable):
    return functools.partial(_raise_if_error, test=test)


def _raise_if_not_external():
    return _raise_if(lambda e: e.type != SemanticErrorType.EXTERNAL_LOOKUP_FAILED)


@pytest.mark.parametrize("path", demo_paths)
def test_round_trip_demo_files(path: str):
    source_file = SourceFile(path=path, content=Path(path).read_text())
    module = parse(lex(source_file), on_error=_raise_if_not_external())
    reconstructed = render(module.files)
    assert reconstructed == source_file.content


def test_resolve_nested_indirect_type():
    module = parse_string(
        """
--- test.instruct ---
type RealString = string
type MyString = RealString
type EntityType = MyString

type Entity:
name: string
'type': EntityType
"""
    )
    idx = index_module(module)

    type_entity_type = idx.symbol(".test:EntityType", Type).node
    assert type_entity_type.type == TypeTag.STRING

    type_entity = idx.symbol(".test:Entity", Type).node
    assert type_entity.child("type").type == TypeTag.STRING


def test_resolve_circular_type():
    module = parse_string(
        """
--- test.instruct ---
type Entity:
name: string
first_event: Event | null

type Event:
summary: string
entities: [Entity]
"""
    )
    idx = index_module(module)

    type_event = idx.symbol(".test:Event", Type).node
    assert type_event.child("entities").type == TypeTag.ARRAY

    type_entity = idx.symbol(".test:Entity", Type).node
    assert type_entity.child("first_event").children[0].name == type_event.name
