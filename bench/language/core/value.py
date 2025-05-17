import base64
import dataclasses
from collections.abc import Mapping
from datetime import date, datetime, time, timedelta
from decimal import Decimal
from typing import (
    TYPE_CHECKING,
    Any,
    Callable,
    Collection,
    Literal,
    NamedTuple,
    Sequence,
    TypeGuard,
    Union,
    assert_never,
    cast,
)
from uuid import UUID

import pytz
import regex
import structlog
from google.protobuf.duration_pb2 import Duration
from google.protobuf.json_format import MessageToDict
from google.protobuf.struct_pb2 import NULL_VALUE as PROTO_NULL_VALUE
from google.protobuf.struct_pb2 import ListValue as ProtoList
from google.protobuf.struct_pb2 import Struct as ProtoStruct
from google.protobuf.struct_pb2 import Value as ProtoValue
from google.protobuf.timestamp_pb2 import Timestamp
from opentelemetry import trace

from bench.language.registry import (
    BENCH_CLASS_BY_TYPE,
    BENCH_TYPE_BY_CLASS,
    BUILTIN_OBJECT_CLASS_BY_TYPE,
    ENUM_CLASS_BY_TYPE,
    _on_completing_setup,
)
from bench.pb2 import AnyNodeData, AnyStructData, Date, NodeReferenceData, TimeOfDay
from bench.utils.time import timedelta_from_isoformat, timedelta_to_isoformat

from .const import (
    EMPTY_DICT,
    FLOAT_EPSILON,
    PY_TYPE_BY_PRIMITIVE_TYPE,
    EnumType,
    ObjectType,
    PrimitiveType,
    PrimitiveValue,
    StructType,
    TypeKind,
)
from .graph import NodeSuperGraph
from .property import Property
from .struct import Struct, struct_
from .validation import TYPE_CONSTRAINT_BY_FORMAT, on_invalid_raise

if TYPE_CHECKING:
    from bench.language import (
        BuiltinObject,
        Node,
        NodeReference,
        Session,
        TypeBase,
        TypeConstraint,
        TypeConstraintIn,
        TypeIdentity,
    )

    from .validation import ValidationHandler


# pyright: reportIncompatibleVariableOverride=false


logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

ScalarValue = Union[PrimitiveValue, "BuiltinObject"]
ScalarValueData = Union[
    AnyNodeData,
    AnyStructData,
    PrimitiveValue,
    dict[str, "ScalarValueData"],
    list["ScalarValueData"],
    ProtoStruct,
    ProtoValue,
    Timestamp,
    Date,
    TimeOfDay,
    Duration,
]
SomeValue = Union[ScalarValue, Collection[ScalarValue], None]
SomeValueData = Union[ScalarValueData, Collection[ScalarValueData], None]
JsonPrimitive = Union[str, int, float, bool, None]
JsonValue = Union[JsonPrimitive, dict[str, "JsonValue"], list["JsonValue"]]


@struct_(StructType.VALUE)
class Value(Struct):
    """A value of any type."""

    value: SomeValue


#
# Type checking :TypeChecking
#

# bounds checking
MIN_VALUE_BY_PRIMITIVE_TYPE: dict[PrimitiveType, Any] = {
    PrimitiveType.INT16: -(2**15),
    PrimitiveType.INT32: -(2**31),
    PrimitiveType.INT64: -(2**63),
    PrimitiveType.FLOAT32: -3.4028235e38,
    PrimitiveType.FLOAT64: -1.7976931348623157e308,
    PrimitiveType.DURATION: timedelta(days=-(1000 * 365)),
}
MAX_VALUE_BY_PRIMITIVE_TYPE: dict[PrimitiveType, Any] = {
    PrimitiveType.INT16: 2**15 - 1,
    PrimitiveType.INT32: 2**31 - 1,
    PrimitiveType.INT64: 2**63 - 1,
    PrimitiveType.FLOAT32: 3.4028235e38,
    PrimitiveType.FLOAT64: 1.7976931348623157e308,
    PrimitiveType.DURATION: timedelta(days=(1000 * 365)),
}


class CheckOptions(NamedTuple):
    """Options for checking a value."""

    detached_is: Literal["none", "invalid"] = "none"


DEFAULT_CHECK_OPTIONS = CheckOptions()


