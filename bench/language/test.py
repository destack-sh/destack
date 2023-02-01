from __future__ import annotations

import functools
import glob
from pathlib import Path
from typing import Callable

import pytest

from bench.language import TypeTag, parse
from bench.language.lex import SourceFile, lex
from bench.language.parse import ErrorType, ParseError, SemanticError, parse_string
from bench.language.reconstruct import render
from bench.language.type import Type

# all .x files in bench/bench
demo_paths = glob.glob("../bench/*.bench")
if len(demo_paths) == 0:
    raise RuntimeError(f"no demo files found (cwd={Path.cwd()})")


def _raise_if_error(error: ParseError | SemanticError, test: Callable):
    if test(error):
        raise error


def _raise_if(test: Callable):
    return functools.partial(_raise_if_error, test=test)


def _raise_if_not_external():
    return _raise_if(lambda e: e.type != ErrorType.EXTERNAL_LOOKUP_FAILED)


@pytest.mark.parametrize("path", demo_paths)
def test_round_trip_demo_files(path: str):
    source_file = SourceFile(path=path, content=Path(path).read_text())
    module, _ = parse(lex(source_file), on_error=_raise_if_not_external())
    reconstructed = render(module.files)
    assert reconstructed == source_file.content


def test_unexpected_indent():
    with pytest.raises(ParseError) as excinfo:
        # empty un-indented line stops the indent
        parse_string(
            """
--- test.x ---
task something :: ():
"Do something"



    task something_else :: ():
    "Do something else"
"""
        )
    assert excinfo.value.type == ErrorType.UNEXPECTED_INDENT


def test_resolve_nested_aliased_type():
    module, idx = parse_string(
        """
--- test.x ---
type RealString = string
type MyString = RealString
type EntityType = MyString

type Entity:
name: string
'type': EntityType
"""
    )

    type_entity_type = idx.symbol(".test:EntityType", Type)
    assert type_entity_type.tag == TypeTag.STRING

    type_entity = idx.symbol(".test:Entity", Type)
    assert type_entity.child("type").tag == TypeTag.STRING


def test_resolve_circular_type():
    module, idx = parse_string(
        """
--- test.x ---
type Entity:
name: string
first_event: Event | null

type Event:
summary: string
entities: [Entity]
"""
    )

    type_event = idx.symbol(".test:Event", Type)
    assert type_event.child("entities").tag == TypeTag.ARRAY

    type_entity = idx.symbol(".test:Entity", Type)
    assert type_entity.child("first_event").children[0].tag == type_event.tag
