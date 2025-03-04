from string import ascii_lowercase
from typing import Any, assert_never, cast

import hypothesis
import more_itertools
import pytz
import structlog
from cachetools import cached
from hypothesis import example, given, reject, settings
from hypothesis import strategies as st
from hypothesis.strategies import SearchStrategy
from hypothesis.strategies._internal.utils import cacheable, defines_strategy

from bench.language import (
    BUILTIN_OBJECT_CLASS_BY_TYPE,
    ENUM_CLASS_BY_TYPE,
    MAX_VALUE_BY_PRIMITIVE_TYPE,
    MIN_VALUE_BY_PRIMITIVE_TYPE,
    NODE_CLASS_BY_TYPE,
    NODE_TYPES,
    BuiltinObject,
    EnumType,
    Field,
    FieldType,
    Icon,
    IconType,
    NodeReference,
    NodeType,
    ObjectType,
    PrimitiveType,
    StructType,
    Type,
    TypeBase,
    TypeConstraint,
    TypeKind,
    ValidationError,
)
from bench.utils.fractional import INTEGER_ZERO
from bench.utils.oracle import MAX_SCHEDULE_DURATION

logger = structlog.get_logger(__name__)

ALL_DECLARED_PROPERTIES = tuple(
    more_itertools.flatten(
        (p for p in object_cls.__declared_properties__.values() if p.id is not None)
        for object_cls in BUILTIN_OBJECT_CLASS_BY_TYPE.values()
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


DURATION_STRATEGY = st.floats(
    min_value=0, max_value=MAX_SCHEDULE_DURATION, allow_nan=False, allow_infinity=False
)
JSON_STRATEGY = st.none()  # not needed yet
ORDER_KEY_STRATEGY = st.just(INTEGER_ZERO)  # NOTE :Test: generate order keys properly
BYTES_STRATEGY = st.binary(min_size=1, max_size=32)
PROPERTY_STRATEGY = st.sampled_from(ALL_DECLARED_PROPERTIES)
NAME_STRATEGY = st.text(alphabet=ascii_lowercase, min_size=1, max_size=64)
SLUG_STRATEGY = st.text(alphabet=ascii_lowercase, min_size=1, max_size=64)

TYPE_KIND_STRATEGY = st.sampled_from(TypeKind)
PRIMITIVE_TYPE_STRATEGY = st.sampled_from(PrimitiveType)
ENUM_TYPE_STRATEGY = st.sampled_from(EnumType)
STRUCT_TYPE_STRATEGY = st.sampled_from(StructType)
OBJECT_TYPE_STRATEGY: SearchStrategy[ObjectType] = st.sampled_from(ObjectType)  # type: ignore


@cacheable
@defines_strategy()
def properties(object_type: ObjectType | None = None):
    if object_type is None:
        return PROPERTY_STRATEGY
    else:
        object_cls = BUILTIN_OBJECT_CLASS_BY_TYPE[object_type]
        return st.sampled_from(
            tuple(p for p in object_cls.__declared_properties__.values() if p.id is not None)
        )


@cacheable
@defines_strategy()
def from_type_info_scalar(typ: TypeBase) -> st.SearchStrategy[Any]:
    """Turns a type into a strategy for a scalar. Considers constraints. See check_value_scalar."""
    constraint = typ.constraint or TypeConstraint()
    if typ.kind == TypeKind.PRIMITIVE:
        # apply constraints to primitive types
        assert typ.primitive_type is not None, f"{typ!r} has no primitive type"
        if typ.primitive_type in (PrimitiveType.INT16, PrimitiveType.INT32, PrimitiveType.INT64):
            min_value = (
                int(constraint.min_value)
                if constraint.min_value is not None
                else cast(int, MIN_VALUE_BY_PRIMITIVE_TYPE[typ.primitive_type])
            )
            max_value = (
                int(constraint.max_value)
                if constraint.max_value is not None
                else cast(int, MAX_VALUE_BY_PRIMITIVE_TYPE[typ.primitive_type])
            )
            return st.integers(min_value=min_value, max_value=max_value)
        elif typ.primitive_type in (PrimitiveType.FLOAT32, PrimitiveType.FLOAT64):
            min_value = (
                (constraint.min_value)
                if constraint.min_value is not None
                else cast(float, MIN_VALUE_BY_PRIMITIVE_TYPE[typ.primitive_type])
            )
            max_value = (
                (constraint.max_value)
                if constraint.max_value is not None
                else cast(float, MAX_VALUE_BY_PRIMITIVE_TYPE[typ.primitive_type])
            )
            return st.floats(
                min_value=min_value,
                max_value=max_value,
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
                return st.text(min_size=constraint.min_length or 1, max_size=constraint.max_length)
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
def from_type_info(typ: TypeBase) -> st.SearchStrategy[Any]:
    value_st = from_type_info_scalar(typ)
    constraint = typ.constraint or TypeConstraint()
    return wrap_value_scalar(
        value_st,
        is_required=typ.is_required,
        is_list=typ.is_list,
        min_length=constraint.min_length or 0,
        max_length=constraint.max_length or None,
    )


@cached({})
def get_naive_object_strategy(object_type: ObjectType):
    """Gets the default uncorrelated strategies for every (init) property of an object type."""
    object_cls = BUILTIN_OBJECT_CLASS_BY_TYPE[object_type]
    object_kwargs: dict[str, st.SearchStrategy] = {}
    for prop in object_cls.__runtime_properties__.values():
        if (
            # ignore runtime-only properties
            prop.id is None
            # ignore identity/tracking properties
            or (prop.id < 30 and prop.reference_kind is not None)
            # ignore contributed wired properties (they're derived from the generated one)
            or (prop.reference_source is not None)
            # ignore autoset properties (ids, timestamps)
            or prop.is_autoset
            # ignore node data properties
            or prop.reference_is_node_data
        ):
            continue  # :IgnoredGeneratedProperties
        elif (object_type, prop.name) in STRATEGY_BY_OBJECT_PROPERTY:
            object_kwargs[prop.name] = STRATEGY_BY_OBJECT_PROPERTY[object_type, prop.name]
        elif prop.name in STRATEGY_BY_PROPERTY:
            object_kwargs[prop.name] = STRATEGY_BY_PROPERTY[prop.name]
        elif prop.is_node_reference:
            if not prop.reference_nodes:
                continue  # nothing to do
            # generate random reference instead of node (sometimes this is enough)
            assert prop.reference_wired_ptr is not None, f"{prop!r} has no wired ptr"
            reference_nodes = (
                prop.reference_nodes if prop.reference_nodes != "any" else NODE_TYPES.tuple
            )
            object_kwargs[prop.reference_wired_ptr.name] = wrap_value_scalar(
                node_references(st.sampled_from(reference_nodes)),
                is_required=prop.is_required,
                is_list=prop.is_list,
            )
        elif prop.is_property_reference:
            object_kwargs[prop.name] = wrap_value_scalar(
                properties(), is_required=prop.is_required, is_list=prop.is_list
            )
        elif prop.reference_is_node_data:
            object_kwargs[prop.name] = st.none()  # nothing meaningful to generate?
        elif prop.is_value_runtime or prop.is_value_packed:
            object_kwargs[prop.name] = st.none()  # TODO :Test :Incomplete: add strategy for Values
        else:
            object_kwargs[prop.name] = from_type_info(prop.type_info)
    return object_kwargs


@cacheable
@defines_strategy()
def from_object_type(
    object_type: ObjectType,
    /,
    reject_invalid: bool = True,
    **custom_strategies: st.SearchStrategy,
) -> st.SearchStrategy[BuiltinObject]:
    # special case some types
    if object_type == StructType.TYPE:
        return cast(st.SearchStrategy[BuiltinObject], type_infos(SIMPLE_TYPE_KINDS))
    elif object_type == StructType.PROPERTY_REFERENCE:
        return cast(
            st.SearchStrategy[BuiltinObject], properties(object_type=None).map(lambda p: p.to_ref())
        )
    elif object_type == NodeType.FIELD:
        return cast(st.SearchStrategy[BuiltinObject], fields(SIMPLE_TYPE_KINDS))
    elif object_type == StructType.ICON:
        return cast(st.SearchStrategy[BuiltinObject], icons())

    object_cls = BUILTIN_OBJECT_CLASS_BY_TYPE[object_type]
    object_dict = get_naive_object_strategy(object_type)
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
    if node_cls.__is_in_bench__:
        bench_id = draw(STRATEGY_BY_PRIMITIVE_TYPE[PrimitiveType.UUID])
    else:
        bench_id = None
    return NodeReference(node_type=node_type, id=node_id, ck=node_ck, bench_id=bench_id)


STRATEGY_BY_PRIMITIVE_TYPE: dict[PrimitiveType, st.SearchStrategy] = {
    PrimitiveType.BOOLEAN: st.booleans(),
    PrimitiveType.INT16: st.integers(min_value=-(2**15), max_value=2**15 - 1),
    PrimitiveType.INT32: st.integers(min_value=-(2**31), max_value=2**31 - 1),
    PrimitiveType.INT64: st.integers(min_value=-(2**63), max_value=2**63 - 1),
    PrimitiveType.DECIMAL: st.decimals(),
    PrimitiveType.FLOAT32: st.floats(allow_nan=False, allow_infinity=False),
    PrimitiveType.FLOAT64: st.floats(allow_nan=False, allow_infinity=False),
    PrimitiveType.STRING: st.text(min_size=1),
    PrimitiveType.UUID: st.uuids(),
    PrimitiveType.JSON: JSON_STRATEGY,
    PrimitiveType.BYTES: st.binary(),
    PrimitiveType.DATETIME: st.datetimes(timezones=st.just(pytz.utc)),
    PrimitiveType.DURATION: st.timedeltas(
        min_value=MIN_VALUE_BY_PRIMITIVE_TYPE[PrimitiveType.DURATION],
        max_value=MAX_VALUE_BY_PRIMITIVE_TYPE[PrimitiveType.DURATION],
    ),
}
STRATEGY_BY_PROPERTY: dict[str, st.SearchStrategy] = {
    "order_key": ORDER_KEY_STRATEGY,
    "name": NAME_STRATEGY,
}
STRATEGY_BY_OBJECT_PROPERTY: dict[tuple[ObjectType, str], st.SearchStrategy] = {
    (NodeType.FILE, "inline_content"): BYTES_STRATEGY,
    (StructType.FILE_INFO, "inline_content"): BYTES_STRATEGY,
    (StructType.CODE, "lines"): st.lists(from_object_type(StructType.CODE_LINE), min_size=0),
    (StructType.CODE_LINE, "content"): st.text(min_size=1, max_size=64, alphabet=ascii_lowercase),
    # Text is pretty limited right now :CrummyMarkdown
    (StructType.TEXT, "lines"): st.lists(from_object_type(StructType.TEXT_LINE), max_size=0),
    (StructType.TEXT_LINE, "spans"): st.lists(
        from_object_type(StructType.TEXT_SPAN), min_size=1, max_size=1
    ),
    (StructType.TEXT_SPAN, "content"): st.text(min_size=1, max_size=64, alphabet=ascii_lowercase),
    **{
        (s, p): st.just(None)
        for p in ("color", "is_bold", "is_italic", "is_strikethrough", "is_underline", "is_code")
        for s in (StructType.TEXT_LINE, StructType.TEXT_SPAN)
    },
}

SIMPLE_TYPE_KINDS = st.sampled_from((TypeKind.PRIMITIVE, TypeKind.ENUM, TypeKind.STRUCT))


def draw_type_info_base_dict(draw: st.DrawFn, kinds: st.SearchStrategy[TypeKind]) -> dict[str, Any]:
    kind = draw(kinds)
    primitive_type = None
    bench_type = None
    base_type = None
    if kind == TypeKind.PRIMITIVE:
        primitive_type = draw(PRIMITIVE_TYPE_STRATEGY)
    elif kind == TypeKind.ENUM:
        bench_type = draw(ENUM_TYPE_STRATEGY)
    elif kind == TypeKind.STRUCT:
        bench_type = draw(STRUCT_TYPE_STRATEGY)
    else:
        raise NotImplementedError(f"TypeKind {kind!r} not implemented")
    return {
        "kind": kind,
        "primitive_type": primitive_type,
        "bench_type": bench_type,
        "base_type": base_type,
        "base_field_types": [],  # NOTE :Incomplete: base_field_type is not rendered properly
        "property_field_types": [],
        "format": None,
    }


@cacheable
@st.composite
def type_infos(draw: st.DrawFn, kinds: st.SearchStrategy[TypeKind]):
    base_dict = draw_type_info_base_dict(draw, kinds)
    return Type(**base_dict)


@st.composite
def fields(draw: st.DrawFn, kinds: st.SearchStrategy[TypeKind]):
    type_info_base_dict = draw_type_info_base_dict(draw, kinds)
    naive_base_dict = get_naive_object_strategy(NodeType.FIELD)
    naive_base_dict["type"] = st.just(FieldType.RESOURCE)
    combined_dict = {}
    for key in naive_base_dict:
        # prefer type info where set
        if key in type_info_base_dict:
            combined_dict[key] = type_info_base_dict[key]
        else:
            combined_dict[key] = draw(naive_base_dict[key])
    combined_dict["is_list"] = False
    return Field(**combined_dict)


@st.composite
def icons(draw: st.DrawFn):
    kind = draw(st.sampled_from(list(IconType)))
    if kind == IconType.EMOJI:
        return Icon.new("😀")
    elif kind == IconType.FONT_AWESOME:
        return Icon.new("fas fa-circle-dot")
    elif kind == IconType.VS_CODE:
        return Icon(type=IconType.VS_CODE, fa_name="file_type_ada")
    else:
        assert_never(kind)


@cacheable
@st.composite
def builtin_objects(
    draw: st.DrawFn, object_types: st.SearchStrategy[ObjectType] = OBJECT_TYPE_STRATEGY
):
    object_type = draw(object_types)
    return draw(from_object_type(object_type, reject_invalid=True))


nodes = builtin_objects(object_types=st.sampled_from(NodeType))
structs = builtin_objects(object_types=st.sampled_from(StructType))