def check_value_scalar_constraint(
    value: SomeValue,
    typ: "TypeBase | TypeIdentity",
    constraint: "TypeConstraint | TypeConstraintIn",
    *,
    options: CheckOptions = DEFAULT_CHECK_OPTIONS,
    invalid: "ValidationHandler" = on_invalid_raise,
) -> None:
    """Checks whether the given value satisfies the given scalar constraint."""  # :TypeChecking
    if type(value) is int or type(value) is float:
        if constraint.min_value is not None and value < constraint.min_value:
            invalid(value, "too small", typ)
        if constraint.max_value is not None and value > constraint.max_value:
            invalid(value, "too large", typ)
        if constraint.step_value is not None:
            delta = abs(value % constraint.step_value)
            if delta > FLOAT_EPSILON and abs(delta - constraint.step_value) > FLOAT_EPSILON:
                invalid(value, f"not a multiple of {constraint.step_value}", typ)
    elif type(value) is str:
        if constraint.min_length is not None and len(value) < constraint.min_length:
            invalid(value, "too short", typ)
        if constraint.max_length is not None and len(value) > constraint.max_length:
            invalid(value, "too long", typ)
        if constraint.regex is not None and not regex.match(constraint.regex, value):
            invalid(value, "does not match regex", typ)
        if constraint.starts_with is not None and not value.startswith(constraint.starts_with):
            invalid(value, f"does not start with {constraint.starts_with}", typ)
        if constraint.ends_with is not None and not value.endswith(constraint.ends_with):
            invalid(value, f"does not end with {constraint.ends_with}", typ)


def check_value_scalar(
    value: SomeValue,
    typ: "TypeBase | TypeIdentity",
    *,
    options: CheckOptions = DEFAULT_CHECK_OPTIONS,
    invalid: "ValidationHandler" = on_invalid_raise,
) -> None:
    """Checks whether the given scalar value has the expected type."""  # :TypeChecking
    if typ.kind == TypeKind.PRIMITIVE:
        expected_type = PY_TYPE_BY_PRIMITIVE_TYPE.get(cast(PrimitiveType, typ.primitive_type))
        if expected_type is None:
            return  # nothing to check?
        elif type(value) is not expected_type and not isinstance(value, expected_type):
            invalid(value, f"not of type, is '{type(value).__name__}'", typ)
            return  # also nothing to do
        # check constraint
        if typ.constraint is not None:
            check_value_scalar_constraint(
                value, typ, typ.constraint, options=options, invalid=invalid
            )
        if typ.format is not None:
            check_value_scalar_constraint(
                value, typ, TYPE_CONSTRAINT_BY_FORMAT[typ.format], options=options, invalid=invalid
            )
        # required strings cannot be empty (because of protobuf we must disambiguate unset from empty)
        if type(value) is str and len(value) == 0 and typ.is_required:
            invalid(value, "empty string", typ)
        # check bounds
        min_value = MIN_VALUE_BY_PRIMITIVE_TYPE.get(cast(PrimitiveType, typ.primitive_type))
        max_value = MAX_VALUE_BY_PRIMITIVE_TYPE.get(cast(PrimitiveType, typ.primitive_type))
        if min_value is not None and value < min_value:
            invalid(value, "too small", typ)
        if max_value is not None and value > max_value:
            invalid(value, "too large", typ)
    elif typ.kind == TypeKind.NODE:
        from .node import Node, NodeReference

        if typ.bench_type is not None:
            allowed_types = (typ.bench_type,)
        elif typ.constraint is not None and typ.constraint.node_types:
            allowed_types = typ.constraint.node_types
        else:
            allowed_types = None

        if isinstance(value, Node):
            if options.detached_is == "invalid" and not value.is_attached:
                invalid(value, "detached node", typ)
            elif allowed_types and value.metatype not in allowed_types:
                invalid(value, f"not of type, is {value.metatype}", typ)
        elif isinstance(value, NodeReference):
            if allowed_types and value.node_type not in allowed_types:
                invalid(value, f"not of type, is {value.node_type}", typ)
        else:
            invalid(value, "not a Node", typ)
    elif typ.kind == TypeKind.STRUCT:
        if typ.bench_type == StructType.PROPERTY_REFERENCE:
            if not isinstance(value, Property):
                invalid(value, "not a Property", typ)
        else:
            if not getattr(cast("Struct", value), "__is_struct__", False):
                invalid(value, "not a Struct", typ)
            elif cast("Struct", value).metatype != typ.bench_type:
                invalid(value, f"not of type, is {cast('Struct', value).metatype}", typ)
    elif typ.kind == TypeKind.ENUM:
        enum_cls = ENUM_CLASS_BY_TYPE[cast(EnumType, typ.bench_type)]
        try:
            enum_cls(cast(int, value))
        except ValueError:
            invalid(value, f"not a valid {enum_cls}", typ)
    else:
        raise RuntimeError(f"unexpected type {typ!r}")


