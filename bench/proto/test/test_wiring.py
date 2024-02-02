import enum
import random
import string
import uuid
from datetime import datetime
from typing import cast
from uuid import UUID

import pytest
import pytz

from bench.language import Node, NodeReference, Struct
from bench.language.const import BENCH_TYPES, NODE_TYPES, BenchType, StructType
from bench.language.node import BENCH_CLASS_BY_TYPE, NODE_CLASS_BY_TYPE, Property
from bench.proto import wiring
from bench.proto.wire import NodeReferenceData
from bench.sql.core import PrimitiveType

random = random.Random(42)

DEFAULT_GENERATORS: dict[type, callable] = {
    bool: lambda: random.choice([True, False]),
    str: lambda: "".join(random.choices(string.ascii_letters, k=10)),
    int: lambda: random.randint(0, 1000),
    float: lambda: random.random(),
    bytes: lambda: random.randbytes(24),
    UUID: lambda: uuid.uuid4(),
    datetime: lambda: datetime.utcnow().replace(tzinfo=pytz.utc),
}


def fabricate_prop_scalar(prop: Property, path: tuple[BenchType, ...]) -> any:
    if prop.is_enum:
        enum_cls = cast(type[enum.Enum], prop.py_type_stripped)
        return random.choice(tuple(enum_cls)) if len(enum_cls) > 0 else None
    elif prop.is_struct:
        return fabricate(prop.struct_type, path)
    elif prop.py_type_stripped == NodeReferenceData:
        return fabricate(StructType.NODE_REFERENCE, path)
    elif prop.primitive_type == PrimitiveType.JSON:
        return {
            DEFAULT_GENERATORS[str](): DEFAULT_GENERATORS[str](),
            DEFAULT_GENERATORS[str](): DEFAULT_GENERATORS[int](),
            DEFAULT_GENERATORS[str](): None,
        }
    elif prop.py_type_stripped in DEFAULT_GENERATORS:
        return DEFAULT_GENERATORS[prop.py_type_stripped]()
    else:
        raise ValueError(f"cannot fabricate {prop}")


def fabricate(bench_type: BenchType, path: tuple[BenchType, ...]) -> Node | Struct:
    path = path + (bench_type,)

    # special cases for semantic correctness
    if bench_type == StructType.NODE_REFERENCE:
        type = random.choice(NODE_TYPES)
        id = uuid.uuid4()
        if "ck" in NODE_CLASS_BY_TYPE[type].__properties__:
            ck = uuid.uuid4()
        else:
            ck = None
        return NodeReference(type=type, id=id, ck=ck)
    elif bench_type == StructType.PROPERTY_REFERENCE:
        type = random.choice(NODE_TYPES)
        prop = random.choice(
            tuple(p for p in NODE_CLASS_BY_TYPE[type].__properties__.values() if p.id)
        )
        return prop.to_ref
    else:  # default random and unconstrained jumble of properties
        kwargs = {}
        bench_cls = BENCH_CLASS_BY_TYPE[bench_type]
        for prop in bench_cls.__runtime_properties__.values():
            if prop.is_runtime_only or prop.is_computed:
                continue
            elif prop.struct_type in path:  # prevent circles
                kwargs[prop.name] = [] if prop.is_array else None
            elif prop.is_array:
                len = random.randint(1, 4)
                kwargs[prop.name] = [fabricate_prop_scalar(prop, path) for _ in range(len)]
            else:
                kwargs[prop.name] = fabricate_prop_scalar(prop, path)
        return bench_cls(**kwargs)


BENCH_OBJECTS = tuple(fabricate(t, ()) for t in BENCH_TYPES)


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
