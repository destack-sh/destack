from string import printable
from typing import Any, Sequence, cast

import more_itertools
import structlog
from hypothesis import strategies as st
from hypothesis.strategies._internal.utils import cacheable, defines_strategy

from bench.language.const import (
    EnumType,
    NodeType,
    ObjectType,
    PrimitiveType,
    StructType,
    TypeKind,
)
from bench.language.field import TypeInfoBase
from bench.language.node import BuiltinObject
from bench.language.setup import ENUM_CLASS_BY_TYPE, OBJECT_CLASS_BY_TYPE
from bench.utils.fractional import INTEGER_ZERO

logger = structlog.get_logger(__name__)

JSON_STRATEGY = st.recursive(
    st.none() | st.booleans() | st.floats() | st.text(printable),
    lambda children: st.lists(children) | st.dictionaries(st.text(printable), children),
    max_leaves=5,  # only small json
)
ORDER_KEY_STRATEGY = st.just(INTEGER_ZERO)  # TODO :Test: generate order keys properly
ALL_DECLARED_PROPERTIES = tuple(
    more_itertools.flatten(
        object_cls.__declared_properties__.values() for object_cls in OBJECT_CLASS_BY_TYPE.values()
    )
)
PROPERTY_STRATEGY = st.sampled_from(ALL_DECLARED_PROPERTIES)

STRATEGY_BY_PRIMITIVE_TYPE: dict[PrimitiveType, st.SearchStrategy] = {
    PrimitiveType.BOOLEAN: st.booleans(),
    PrimitiveType.INT16: st.integers(min_value=-(2**15), max_value=2**15 - 1),
    PrimitiveType.INT32: st.integers(min_value=-(2**31), max_value=2**31 - 1),
    PrimitiveType.INT64: st.integers(min_value=-(2**63), max_value=2**63 - 1),
    PrimitiveType.DECIMAL: st.decimals(),
    PrimitiveType.FLOAT32: st.floats(allow_nan=False, allow_infinity=False),
    PrimitiveType.FLOAT64: st.floats(allow_nan=False, allow_infinity=False),
    PrimitiveType.STRING: st.text(),
    PrimitiveType.UUID: st.uuids(),
    PrimitiveType.JSON: JSON_STRATEGY,
    PrimitiveType.BYTES: st.binary(),
    PrimitiveType.DATETIME: st.datetimes(),
    PrimitiveType.INTERVAL: st.timedeltas(),
}

STRATEGY_BY_PROPERTY_NAME: dict[str, st.SearchStrategy] = {
    "order_key": ORDER_KEY_STRATEGY,
}


@cacheable
@defines_strategy()
def properties(object_type: ObjectType | None = None):
    if object_type is None:
        return PROPERTY_STRATEGY
    else:
        object_cls = OBJECT_CLASS_BY_TYPE[object_type]
        return st.sampled_from(tuple(object_cls.__declared_properties__.values()))


@cacheable
@defines_strategy()
def from_primitive_type(primitive_type: PrimitiveType) -> st.SearchStrategy[Any]:
    return STRATEGY_BY_PRIMITIVE_TYPE[primitive_type]


@cacheable
@defines_strategy()
def from_enum_type(enum_type: EnumType) -> st.SearchStrategy:
    enum_cls = ENUM_CLASS_BY_TYPE[enum_type]
    return st.sampled_from(enum_cls)


object_types = from_enum_type(EnumType.OBJECT_TYPE)
node_types = from_enum_type(EnumType.NODE_TYPE)
struct_types = from_enum_type(EnumType.STRUCT_TYPE)


@cacheable
@defines_strategy()
def from_type_info_scalar(typ: TypeInfoBase) -> st.SearchStrategy[Any]:
    if typ.kind == TypeKind.PRIMITIVE:
        assert typ.primitive_type is not None, f"{typ!r} has no primitive type"
        return from_primitive_type(typ.primitive_type)
    elif typ.kind == TypeKind.ENUM:
        assert typ.bench_type is not None, f"{typ!r} has no enum type"
        return from_enum_type(cast(EnumType, typ.bench_type))
    elif typ.kind == TypeKind.STRUCT:
        assert typ.bench_type is not None, f"{typ!r} has no struct type"
        return from_object_type(cast(StructType, typ.bench_type))
    else:
        raise NotImplementedError(f"{typ!r} has no strategy yet")


def wrap_value_scalar(
    strat: st.SearchStrategy, *, is_required: bool, is_list: bool
) -> st.SearchStrategy[Any]:
    if is_list:
        return st.lists(strat, min_size=0, max_size=10)
    elif not is_required:
        return st.none() | strat
    else:
        return strat


@cacheable
@defines_strategy()
def from_type_info(typ: TypeInfoBase) -> st.SearchStrategy[Any]:
    value_st = from_type_info_scalar(typ)
    return wrap_value_scalar(value_st, is_required=typ.is_required, is_list=typ.is_list)


@cacheable
@defines_strategy()
def from_object_type(
    object_type: ObjectType, /, **custom_strategies: st.SearchStrategy
) -> st.SearchStrategy[BuiltinObject]:
    object_cls = OBJECT_CLASS_BY_TYPE[object_type]
    object_dict: dict[str, st.SearchStrategy] = {}
    for prop in object_cls.__runtime_properties__.values():
        if prop.name in custom_strategies:
            object_dict[prop.name] = custom_strategies[prop.name]
        elif (
            # ignore runtime-only properties
            prop.id is None
            # ignore identity/tracking properties
            or (prop.id < 30 and prop._type_info is None)
            # ignore contributed wired properties (they're derived from the generated one)
            or (prop.reference_source is not None)
            # ignore autoset properties (id, timestamps)
            or prop.is_autoset
        ):
            continue
        elif prop.name in STRATEGY_BY_PROPERTY_NAME:
            object_dict[prop.name] = STRATEGY_BY_PROPERTY_NAME[prop.name]
        elif prop.is_node_reference:
            # generate reference instead of node
            assert prop.reference_wired_ptr is not None, f"{prop!r} has no wired ptr"
            assert prop.reference_nodes, f"{prop!r} has no reference nodes"
            object_dict[prop.reference_wired_ptr.name] = node_references(prop.reference_nodes)
        elif prop.is_property_reference:
            object_dict[prop.name] = wrap_value_scalar(
                properties(), is_required=prop.is_required, is_list=prop.is_list
            )
        else:
            object_dict[prop.name] = from_type_info(prop.type_info)
    return st.builds(object_cls, **object_dict)


@st.composite
def node_references(draw, node_types: Sequence[NodeType]):
    node_type = st.sampled_from(node_types)
    return from_object_type(StructType.NODE_REFERENCE, node_type=node_type)


@st.composite
def builtin_objects(draw):
    object_type = draw(object_types)
    return draw(from_object_type(object_type))


@st.composite
def nodes(draw):
    node_type = draw(node_types)
    return draw(from_object_type(node_type))


@st.composite
def structs(draw):
    struct_type = draw(struct_types)
    return draw(from_object_type(struct_type))