def _check_is_list(
    value: SomeValue,
    typ: "TypeBase | TypeIdentity",
    *,
    options: CheckOptions = DEFAULT_CHECK_OPTIONS,
    invalid: "ValidationHandler" = on_invalid_raise,
) -> TypeGuard[list]:
    """Checks whether the given value is a list of the expected dimensions."""
    if not isinstance(value, (list, tuple)):
        invalid(value, "not a list", typ)
        return False
    if typ.constraint is not None:
        if typ.constraint.min_length is not None and len(value) < typ.constraint.min_length:
            invalid(value, "too short", typ)
        if typ.constraint.max_length is not None and len(value) > typ.constraint.max_length:
            invalid(value, "too long", typ)
    return True


def check_value(
    value: Any,
    typ: "TypeBase",
    *,
    options: CheckOptions = DEFAULT_CHECK_OPTIONS,
    invalid: "ValidationHandler" = on_invalid_raise,
) -> None:
    """
    Checks whether the given value has the expected type (recursively).
    """
    if value is None:
        if typ.is_required:
            invalid(value, "missing required value", typ)
        else:
            return
    if not typ.is_list:
        check_value_scalar(value, typ, options=options, invalid=invalid)
    elif _check_is_list(value, typ, options=options, invalid=invalid):
        for element in value:
            check_value_scalar(element, typ, options=options, invalid=invalid)


def is_value(value: Any, typ: "TypeBase", options: CheckOptions = DEFAULT_CHECK_OPTIONS) -> bool:
    """Checks whether the given value has the expected type (recursively)."""
    # NOTE :Performance: make is_value more efficient (don't just use check_value?)
    try:
        check_value(value, typ, options=options, invalid=on_invalid_raise)
        return True
    except Exception:
        return False


#
# Coercion
#


@dataclasses.dataclass(slots=True)
class CoercionRule:
    """A coercer for a specific target type."""

    default: Callable[[Any], Any] | None = None
    by_source: dict[type, Callable[[Any], Any]] = dataclasses.field(default_factory=dict)


COERCION_RULE_BY_TYPE: dict[type, CoercionRule] = {}


def register_coercion[T](
    typ: type[T],
    coerce_from: Mapping[type, Callable[[Any], T]] = EMPTY_DICT,
    coerce_to: Mapping[type, Callable[[T], Any]] = EMPTY_DICT,
    *,
    default: Callable[[Any], T] | None = None,
):
    """Register a coercion rule for a specific target type."""
    # add coercion rule for target type
    if typ in COERCION_RULE_BY_TYPE:
        raise ValueError(f"coercion rule already registered for {typ!r}")
    COERCION_RULE_BY_TYPE[typ] = CoercionRule(default=default)

    # source type to target type
    for source_type, coercer in coerce_from.items():
        COERCION_RULE_BY_TYPE[typ].by_source[source_type] = coercer

    # target type to other types
    for target_type, coercer in coerce_to.items():
        if (coercion_rule := COERCION_RULE_BY_TYPE.get(target_type)) is None:
            coercion_rule = CoercionRule()
            COERCION_RULE_BY_TYPE[target_type] = coercion_rule
        coercion_rule.by_source[typ] = coercer


# primitive coercions
register_coercion(bool)
register_coercion(int, {float: int})
register_coercion(Decimal)
register_coercion(float, {int: float})
register_coercion(str, {bytes: str, int: str, float: str, UUID: str, bool: str})
register_coercion(bytes, {str: bytes})
register_coercion(UUID)
register_coercion(datetime, {str: datetime.fromisoformat}, {str: datetime.isoformat})
register_coercion(date, {str: date.fromisoformat}, {str: date.isoformat})


