from typing import Any, cast

import hypothesis
import more_itertools
import structlog
from cachetools import cached
from hypothesis import example, given, reject, settings
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
from bench.language.expression import NodeReference
from bench.language.field import DEFAULT_CONSTRAINT, Field, TypeInfoBase
from bench.language.node import BuiltinObject
from bench.language.setup import ENUM_CLASS_BY_TYPE, NODE_CLASS_BY_TYPE, OBJECT_CLASS_BY_TYPE
from bench.language.validation import ValidationError
from bench.utils.fractional import INTEGER_ZERO

logger = structlog.get_logger(__name__)


def examples(examples: list[dict]):
    """Decorator to provide many hypothesis examples in one go"""

    def apply_examples(func):
        for example_kwargs in examples:
            example(**example_kwargs)(func)
        return func

    return apply_examples


# TODO :Broken! :Test!: figure out better way of drawing directly from strategies
#  (outside of given context.. maybe have our own dummy conjecture data with a seed?)
def draw_direct(strat: st.SearchStrategy):
    examples = []

    @given(strat)
    @settings(
        database=None,
        max_examples=1,
        deadline=None,
        verbosity=hypothesis.Verbosity.quiet,
        phases=(hypothesis.Phase.generate,),
    )
    def example_generating_inner_function(ex):
        examples.append(ex)

    example_generating_inner_function()

    return examples[0]


JSON_STRATEGY = st.none()  # not needed yet
ORDER_KEY_STRATEGY = st.just(INTEGER_ZERO)  # TODO :Test: generate order keys properly
ALL_DECLARED_PROPERTIES = tuple(
    more_itertools.flatten(
        object_cls.__declared_properties__.values() for object_cls in OBJECT_CLASS_BY_TYPE.values()
    )
)
PROPERTY_STRATEGY = st.sampled_from(ALL_DECLARED_PROPERTIES)

TYPE_KIND_STRATEGY = st.sampled_from(TypeKind)
PRIMITIVE_TYPE_STRATEGY = st.sampled_from(PrimitiveType)
ENUM_TYPE_STRATEGY = st.sampled_from(EnumType)
STRUCT_TYPE_STRATEGY = st.sampled_from(StructType)
OBJECT_TYPE_STRATEGY = st.sampled_from(ObjectType)

MIN_BY_PRIMITIVE_TYPE: dict[PrimitiveType, int] = {
    PrimitiveType.INT16: -(2**15),
    PrimitiveType.INT32: -(2**31),
    PrimitiveType.INT64: -(2**63),
}
MAX_BY_PRIMITIVE_TYPE: dict[PrimitiveType, int] = {
    PrimitiveType.INT16: 2**15 - 1,
    PrimitiveType.INT32: 2**31 - 1,
    PrimitiveType.INT64: 2**63 - 1,
}

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


object_types = st.sampled_from(ObjectType)
node_types = st.sampled_from(NodeType)
struct_types = st.sampled_from(StructType)


@cacheable
@defines_strategy()
def from_type_info_scalar(typ: TypeInfoBase) -> st.SearchStrategy[Any]:
    """Turns a type into a strategy for a scalar. Considers constraints."""
    constraint = typ.constraint or DEFAULT_CONSTRAINT
    if typ.kind == TypeKind.PRIMITIVE:
        # apply constraints to primitive types
        assert typ.primitive_type is not None, f"{typ!r} has no primitive type"
        if typ.primitive_type in (PrimitiveType.INT16, PrimitiveType.INT32, PrimitiveType.INT64):
            min_value = (
                int(constraint.min_value)
                if constraint.min_value is not None
                else MIN_BY_PRIMITIVE_TYPE[typ.primitive_type]
            )
            max_value = (
                int(constraint.max_value)
                if constraint.max_value is not None
                else MAX_BY_PRIMITIVE_TYPE[typ.primitive_type]
            )
            return st.integers(min_value=min_value, max_value=max_value)
        elif typ.primitive_type in (PrimitiveType.FLOAT32, PrimitiveType.FLOAT64):
            return st.floats(
                min_value=constraint.min_value,
                max_value=constraint.max_value,
                allow_nan=False,
                allow_infinity=False,
            )
        elif typ.primitive_type == PrimitiveType.STRING:
            if constraint.regex:
                return st.from_regex(constraint.regex)
            elif constraint.starts_with or constraint.ends_with:
                regex = rf"^{constraint.starts_with or ''}.*{constraint.ends_with or ''}$"
                return st.from_regex(regex)
            else:
                return st.text(min_size=constraint.min_length or 0, max_size=constraint.max_length)
        else:
            return STRATEGY_BY_PRIMITIVE_TYPE[typ.primitive_type]
    elif typ.kind == TypeKind.ENUM:
        assert typ.bench_type is not None, f"{typ!r} has no enum type"
        enum_cls = ENUM_CLASS_BY_TYPE[cast(EnumType, typ.bench_type)]
        return st.sampled_from(enum_cls)
    elif typ.kind == TypeKind.STRUCT:
        assert typ.bench_type is not None, f"{typ!r} has no struct type"
        return from_object_type(cast(StructType, typ.bench_type))
    else:
        raise NotImplementedError(f"{typ!r} has no strategy yet")


def wrap_value_scalar(
    strat: st.SearchStrategy,
    *,
    is_required: bool,
    is_list: bool,
    min_length: int = 0,
    max_length: int | None = None,
) -> st.SearchStrategy[Any]:
    if is_list:
        return st.lists(strat, min_size=min_length, max_size=max_length)
    elif not is_required:
        return st.none() | strat
    else:
        return strat


