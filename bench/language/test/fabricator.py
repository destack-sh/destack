import enum
import random
import string
import uuid
from datetime import datetime
from typing import TypeVar, cast

import pytz

from bench.language import NodeReference, Property
from bench.language.const import (
    EMPTY_DICT,
    NODE_TYPES,
    BenchType,
    PrimitiveType,
    ReferenceKind,
    StructType,
)
from bench.language.node import Node, Struct
from bench.language.setup import BENCH_CLASS_BY_TYPE, NODE_CLASS_BY_TYPE
from bench.proto.wire import NodeReferenceData
from bench.utils.fractional import INTEGER_ZERO

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

    def fabricate_prop_scalar(self, prop: Property, path: tuple[BenchType, ...] = ()) -> any:
        if prop.is_enum:
            enum_cls = cast(type[enum.Enum], prop.py_type_stripped)
            return random.choice(tuple(enum_cls)) if len(enum_cls) > 0 else None
        elif prop.is_struct:
            return self.fabricate(prop.reference_struct, path)
        elif prop.py_type_stripped == NodeReferenceData:
            return self.fabricate(StructType.NODE_REFERENCE, path)
        elif prop.primitive_type == PrimitiveType.JSON:
            return {
                self.DEFAULT_GENERATORS[str](): self.DEFAULT_GENERATORS[str](),
                self.DEFAULT_GENERATORS[str](): self.DEFAULT_GENERATORS[int](),
                self.DEFAULT_GENERATORS[str](): None,
            }
        elif prop.name == "order_key":
            return INTEGER_ZERO
        elif prop.py_type_stripped in self.DEFAULT_GENERATORS:
            return self.DEFAULT_GENERATORS[prop.py_type_stripped]()
        elif prop.primitive_type == PrimitiveType.JSON:
            return {}  # not correct, not sure what to do
        else:
            raise ValueError(f"cannot fabricate {prop!r}")

    def fabricate(
        self, bench_type: BenchType, path: tuple[BenchType, ...] = (), **override
    ) -> NodeT | StructT:
        path = path + (bench_type,)
        override = override or EMPTY_DICT

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
        else:  # default unconstrained random jumble of properties
            kwargs = {**override}
            bench_cls = BENCH_CLASS_BY_TYPE[bench_type]
            for prop in bench_cls.__wired_properties__.values():
                if prop.name in override:
                    continue
                elif prop.is_ephemeral or prop.is_computed:
                    continue
                elif prop.reference_kind == ReferenceKind.STRUCT_PARENT or (
                    prop.reference_kind
                    and prop.reference_kind.is_node_tree
                    and not prop.reference_source
                ):
                    continue  # set indirectly via the underlying NodeReference/PropertyReference
                elif prop.reference_struct in path:  # prevent circles
                    kwargs[prop.name] = [] if prop.is_list else None
                elif prop.is_list:
                    len = random.randint(1, 4)
                    kwargs[prop.name] = [self.fabricate_prop_scalar(prop, path) for _ in range(len)]
                else:
                    kwargs[prop.name] = self.fabricate_prop_scalar(prop, path)
            fabricated = bench_cls(**kwargs)
            assert fabricated.metatype == bench_type, f"{fabricated!r}.metatype is not {bench_type}"
            return fabricated