@_on_completing_setup
def _register_other_coercions():
    from bench.language.core import Code, Text

    register_coercion(Code, coerce_from={str: Code.from_string}, coerce_to={str: Code.to_string})
    register_coercion(
        Text,
        coerce_from={str: Text.from_markdown, Code: Text.code},
        coerce_to={str: Text.to_markdown},
    )


def _do_coerce(
    value: ScalarValue, typ: "TypeBase | TypeIdentity", source_type: type, target_type: type
) -> ScalarValue:
    """Apply coercion rules to get from source type to target type."""
    coercion = COERCION_RULE_BY_TYPE.get(target_type)
    coercer = (
        coercion.by_source.get(source_type, coercion.default) if coercion is not None else None
    )
    if coercer is None:
        raise ValueError(
            f"{value!r} ({source_type.__name__}) is not a {target_type.__name__}, expected {typ!r}"
        )
    try:
        return coercer(value)
    except Exception as e:
        raise ValueError(
            f"{value!r} ({source_type.__name__}) is not a {target_type.__name__}, expected {typ!r}"
        ) from e


def coerce_value_scalar(
    value: ScalarValue, typ: "TypeBase | TypeIdentity", *, as_packed: bool = False
) -> ScalarValue:
    """Coerces a scalar value (primitive, node, struct)"""
    # apply coercion/check rules
    source_type = type(value)
    if typ.kind == TypeKind.PRIMITIVE:
        if typ.primitive_type == PrimitiveType.JSON:
            return value  # as is
        assert typ.primitive_type is not None, f"missing primitive type for {typ!r}"
        target_type = PY_TYPE_BY_PRIMITIVE_TYPE.get(typ.primitive_type)
        assert target_type is not None, f"missing target type for {typ!r}"
        if source_type is not target_type and not isinstance(value, target_type):
            value = _do_coerce(value, typ, source_type, target_type)
        return value
    elif typ.kind == TypeKind.STRUCT:
        assert typ.bench_type is not None, f"missing bench type for {typ!r}"
        target_type = BENCH_CLASS_BY_TYPE[typ.bench_type]
        if source_type is not target_type:
            if source_type == Property and typ.bench_type == StructType.PROPERTY_REFERENCE:
                return value  # Property is unpacked PropertyReference
            value = _do_coerce(value, typ, source_type, target_type)
        return value
    elif typ.kind == TypeKind.ENUM:
        assert typ.bench_type is not None, f"missing bench type for {typ!r}"
        target_type = BENCH_CLASS_BY_TYPE[typ.bench_type]
        if source_type is int or source_type is not target_type:
            try:
                value = target_type(value)  # type: ignore
            except ValueError as e:
                raise ValueError(
                    f"{value!r} ({source_type.__name__}) is not a valid {target_type.__name__}, expected {typ!r}"
                ) from e
        return value
    elif typ.kind == TypeKind.NODE:
        if (
            not getattr(type(value), "__is_node__", False)
            and getattr(type(value), "metatype", None) != StructType.NODE_REFERENCE
        ):
            raise TypeError(f"expected Node, got {value!r}")
        if as_packed:
            value = cast("Node", value).to_ref()
        return value
    else:
        assert_never(typ.kind)


def coerce_value(
    value: Any,
    typ: "TypeBase | TypeIdentity",
    *,
    as_packed: bool = False,
    supergraph: NodeSuperGraph | None = None,
) -> SomeValue:
    """
    Coerces the given value to the expected type (recursively).
    Returns value as is if already of correct type, raises TypeError if coercion is not possible.
    NOTE :Performance: we re-create and copy lists during coercion even if the type was already good
    """
    if value is None:
        return None
    elif not typ.is_list:
        if isinstance(value, (list, tuple)):
            raise TypeError(f"{value!r} ({type(value).__name__}) is a sequence, expected {typ!r}")
        return coerce_value_scalar(value, typ, as_packed=as_packed)
    else:
        if not isinstance(value, Sequence):
            raise TypeError(
                f"{value!r} ({type(value).__name__}) is not a sequence, expected {typ!r}"
            )
        return [coerce_value_scalar(element, typ, as_packed=as_packed) for element in value]


#
# Value packing
#


