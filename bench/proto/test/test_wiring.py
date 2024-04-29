import pytest

from bench.language import Node, Struct
from bench.language.const import OBJECT_TYPES
from bench.language.setup import BENCH_CLASS_BY_TYPE
from bench.language.test.fabricator import Fabricator
from bench.proto import wiring

fabricator = Fabricator(42)
BENCH_OBJECTS = tuple(fabricator.fabricate(BENCH_CLASS_BY_TYPE[t], ()) for t in OBJECT_TYPES)


@pytest.mark.parametrize("bench_obj", BENCH_OBJECTS, ids=lambda o: o.__class__.__name__)
def test_roundtrip_bytes(bench_obj: Node | Struct):
    packed_wire_obj = wiring.pack_struct(bench_obj)
    packed_bytes = bytes(packed_wire_obj)
    unpacked_wire_obj = type(packed_wire_obj)().parse(packed_bytes)
    unpacked_obj = wiring.unpack_struct(unpacked_wire_obj)
    assert unpacked_obj.equals_content(bench_obj), f"{unpacked_obj!r} != {bench_obj!r}"


@pytest.mark.parametrize("bench_obj", BENCH_OBJECTS, ids=lambda o: o.__class__.__name__)
def test_roundtrip_json(bench_obj: Node | Struct):
    packed_wire_obj = wiring.pack_struct(bench_obj)
    packed_json = packed_wire_obj.to_json(indent=2)
    unpacked_wire_obj = type(packed_wire_obj)().from_json(packed_json)
    unpacked_obj = wiring.unpack_struct(unpacked_wire_obj)
    assert unpacked_obj.equals_content(bench_obj), f"{unpacked_obj!r} != {bench_obj!r}"


@pytest.mark.parametrize("bench_obj", BENCH_OBJECTS, ids=lambda o: o.__class__.__name__)
def test_roundtrip_robust_json(bench_obj: Node | Struct):
    packed_wire_obj = wiring.pack_struct(bench_obj)
    packed_json = packed_wire_obj.to_robust_json(indent=2)
    unpacked_wire_obj = type(packed_wire_obj)().from_robust_json(packed_json)
    unpacked_obj = wiring.unpack_struct(unpacked_wire_obj)
    assert unpacked_obj.equals_content(bench_obj), f"{unpacked_obj!r} != {bench_obj!r}"
