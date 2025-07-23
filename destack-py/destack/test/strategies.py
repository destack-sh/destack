import zoneinfo
from itertools import chain
from string import ascii_letters, ascii_lowercase
from typing import Any, assert_never, cast

import fastuuid
import hypothesis
import more_itertools
import structlog
from cachetools import cached
from hypothesis import example, given, settings
from hypothesis import strategies as st
from hypothesis.strategies._internal.utils import cacheable, defines_strategy

from destack.language import (
    ENUM_CLASS_BY_TYPE,
    NODE_CLASS_BY_TYPE,
    BuiltinObject,
    CustomProperty,
    EnumType,
    IconType,
    NodeReference,
    NodeType,
    PrimitiveType,
    PropertyType,
    ScalarType,
    StructType,
    Type,
    TypeCardinality,
    icon,
)
from destack.language.registry import STRUCT_CLASS_BY_TYPE, get_builtin_object_cls

logger = structlog.get_logger(__name__)

ALL_DECLARED_PROPERTIES = tuple(
    more_itertools.flatten(
        (p for p in object_cls.__declared_properties__.values() if p.id is not None)
        for object_cls in chain(NODE_CLASS_BY_TYPE.values(), STRUCT_CLASS_BY_TYPE.values())
    )
)


def examples(examples: list[dict]):
    """Decorator to provide many hypothesis examples in one go"""

    def apply_examples(func):
        for example_kwargs in examples:
            example(**example_kwargs)(func)
        return func

    return apply_examples


# NOTE :Test: figure out better way of drawing directly from strategies
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


DURATION_STRATEGY = st.floats(min_value=0, allow_nan=False, allow_infinity=False)
JSON_STRATEGY = st.none()  # not needed yet
ORDER_KEY_STRATEGY = st.one_of(
    st.just("a0"), st.just("a1"), st.just("a2"), st.just("a3"), st.just("a4")
)
BYTES_STRATEGY = st.binary(min_size=1, max_size=32)
NAME_STRATEGY = st.text(alphabet=ascii_lowercase, min_size=1, max_size=64)
SLUG_STRATEGY = st.text(alphabet=ascii_lowercase, min_size=1, max_size=64)


@cacheable
@defines_strategy()
def properties(object_type: NodeType | StructType | None = None):
    if object_type is None:
        return st.sampled_from(ALL_DECLARED_PROPERTIES)
    else:
        object_cls = get_builtin_object_cls(object_type)
        return st.sampled_from(
            tuple(p for p in object_cls.__declared_properties__.values() if p.id is not None)
        )


@cacheable
@defines_strategy()
def get_scalar_type_strategy(
    typ: Type,
) -> st.SearchStrategy[Any]:
    """Get a strategy for generating scalar values based on the scalar type and specific type info."""
    if typ.scalar_type == ScalarType.PRIMITIVE:
        assert typ.primitive_type is not None, "primitive_type required for PRIMITIVE scalar_type"
        return get_primitive_strategy(typ.primitive_type)
    elif typ.scalar_type == ScalarType.ENUM:
        assert typ.enum_type is not None, "enum_type required for ENUM scalar_type"
        enum_cls = ENUM_CLASS_BY_TYPE[typ.enum_type]
        return st.sampled_from(enum_cls)
    elif typ.scalar_type == ScalarType.STRUCT:
        assert typ.struct_type is not None, "struct_type required for STRUCT scalar_type"
        return from_object_type(typ.struct_type)
    elif typ.scalar_type == ScalarType.NODE_REFERENCE:
        return node_references(st.sampled_from(NODE_TYPES))
    elif typ.scalar_type == ScalarType.NODE_VALUE:
        return from_object_type(typ.node_types[0] if typ.node_types else NodeType.SPACE)
    else:
        assert_never(typ.scalar_type)


@cacheable
@defines_strategy()
def get_primitive_strategy(primitive_type: PrimitiveType) -> st.SearchStrategy[Any]:
    """Get a strategy for generating primitive values with proper constraints."""
    if primitive_type in (PrimitiveType.INT16, PrimitiveType.INT32, PrimitiveType.INT64):
        min_value = cast(int, MIN_VALUE_BY_PRIMITIVE_TYPE[primitive_type])
        max_value = cast(int, MAX_VALUE_BY_PRIMITIVE_TYPE[primitive_type])
        return st.integers(min_value=min_value, max_value=max_value)
    elif primitive_type in (PrimitiveType.FLOAT32, PrimitiveType.FLOAT64):
        min_value = cast(float, MIN_VALUE_BY_PRIMITIVE_TYPE[primitive_type])
        max_value = cast(float, MAX_VALUE_BY_PRIMITIVE_TYPE[primitive_type])
        return st.floats(
            min_value=min_value,
            max_value=max_value,
            allow_nan=False,
            allow_infinity=False,
        )
    elif primitive_type == PrimitiveType.STRING:
        return st.text(min_size=1, alphabet=ascii_letters)
    else:
        return STRATEGY_BY_PRIMITIVE_TYPE[primitive_type]