def pack_value_scalar(
    value: ScalarValue | ScalarValueData, typ: "TypeBase | TypeIdentity"
) -> JsonValue:
    """
    Packs the given scalar runtime or data value into a JSON-able representation.
    """
    if typ.kind == TypeKind.PRIMITIVE:
        if typ.primitive_type == PrimitiveType.BYTES:
            return base64.b64encode(cast(bytes, value)).decode()
        elif typ.primitive_type == PrimitiveType.UUID:
            return str(cast(UUID, value))
        elif typ.primitive_type == PrimitiveType.JSON:
            if type(value) is ProtoValue:
                return cast(JsonValue, unpack_proto_json_struct(value))
            elif type(value) is ProtoStruct:
                return cast(JsonValue, MessageToDict(value))
            else:
                return cast(JsonValue, value)
        elif typ.primitive_type == PrimitiveType.DATETIME:
            if type(value) is Timestamp:
                return value.ToDatetime(tzinfo=pytz.utc).isoformat()
            else:
                return cast(datetime, value).isoformat()
        elif typ.primitive_type == PrimitiveType.DATE:
            if type(value) is Date:
                return unpack_proto_date(value).isoformat()
            else:
                return cast(date, value).isoformat()
        elif typ.primitive_type == PrimitiveType.TIME:
            if type(value) is TimeOfDay:
                return unpack_proto_time(value).isoformat()
            else:
                return cast(time, value).isoformat()
        elif typ.primitive_type == PrimitiveType.DURATION:
            if type(value) is Duration:
                return timedelta_to_isoformat(value.ToTimedelta())
            else:
                return timedelta_to_isoformat(cast(timedelta, value))
        else:
            return cast(JsonValue, value)
    elif typ.kind == TypeKind.NODE:
        if cast("Struct | AnyStructData", value).metatype != StructType.NODE_REFERENCE:
            ref = cast("Node", value).to_ref()
        else:
            ref = cast("NodeReference | NodeReferenceData", value)
        if isinstance(ref, BuiltinObject):
            ref = ref._to_data()
        return pack_builtin_object_data(ref)
    elif typ.kind == TypeKind.ENUM:
        return int(cast(Any, value))
    elif typ.kind == TypeKind.STRUCT:
        if isinstance(value, BuiltinObject):
            return pack_builtin_object(value)
        else:
            assert hasattr(
                value, "metatype"
            ), f"unexpected value {value!r} ({type(value).__name__ }) for {typ!r}"
            return pack_builtin_object_data(cast(AnyStructData | AnyNodeData, value))
    else:
        raise TypeError(f"cannot pack value of type {typ!r}")


def unpack_value_scalar(
    value_packed: JsonValue, typ: "TypeBase | TypeIdentity", *, supergraph: NodeSuperGraph | None
) -> ScalarValue:
    """
    Unpacks the given scalar value into its runtime representation.
    """
    if typ.kind == TypeKind.PRIMITIVE:
        if typ.primitive_type == PrimitiveType.BYTES:
            return base64.b64decode(cast(str, value_packed))
        elif typ.primitive_type in (PrimitiveType.INT16, PrimitiveType.INT32, PrimitiveType.INT64):
            return int(cast(int, value_packed))
        elif typ.primitive_type == PrimitiveType.UUID:
            return UUID(cast(str, value_packed))
        elif typ.primitive_type == PrimitiveType.DATETIME:
            return datetime.fromisoformat(cast(str, value_packed))
        elif typ.primitive_type == PrimitiveType.DATE:
            return datetime.fromisoformat(cast(str, value_packed)).date()
        elif typ.primitive_type == PrimitiveType.TIME:
            return time.fromisoformat(cast(str, value_packed))
        elif typ.primitive_type == PrimitiveType.DURATION:
            return timedelta_from_isoformat(cast(str, value_packed))
        else:
            return cast(PrimitiveValue, value_packed)
    elif typ.kind == TypeKind.ENUM:
        enum_cls = ENUM_CLASS_BY_TYPE[cast(EnumType, typ.bench_type)]
        return enum_cls(cast(int, value_packed))
    elif typ.kind in (TypeKind.NODE, TypeKind.STRUCT):
        assert isinstance(value_packed, dict), f"{value_packed!r} is not a dict, expected {typ!r}"
        return unpack_builtin_object(value_packed, supergraph=supergraph)
    else:
        raise TypeError(f"cannot unpack value of type {typ!r}")


