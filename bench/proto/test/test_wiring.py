from datetime import datetime
import enum
import random
import string
from typing import cast
import uuid
from uuid import UUID

import pytest
import pytz

from bench.language import Struct, Node
from bench.language.const import BenchType, BENCH_TYPES, StructType
from bench.language.node import BENCH_CLASS_BY_TYPE, Property
from bench.proto import wiring
from bench.proto.wire import NodeReferenceData
from bench.sql.core import ColumnType

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
    elif prop.column_type == ColumnType.JSON:
        return {}
    elif prop.py_type_stripped in DEFAULT_GENERATORS:
        return DEFAULT_GENERATORS[prop.py_type_stripped]()
    else:
        raise ValueError(f"cannot fabricate {prop}")


def fabricate(bench_type: BenchType, path: tuple[BenchType, ...]) -> Node | Struct:
    path = path + (bench_type,)
    kwargs = {}
    bench_cls = BENCH_CLASS_BY_TYPE[bench_type]
    for prop in bench_cls.__properties__.values():
        if not prop.is_runtime or prop.is_computed:
            continue  # ignore
        elif prop.struct_type in path:  # prevent circles
            kwargs[prop.name] = () if prop.is_array else None
        elif prop.is_array:
            kwargs[prop.name] = [fabricate_prop_scalar(prop, path) for _ in range(3)]
        else:
            kwargs[prop.name] = fabricate_prop_scalar(prop, path)
    return bench_cls(**kwargs)


BENCH_OBJECTS = [fabricate(t, ()) for t in BENCH_TYPES]


@pytest.mark.parametrize("bench_obj", BENCH_OBJECTS, ids=lambda o: o.__class__.__name__)
def test_roundtrip_bytes(bench_obj: Node | Struct):
    packed_wire_obj = wiring.pack_struct(bench_obj)
    packed_bytes = bytes(packed_wire_obj)
    unpacked_wire_obj = type(packed_wire_obj)().parse(packed_bytes)
    unpacked_obj = wiring.unpack_struct(unpacked_wire_obj)
    assert unpacked_obj.equals_content(bench_obj), f"{unpacked_obj!r} != {bench_obj!r}"
