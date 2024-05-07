import pytest

from bench.language import Node, Struct
from bench.language.const import OBJECT_TYPES
from bench.language.setup import BENCH_CLASS_BY_TYPE
from bench.language.test.fabricator import Fabricator

fabricator = Fabricator(42)
BENCH_OBJECTS = tuple(fabricator.fabricate(BENCH_CLASS_BY_TYPE[t], ()) for t in OBJECT_TYPES)


@pytest.mark.parametrize("bench_obj", BENCH_OBJECTS, ids=lambda o: o.__class__.__name__)
def test_render(bench_obj: Node | Struct):
    pass