def unpack_value_scalar_data(
    value_packed: JsonValue, typ: "TypeBase | TypeIdentity"
) -> ScalarValueData:
    """
    Unpacks the given scalar value into its proto data representation. See above.
    """
    if typ.kind == TypeKind.PRIMITIVE:
        if typ.primitive_type == PrimitiveType.BYTES:
            return base64.b64decode(cast(str, value_packed))
        elif typ.primitive_type in (PrimitiveType.INT16, PrimitiveType.INT32, PrimitiveType.INT64):
            return int(cast(int, value_packed))
        elif typ.primitive_type == PrimitiveType.UUID:
            return cast(str, value_packed)  # leave as string
        elif typ.primitive_type == PrimitiveType.JSON:
            return pack_proto_json(cast(Any, value_packed))
        elif typ.primitive_type == PrimitiveType.DATETIME:
            ts = Timestamp()
            ts.FromDatetime(datetime.fromisoformat(cast(str, value_packed)))
            return ts
        elif typ.primitive_type == PrimitiveType.DATE:
            dt = date.fromisoformat(cast(str, value_packed))
            return pack_proto_date(dt)
        elif typ.primitive_type == PrimitiveType.TIME:
            dt = time.fromisoformat(cast(str, value_packed))
            return pack_proto_time(dt)
        elif typ.primitive_type == PrimitiveType.DURATION:
            dur = Duration()
            dur.FromTimedelta(timedelta_from_isoformat(cast(str, value_packed)))
            return dur
        else:
            return cast(PrimitiveValue, value_packed)
    elif typ.kind == TypeKind.ENUM:
        enum_cls = ENUM_CLASS_BY_TYPE[cast(EnumType, typ.bench_type)]
        return enum_cls(cast(int, value_packed))
    elif typ.kind in (TypeKind.NODE, TypeKind.STRUCT):
        assert isinstance(value_packed, dict), f"{value_packed!r} is not a dict, expected {typ!r}"
        return unpack_builtin_object_data(value_packed)
    else:
        raise TypeError(f"cannot unpack value of type {typ!r}")


def pack_builtin_object(
    value: "BuiltinObject", only: Collection[Property] | None = None
) -> dict[str, JsonValue]:
    """Packs a BuiltinObject into a JSON representation."""
    value_packed: dict[str, JsonValue] = {}
    object_cls = BUILTIN_OBJECT_CLASS_BY_TYPE[value.metatype]
    for prop in only if only is not None else object_cls.__proto_properties__.values():
        if prop.ptr_prop is not None:
            prop = prop.ptr_prop
        prop_name = prop.name
        prop_value = getattr(value, prop_name)
        if prop_value is None or (prop.is_list and len(prop_value) == 0):
            continue
        elif prop.is_list:
            prop_type = prop.type_info
            prop_value_packed = [pack_value_scalar(e, prop_type) for e in prop_value]
        else:
            prop_value_packed = pack_value_scalar(prop_value, prop.type_info)
        value_packed[prop.key] = prop_value_packed
    return value_packed


def unpack_builtin_object[T: BuiltinObject = BuiltinObject](
    value_packed: dict[str, Any],
    *,
    supergraph: NodeSuperGraph | None,
    expect: type[T] | None = None,
    session: "Session | None" = None,
) -> T:
    """Unpacks a BuiltinObject from a JSON representation."""

    if expect is None:
        object_type = value_packed.get("1")
        assert object_type is not None, f"{value_packed!r} has no object type and none given"
        object_type = cast(ObjectType, int(object_type))  # type: ignore
    else:
        object_type = BENCH_TYPE_BY_CLASS[expect]
    object_cls = BUILTIN_OBJECT_CLASS_BY_TYPE[cast(ObjectType, object_type)]
    assert not object_cls.__is_node__, f"cannot unpack {object_cls.__name__} from value"

    object_kwargs = {}
    for prop in object_cls.__proto_properties__.values():
        if prop.ptr_prop is not None:
            prop = prop.ptr_prop
        prop_value_packed = value_packed.get(prop.key)
        if prop_value_packed is None:
            continue
        elif prop.is_list:
            prop_type = prop.type_info
            object_kwargs[prop.name] = [
                unpack_value_scalar(e, prop_type, supergraph=supergraph) for e in prop_value_packed
            ]
        else:
            object_kwargs[prop.name] = unpack_value_scalar(
                prop_value_packed, prop.type_info, supergraph=supergraph
            )
    if session is not None:
        object_kwargs["_session"] = session

    obj = object_cls(**object_kwargs)
    return cast(T, obj)