def _wrap_value_scalar_strategy(
    strat: st.SearchStrategy,
    *,
    is_required: bool,
    cardinality: TypeCardinality,
    min_length: int = 0,
    max_length: int | None = None,
    key_type: Type | None = None,
) -> st.SearchStrategy[Any]:
    """Wrap a scalar strategy based on cardinality and requirements."""
    if cardinality == TypeCardinality.LIST:
        return st.lists(strat, min_size=min_length, max_size=max_length)
    elif cardinality == TypeCardinality.MAP:
        assert key_type is not None, "key_type required for MAP cardinality"
        key_strategy = get_type_strategy(key_type)
        return st.dictionaries(key_strategy, strat, min_size=min_length, max_size=max_length)
    elif not is_required:
        return st.none() | strat
    else:
        return strat


@cacheable
@defines_strategy()
def get_type_strategy(typ: Type) -> st.SearchStrategy[Any]:
    """Generate a strategy from a Type object, handling all cardinalities and constraints."""
    scalar_strategy = get_scalar_type_strategy(typ)
    return _wrap_value_scalar_strategy(
        scalar_strategy,
        is_required=typ.is_required or False,
        cardinality=typ.cardinality,
        min_length=0,
        max_length=None,
        key_type=typ.key_type,
    )


@cached({})
def get_naive_object_strategy(object_cls: type[BuiltinObject]):
    """Gets the default uncorrelated strategies for every (init) property of an object type."""
    object_kwargs: dict[str, st.SearchStrategy] = {}
    for prop in object_cls.__wired_properties__.values():
        if (
            prop.id is None
            or prop.id == 1
            or prop.default_factory is not None
            or (prop.is_internal and prop.name != "definition")
            or prop.is_computed
            or prop.is_unique  # is unique (causes meaningless test errors)
        ):
            continue  # ignore
        if prop.scalar_type == ScalarType.NODE_REFERENCE:
            prop_name = prop.name + "_ptr"
        else:
            prop_name = prop.name
        if prop.name in STRATEGY_BY_PROPERTY:
            object_kwargs[prop_name] = STRATEGY_BY_PROPERTY[prop.name]
        else:
            object_kwargs[prop_name] = get_type_strategy(prop.to_type())

    return object_kwargs


@cacheable
@defines_strategy()
def from_object_type(
    object_type: NodeType | StructType,
    /,
    **custom_strategies: st.SearchStrategy,
) -> st.SearchStrategy[BuiltinObject]:
    # special cases
    if isinstance(object_type, StructType):
        if object_type == StructType.TYPE:
            return cast(st.SearchStrategy[BuiltinObject], types(SIMPLE_TYPE_CARDINALITIES))
        elif object_type == StructType.PROPERTY_REFERENCE:
            return cast(
                st.SearchStrategy[BuiltinObject],
                properties(object_type=None).map(lambda p: p.to_ref()),
            )
        elif object_type == StructType.ICON:
            return cast(st.SearchStrategy[BuiltinObject], icons())
    elif isinstance(object_type, NodeType):
        if object_type == NodeType.CUSTOM_PROPERTY:
            return cast(st.SearchStrategy[BuiltinObject], fields(SIMPLE_TYPE_CARDINALITIES))

    # naive strategy
    object_cls = get_builtin_object_cls(object_type)
    object_dict = get_naive_object_strategy(object_cls)
    if custom_strategies:
        object_dict = {**object_dict}
        object_dict.update(custom_strategies)
    return st.builds(object_cls, **object_dict)


@cacheable
@st.composite
def node_references(draw: st.DrawFn, node_types: st.SearchStrategy[NodeType]):
    node_type = draw(node_types)
    node_id = draw(STRATEGY_BY_PRIMITIVE_TYPE[PrimitiveType.UUID])
    space_id = draw(STRATEGY_BY_PRIMITIVE_TYPE[PrimitiveType.UUID])
    branch_id = draw(STRATEGY_BY_PRIMITIVE_TYPE[PrimitiveType.UUID])
    snapshot_id = draw(STRATEGY_BY_PRIMITIVE_TYPE[PrimitiveType.UUID])
    return NodeReference(
        type=node_type,
        id=node_id,
        space_id=space_id,
        branch_id=branch_id,
        snapshot_id=snapshot_id,
    )


@cacheable
@defines_strategy()
def uuids():
    """Draw from fastuuid.uuid4"""
    return st.just(fastuuid.uuid4())


