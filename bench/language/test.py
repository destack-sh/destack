# all .bench files in bench/demo
import glob
from pathlib import Path

import pytest

from bench.language import parse
from bench.language.lex import SourceFile, lex
from bench.language.reconstruct import render

demo_paths = glob.glob("../demo/*.bench")
if len(demo_paths) == 0:
    raise RuntimeError(f"no demo files found at bench/demo (cwd={Path.cwd()})")


@pytest.mark.parametrize("path", demo_paths)
def test_round_trip_demo(path: str):
    source_file = SourceFile(path=path, content=Path(path).read_text())
    tokens = lex(source_file)
    module = parse(tokens, on_error="raise")
    reconstructed = render(module.files)
    assert reconstructed == source_file.content