def pack_builtin_object_data(
    value: AnyStructData | AnyNodeData,
    only: Collection[Property] | None = None,
) -> dict[str, JsonValue]:
    """Packs a single struct/node data value into a JSON representation."""
    value_packed: dict[str, JsonValue] = {}
    builtin_object_cls = BUILTIN_OBJECT_CLASS_BY_TYPE[value.metatype]  # type: ignore
    for prop in only if only is not None else builtin_object_cls.__proto_properties__.values():
        if prop.ptr_prop is not None:
            prop = prop.ptr_prop
        prop_name = prop.name
        if prop.is_optional_scalar and not value.HasField(prop_name):
            continue
        prop_value = getattr(value, prop_name)
        if prop_value is None or (prop.is_list and len(prop_value) == 0):
            continue
        elif prop.is_list:
            prop_type = prop.type_info
            prop_value_packed = [pack_value_scalar(element, prop_type) for element in prop_value]
        else:
            prop_value_packed = pack_value_scalar(prop_value, prop.type_info)
        value_packed[prop.key] = prop_value_packed
    return value_packed


def unpack_builtin_object_data[T: AnyStructData | AnyNodeData](
    value_packed: dict[str, Any],
    expect: type[T] | None = None,
    into: T | None = None,
) -> AnyStructData | AnyNodeData:
    """Unpacks a single struct/node data value from a JSON representation."""
    from bench.proto import wiring

    if expect is None:
        object_type = value_packed.get("1")
        assert object_type is not None, f"{value_packed!r} has no object type and none given"
        object_type = cast(ObjectType, int(object_type))
    else:
        object_type = wiring.OBJECT_TYPE_BY_PROTO_CLASS[expect]
    object_cls = BUILTIN_OBJECT_CLASS_BY_TYPE[object_type]
    proto_cls = wiring.PROTO_CLASS_BY_TYPE[object_type]

    value = into if into is not None else proto_cls(metatype=object_type)  # type: ignore
    for prop in object_cls.__proto_properties__.values():
        if prop.ptr_prop is not None:
            prop = prop.ptr_prop
        prop_value_packed = value_packed.get(prop.key)
        if prop_value_packed is None or (prop.is_list and len(prop_value_packed) == 0):
            continue
        elif prop.is_list:
            prop_type = prop.type_info
            prop_value = [
                unpack_value_scalar_data(element, prop_type) for element in prop_value_packed
            ]
        else:
            prop_value = unpack_value_scalar_data(prop_value_packed, prop.type_info)
        wiring.set_builtin_object_prop(value, prop, prop_value)
    return value


def pack_value(
    value: SomeValue | None, typ: "TypeBase | TypeIdentity", *, wrap: bool = False
) -> JsonValue:
    """
    Packs a value into a JSON representation.
    """

    # wrap scalar
    value_packed: JsonValue
    if value is None:
        value_packed = None  # no value
    elif not typ.is_list:
        value_packed = pack_value_scalar(cast(ScalarValue, value), typ)
    else:
        value_packed = [pack_value_scalar(element, typ) for element in cast(list, value)]
    if wrap:
        from bench.language.core import TypeBase

        assert isinstance(typ, TypeBase), f"expected full Type for {typ!r}"
        value_packed = {typ.identity_key: value_packed}
    return value_packed


def pack_value_data(
    value: SomeValueData, typ: "TypeBase | TypeIdentity", wrap: bool = False
) -> JsonValue:
    """Packs a data value into a JSON representation. See above."""
    # wrap scalar
    value_packed: JsonValue
    if value is None:
        value_packed = None
    elif not typ.is_list:
        value_packed = pack_value_scalar(cast(ScalarValueData, value), typ)
    else:
        value_packed = [pack_value_scalar(element, typ) for element in cast(list, value)]
    if wrap:
        from bench.language.core import TypeBase

        assert isinstance(typ, TypeBase), f"expected full Type for {typ!r}"
        value_packed = {typ.identity_key: value_packed}
    return value_packed