STRATEGY_BY_PRIMITIVE_TYPE: dict[PrimitiveType, st.SearchStrategy] = {
    PrimitiveType.BOOLEAN: st.booleans(),
    PrimitiveType.INT16: st.integers(min_value=-(2**15), max_value=2**15 - 1),
    PrimitiveType.INT32: st.integers(min_value=-(2**31), max_value=2**31 - 1),
    PrimitiveType.INT64: st.integers(min_value=-(2**63), max_value=2**63 - 1),
    PrimitiveType.DECIMAL: st.decimals(),
    PrimitiveType.FLOAT32: st.floats(allow_nan=False, allow_infinity=False),
    PrimitiveType.FLOAT64: st.floats(allow_nan=False, allow_infinity=False),
    PrimitiveType.STRING: st.text(min_size=1),
    PrimitiveType.UUID: uuids(),
    PrimitiveType.BYTES: st.binary(),
    PrimitiveType.DATETIME: st.datetimes(timezones=st.just(zoneinfo.ZoneInfo("UTC"))),
    PrimitiveType.DURATION: st.timedeltas(),
    PrimitiveType.JSON: JSON_STRATEGY,
    PrimitiveType.CSON: JSON_STRATEGY,
    PrimitiveType.PROTO: st.binary(),
}

MIN_VALUE_BY_PRIMITIVE_TYPE: dict[PrimitiveType, int | float] = {
    PrimitiveType.INT16: -(2**15),
    PrimitiveType.INT32: -(2**31),
    PrimitiveType.INT64: -(2**63),
    PrimitiveType.FLOAT32: -3.4e38,
    PrimitiveType.FLOAT64: -1.7e308,
}

MAX_VALUE_BY_PRIMITIVE_TYPE: dict[PrimitiveType, int | float] = {
    PrimitiveType.INT16: 2**15 - 1,
    PrimitiveType.INT32: 2**31 - 1,
    PrimitiveType.INT64: 2**63 - 1,
    PrimitiveType.FLOAT32: 3.4e38,
    PrimitiveType.FLOAT64: 1.7e308,
}

STRATEGY_BY_PROPERTY: dict[str, st.SearchStrategy] = {
    "order_key": ORDER_KEY_STRATEGY,
    "name": NAME_STRATEGY,
    "slug": SLUG_STRATEGY,
}


def draw_type_base_dict(
    draw: st.DrawFn, cardinalities: st.SearchStrategy[TypeCardinality]
) -> dict[str, Any]:
    cardinality = draw(cardinalities)
    scalar_type = draw(st.sampled_from(ScalarType))
    primitive_type = None
    enum_type = None
    struct_type = None
    node_type = None

    if scalar_type == ScalarType.PRIMITIVE:
        primitive_type = draw(st.sampled_from(PrimitiveType))
    elif scalar_type == ScalarType.ENUM:
        enum_type = draw(st.sampled_from(EnumType))
    elif scalar_type == ScalarType.STRUCT:
        struct_type = draw(st.sampled_from(StructType))
    elif scalar_type == ScalarType.NODE_REFERENCE or scalar_type == ScalarType.NODE_VALUE:
        node_type = draw(st.sampled_from(NODE_TYPES))
    else:
        assert_never(scalar_type)

    return {
        "cardinality": cardinality,
        "scalar_type": scalar_type,
        "primitive_type": primitive_type,
        "enum_type": enum_type,
        "struct_type": struct_type,
        "node_types": [node_type] if node_type else [],
        "is_required": True,
    }


SIMPLE_TYPE_CARDINALITIES = st.sampled_from([TypeCardinality.SCALAR])


@cacheable
@st.composite
def types(draw: st.DrawFn, cardinalities: st.SearchStrategy[TypeCardinality]):
    base_dict = draw_type_base_dict(draw, cardinalities)
    return Type(**base_dict)


@st.composite
def fields(draw: st.DrawFn, cardinalities: st.SearchStrategy[TypeCardinality]):
    type_base_dict = draw_type_base_dict(draw, cardinalities)
    naive_base_dict = get_naive_object_strategy(CustomProperty)
    naive_base_dict["type"] = st.just(PropertyType.INPUT)
    combined_dict = {}
    for key in naive_base_dict:
        # prefer type info where set
        if key in type_base_dict:
            combined_dict[key] = type_base_dict[key]
        else:
            combined_dict[key] = draw(naive_base_dict[key])
    combined_dict["cardinality"] = TypeCardinality.SCALAR
    return CustomProperty(**combined_dict)


@st.composite
def icons(draw: st.DrawFn):
    kind = draw(st.sampled_from(list(IconType)))
    if kind == IconType.EMOJI:
        return icon("😀")
    else:
        return icon("fas fa-circle-dot")


NODE_TYPES = [
    node_type for node_type, node_cls in NODE_CLASS_BY_TYPE.items() if not node_cls.__is_abstract__
]
STRUCT_TYPES = [
    struct_type
    for struct_type, struct_cls in STRUCT_CLASS_BY_TYPE.items()
    if not struct_cls.__is_abstract__
]
OBJECT_TYPES = NODE_TYPES + STRUCT_TYPES


@cacheable
@st.composite
def builtin_objects(
    draw: st.DrawFn,
    object_types: st.SearchStrategy[NodeType | StructType] = st.sampled_from(  # noqa: B008
        OBJECT_TYPES
    ),
):
    object_type = draw(object_types)
    return draw(from_object_type(object_type))


nodes = builtin_objects(object_types=st.sampled_from(NODE_TYPES))
structs = builtin_objects(object_types=st.sampled_from(STRUCT_TYPES))
