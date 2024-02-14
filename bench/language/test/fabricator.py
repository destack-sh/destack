from datetime import datetime
import enum
import random
import string
from typing import cast, TypeVar
import uuid

import pytz

from bench.language import Property, NodeReference
from bench.language.const import BenchType, StructType, PrimitiveType, NODE_TYPES
from bench.language.node import Node, Struct
from bench.language.setup import NODE_CLASS_BY_TYPE, BENCH_CLASS_BY_TYPE
from bench.proto.wire import NodeReferenceData

NodeT = TypeVar("NodeT", bound=Node)
StructT = TypeVar("StructT", bound=Struct)


class Fabricator:
    def __init__(self, seed: int = 42):
        random_random = random.Random(seed)
        self.random = random_random
        self.DEFAULT_GENERATORS: dict[type, callable] = {
            bool: lambda: self.random.choice([True, False]),
            str: lambda: "".join(self.random.choices(string.ascii_letters, k=10)),
            int: lambda: self.random.randint(0, 1000),
            float: lambda: self.random.random(),
            bytes: lambda: self.random.randbytes(24),
            uuid.UUID: lambda: uuid.uuid4(),
            datetime: lambda: datetime.utcnow().replace(tzinfo=pytz.utc),
        }

    def fabricate_prop_scalar(self, prop: Property, path: tuple[BenchType, ...]) -> any:
        if prop.is_enum:
            enum_cls = cast(type[enum.Enum], prop.py_type_stripped)
            return random.choice(tuple(enum_cls)) if len(enum_cls) > 0 else None
        elif prop.is_struct:
            return self.fabricate(prop.struct_type, path)
        elif prop.py_type_stripped == NodeReferenceData:
            return self.fabricate(StructType.NODE_REFERENCE, path)
        elif prop.primitive_type == PrimitiveType.JSON:
            return {
                self.DEFAULT_GENERATORS[str](): self.DEFAULT_GENERATORS[str](),
                self.DEFAULT_GENERATORS[str](): self.DEFAULT_GENERATORS[int](),
                self.DEFAULT_GENERATORS[str](): None,
            }
        elif prop.py_type_stripped in self.DEFAULT_GENERATORS:
            return self.DEFAULT_GENERATORS[prop.py_type_stripped]()
        else:
            raise ValueError(f"cannot fabricate {prop}")

    def fabricate(self, bench_type: BenchType, path: tuple[BenchType, ...] = ()) -> NodeT | StructT:
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
            return prop.to_ref()
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
                    kwargs[prop.name] = [self.fabricate_prop_scalar(prop, path) for _ in range(len)]
                else:
                    kwargs[prop.name] = self.fabricate_prop_scalar(prop, path)
            return bench_cls(**kwargs)