def unpack_value(
    value_packed: JsonValue,
    typ: "TypeBase | TypeIdentity",
    *,
    supergraph: NodeSuperGraph | None = None,
    wrap: bool = False,
) -> SomeValue | None:
    """
    Unpacks a value from its JSON representation.
    """

    # unwrap scalar
    if wrap and isinstance(value_packed, dict):
        from bench.language.core import TypeBase

        assert isinstance(typ, TypeBase), f"expected full Type for {typ!r}"
        value_packed = value_packed.get(typ.identity_key)
    if value_packed is None:
        return None
    elif not typ.is_list:
        return unpack_value_scalar(value_packed, typ, supergraph=supergraph)
    else:
        if not isinstance(value_packed, list):
            raise TypeError(f"{value_packed!r} is not a list, expected {typ!r}")
        return [
            unpack_value_scalar(element, typ, supergraph=supergraph) for element in value_packed
        ]


def unpack_value_data(
    value_packed: JsonValue, typ: "TypeBase | TypeIdentity", wrap: bool = False
) -> SomeValueData | JsonValue | None:
    """
    Unpacks a value from its JSON representation. Return nested objects as JSON (as is).
    """
    # scalar
    if wrap and isinstance(value_packed, dict):
        from bench.language.core import TypeBase

        assert isinstance(typ, TypeBase), f"expected full Type for {typ!r}"
        value_packed = value_packed.get(typ.identity_key)
    if value_packed is None:
        return None
    elif not typ.is_list:
        return unpack_value_scalar_data(value_packed, typ)
    else:
        if not isinstance(value_packed, list):
            raise TypeError(f"expected list for {typ!r}, got {value_packed!r}")
        return [unpack_value_scalar_data(v, typ) for v in value_packed]


#
# Common proto stuff
#

# NOTE :Performance: packing/unpacking proto JSON could probably be much more efficient


def pack_proto_date(value: date) -> Date:
    return Date(year=value.year, month=value.month, day=value.day)


def unpack_proto_date(value: Date) -> date:
    return date(year=value.year, month=value.month, day=value.day)


def pack_proto_time(value: time) -> TimeOfDay:
    return TimeOfDay(hours=value.hour, minutes=value.minute, seconds=value.second)


def unpack_proto_time(value: TimeOfDay) -> time:
    return time(hour=value.hours, minute=value.minutes, second=value.seconds)


def pack_proto_json_struct(value: dict[str, Any]) -> ProtoValue:
    struct = ProtoStruct()
    struct.update(value)
    proto_value = ProtoValue(struct_value=struct)
    return proto_value


def unpack_proto_json_struct(value: ProtoValue | ProtoStruct) -> dict[str, Any]:
    json = MessageToDict(value)
    return json


def pack_proto_json(value: JsonValue) -> ProtoValue:
    t = type(value)
    if value is None:
        return ProtoValue(null_value=PROTO_NULL_VALUE)
    elif t is bool:
        return ProtoValue(bool_value=value)  # type: ignore
    elif t is int or t is float:
        return ProtoValue(number_value=float(value))  # type: ignore
    elif t is str:
        return ProtoValue(string_value=value)  # type: ignore
    elif t is list:
        list_value = ProtoList(values=[pack_proto_json(item) for item in value])  # type: ignore
        return ProtoValue(list_value=list_value)
    elif t is dict:
        struct_value = ProtoStruct()
        struct_value.update(value)  # type: ignore
        return ProtoValue(struct_value=struct_value)
    elif t is ProtoList:
        return ProtoValue(list_value=value)  # type: ignore
    elif t is ProtoStruct:
        return ProtoValue(struct_value=value)  # type: ignore
    else:
        raise ValueError(f"unsupported JSON value {value} ({type(value)!r})")


def unpack_proto_json(value: ProtoValue) -> JsonValue:
    if value.HasField("bool_value"):
        return value.bool_value
    elif value.HasField("number_value"):
        return value.number_value
    elif value.HasField("string_value"):
        return value.string_value
    elif value.HasField("list_value"):
        return [unpack_proto_json(item) for item in value.list_value.values]
    elif value.HasField("struct_value"):
        return MessageToDict(value.struct_value)
    else:
        return None


# import later to avoid circular imports (Object is used in node.py)
from .object import BuiltinObject, Struct  # noqa: E402
