import enum
import random
import string
import uuid
from datetime import datetime, timedelta
from typing import Any, Callable, Type, TypeVar, cast

import structlog

from bench.language import NodeReference, Property
from bench.language.const import (
    EMPTY_DICT,
    NODE_TYPES,
    ObjectType,
    PrimitiveType,
    ReferenceKind,
    StructType,
)
from bench.language.node import BuiltinObject, InlineStruct, Node, Struct
from bench.language.setup import NODE_CLASS_BY_TYPE, OBJECT_CLASS_BY_TYPE
from bench.proto.wire import NodeReferenceData
from bench.utils.fractional import INTEGER_ZERO
from bench.utils.oracle import get_oracle

logger = structlog.get_logger(__name__)

NodeT = TypeVar("NodeT", bound=Node)
StructT = TypeVar("StructT", bound=Struct | InlineStruct)
ObjectT = TypeVar("ObjectT", bound=BuiltinObject)


class Fabricator:
    def __init__(self, seed: int = 42):
        random_random = random.Random(seed)
        self.random = random_random
        self.generators: dict[type, Callable] = {
            bool: lambda: self.random.choice([True, False]),
            str: lambda: "".join(self.random.choices(string.ascii_letters, k=10)),
            int: lambda: self.random.randint(0, 1000),
            float: lambda: self.random.random(),
            bytes: lambda: self.random.randbytes(24),
            uuid.UUID: lambda: uuid.uuid4(),
            datetime: lambda: get_oracle().utc(),
            timedelta: lambda: timedelta(seconds=self.random.randint(0, 1000)),
        }

    def fabricate_prop_scalar(self, prop: Property, path: tuple[ObjectType, ...] = ()) -> Any:
        if prop.is_enum:
            enum_cls = cast(type[enum.Enum], prop.py_type_stripped)
            return random.choice(tuple(enum_cls)) if len(enum_cls) > 0 else None
        elif prop.is_struct:
            assert prop.reference_struct
            return self.fabricate(OBJECT_CLASS_BY_TYPE[prop.reference_struct], path)
        elif prop.py_type_stripped == NodeReferenceData:
            return self.fabricate(OBJECT_CLASS_BY_TYPE[StructType.NODE_REFERENCE], path)
        elif prop.primitive_type == PrimitiveType.JSON:
            return {
                self.generators[str](): self.generators[str](),
                self.generators[str](): self.generators[int](),
                self.generators[str](): None,
            }
        elif prop.name == "order_key":
            return INTEGER_ZERO
        elif prop.py_type_stripped in self.generators:
            return self.generators[prop.py_type_stripped]()
        elif prop.primitive_type == PrimitiveType.JSON:
            return {}  # not correct, not sure what to do
        else:
            raise ValueError(f"cannot fabricate {prop!r}")

    def fabricate(
        self, object_cls: Type[ObjectT], path: tuple[ObjectType, ...] = (), **override
    ) -> ObjectT:
        object_type = object_cls.metatype
        path = (*path, object_type)
        override = override or EMPTY_DICT

        # special cases for semantic correctness
        if object_type == StructType.NODE_REFERENCE:
            type = random.choice(NODE_TYPES)
            id = uuid.uuid4()
            ck = uuid.uuid4() if "ck" in NODE_CLASS_BY_TYPE[type].__properties__ else id
            return cast(ObjectT, NodeReference(type=type, id=id, ck=ck))
        elif object_type == StructType.PROPERTY_REFERENCE:
            type = random.choice(NODE_TYPES)
            prop = random.choice(
                tuple(p for p in NODE_CLASS_BY_TYPE[type].__properties__.values() if p.id)
            )
            ref = prop.to_ref()
            return ref  # type: ignore
        else:  # default unconstrained random jumble of properties
            kwargs = {**override}
            bench_cls = OBJECT_CLASS_BY_TYPE[object_type]
            for prop in bench_cls.__wired_properties__.values():
                if prop.name in override or (prop.is_ephemeral or prop.is_computed):
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
            assert (
                fabricated.metatype == object_type
            ), f"{fabricated!r}.metatype is not {object_type}"
            return cast(ObjectT, fabricated)