@cacheable
@defines_strategy()
def from_type_info(typ: TypeInfoBase) -> st.SearchStrategy[Any]:
    value_st = from_type_info_scalar(typ)
    constraint = typ.constraint or DEFAULT_CONSTRAINT
    return wrap_value_scalar(
        value_st,
        is_required=typ.is_required,
        is_list=typ.is_list,
        min_length=constraint.min_length or 0,
        max_length=constraint.max_length or None,
    )


@cached({})
def get_naive_object_strategies(object_type: ObjectType):
    """Gets the default uncorrelated strategies for every (init) property of an object type."""
    object_cls = OBJECT_CLASS_BY_TYPE[object_type]
    object_dict: dict[str, st.SearchStrategy] = {}
    for prop in object_cls.__runtime_properties__.values():
        if (
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
            object_dict[prop.reference_wired_ptr.name] = wrap_value_scalar(
                node_references(st.sampled_from(prop.reference_nodes)),
                is_required=prop.is_required,
                is_list=prop.is_list,
            )
        elif prop.is_property_reference:
            object_dict[prop.name] = wrap_value_scalar(
                properties(), is_required=prop.is_required, is_list=prop.is_list
            )
        elif prop.is_value_runtime or prop.is_value_packed:
            object_dict[prop.name] = st.none()  # TODO :Test :Incomplete: add strategy for Values
        else:
            object_dict[prop.name] = from_type_info(prop.type_info)
    return object_dict


@cacheable
@defines_strategy()
def from_object_type(
    object_type: ObjectType,
    /,
    reject_invalid: bool = True,
    **custom_strategies: st.SearchStrategy,
) -> st.SearchStrategy[BuiltinObject]:
    # special case some types
    if object_type == StructType.TYPE_INFO:
        return cast(st.SearchStrategy[BuiltinObject], type_infos(SIMPLE_TYPE_KINDS))
    elif object_type == StructType.PROPERTY_REFERENCE:
        return cast(st.SearchStrategy[BuiltinObject], properties(object_type=None))
    elif object_type == NodeType.FIELD:
        return cast(st.SearchStrategy[BuiltinObject], fields(SIMPLE_TYPE_KINDS))

    object_cls = OBJECT_CLASS_BY_TYPE[object_type]
    object_dict = get_naive_object_strategies(object_type)
    if custom_strategies:
        object_dict = {**object_dict}
        object_dict.update(custom_strategies)

    if reject_invalid:

        def _make_object_if_valid(**kwargs):
            # despite best efforts, some combination of arguments may be invalid
            #  (this may be simpler than guaranteeing that all generated objects are valid upfront)
            try:
                return object_cls(**kwargs)
            except ValidationError:
                reject()

        buildf = _make_object_if_valid
    else:
        buildf = object_cls

    return st.builds(buildf, **object_dict)


@cacheable
@st.composite
def node_references(draw: st.DrawFn, node_types: st.SearchStrategy[NodeType]):
    node_type = draw(node_types)
    node_id = draw(STRATEGY_BY_PRIMITIVE_TYPE[PrimitiveType.UUID])
    node_cls = NODE_CLASS_BY_TYPE[node_type]
    if "ck" in node_cls.__properties__:
        node_ck = draw(STRATEGY_BY_PRIMITIVE_TYPE[PrimitiveType.UUID])
    else:
        node_ck = node_id
    return NodeReference(type=node_type, id=node_id, ck=node_ck)


SIMPLE_TYPE_KINDS = st.sampled_from((TypeKind.PRIMITIVE, TypeKind.ENUM, TypeKind.STRUCT))


def draw_type_info_base_dict(draw: st.DrawFn, kinds: st.SearchStrategy[TypeKind]) -> dict[str, Any]:
    kind = draw(kinds)
    if kind == TypeKind.PRIMITIVE:
        primitive_type = draw(PRIMITIVE_TYPE_STRATEGY)
        return {"kind": kind, "primitive_type": primitive_type}
    elif kind == TypeKind.ENUM:
        enum_type = draw(ENUM_TYPE_STRATEGY)
        return {"kind": kind, "bench_type": enum_type}
    elif kind == TypeKind.STRUCT:
        struct_type = draw(STRUCT_TYPE_STRATEGY)
        return {"kind": kind, "bench_type": struct_type}
    else:
        raise NotImplementedError(f"TypeKind {kind!r} not implemented")


@cacheable
@st.composite
def type_infos(draw: st.DrawFn, kinds: st.SearchStrategy[TypeKind]):
    type_info_base_dict = draw_type_info_base_dict(draw, kinds)
    return TypeInfoBase(**type_info_base_dict)


@st.composite
def fields(draw: st.DrawFn, kinds: st.SearchStrategy[TypeKind]):
    type_info_base_dict = draw_type_info_base_dict(draw, kinds)
    naive_base_dict = get_naive_object_strategies(NodeType.FIELD)
    combined_dict = {}
    for key in naive_base_dict:
        # prefer type info where set
        if key in type_info_base_dict:
            combined_dict[key] = type_info_base_dict[key]
        else:
            combined_dict[key] = draw(naive_base_dict[key])
    return Field(**combined_dict)


@cacheable
@st.composite
def builtin_objects(draw: st.DrawFn, object_types: st.SearchStrategy[ObjectType] = object_types):
    object_type = draw(object_types)
    return draw(from_object_type(object_type, reject_invalid=True))


nodes = builtin_objects(object_types=st.sampled_from(NodeType))
structs = builtin_objects(object_types=st.sampled_from(StructType))
