from __future__ import annotations

import functools
import glob
from pathlib import Path
from typing import Callable

import pytest

from bench.language import parse
from bench.language.lex import SourceFile, lex
from bench.language.parse import ParseError, SemanticError, SemanticErrorType
from bench.language.reconstruct import render

# all .bench files in bench/demo
demo_paths = glob.glob("../demo/*.bench")
if len(demo_paths) == 0:
    raise RuntimeError(f"no demo files found at bench/demo (cwd={Path.cwd()})")


def _raise_if_error(error: ParseError | SemanticError, test: Callable):
    if test(error):
        raise error


def _raise_if(test: Callable):
    return functools.partial(_raise_if_error, test=test)


@pytest.mark.parametrize("path", demo_paths)
def test_round_trip_demo_files(path: str):
    source_file = SourceFile(path=path, content=Path(path).read_text())
    module = parse(
        lex(source_file),
        on_error=_raise_if(lambda e: e.type != SemanticErrorType.EXTERNAL_LOOKUP_FAILED),
    )
    reconstructed = render(module.files)
    assert reconstructed == source_file.content
