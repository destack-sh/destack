import base64
from datetime import date, datetime, time, timedelta
from typing import (
    TYPE_CHECKING,
    Any,
    Collection,
    Union,
    cast,
)

import pytz
import structlog
from fastuuid import UUID
from google.protobuf.duration_pb2 import Duration
from google.protobuf.json_format import MessageToDict
from google.protobuf.struct_pb2 import NULL_VALUE as PROTO_NULL_VALUE
from google.protobuf.struct_pb2 import ListValue as ProtoList
from google.protobuf.struct_pb2 import Struct as ProtoStruct
from google.protobuf.struct_pb2 import Value as ProtoValue
from google.protobuf.timestamp_pb2 import Timestamp
from opentelemetry import trace

from bench.language.registry import (
    BENCH_TYPE_BY_CLASS,
    BUILTIN_OBJECT_CLASS_BY_TYPE,
    ENUM_CLASS_BY_TYPE,
)
from bench.pb2 import AnyNodeData, AnyStructData, Date, NodeReferenceData, TimeOfDay
from bench.utils.time import timedelta_from_isoformat, timedelta_to_isoformat

from .const import (
    EnumType,
    ObjectType,
    PrimitiveType,
    PrimitiveValue,
    StructType,
)
from .graph import Supergraph
from .property import Property, p_regular
from .struct import Struct, struct_
from .type import TypeType

if TYPE_CHECKING:
    from bench.language import BuiltinObject, Node, NodeReference, Session, TypeBase


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

    value: dict[str, Any] | None = p_regular(35)


def pack_value_scalar(value: ScalarValue | ScalarValueData, typ: "TypeBase") -> JsonValue:
    """
    Packs the given scalar runtime or data value into a JSON-able representation.
    """
    if typ.kind == TypeType.PRIMITIVE:
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
    elif typ.kind == TypeType.NODE:
        if cast("Struct | AnyStructData", value).metatype != StructType.NODE_REFERENCE:
            ref = cast("Node", value).to_ref()
        else:
            ref = cast("NodeReference | NodeReferenceData", value)
        if isinstance(ref, BuiltinObject):
            ref = ref._to_data()
        return pack_builtin_object_data(ref)
    elif typ.kind == TypeType.ENUM:
        return int(cast(Any, value))
    elif typ.kind == TypeType.STRUCT:
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
    value_packed: JsonValue, typ: "TypeBase", *, supergraph: Supergraph | None
) -> ScalarValue:
    """
    Unpacks the given scalar value into its runtime representation.
    """
    if typ.kind == TypeType.PRIMITIVE:
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
    elif typ.kind == TypeType.ENUM:
        enum_cls = ENUM_CLASS_BY_TYPE[cast(EnumType, typ.enum_type)]
        return enum_cls(cast(int, value_packed))
    elif typ.kind in (TypeType.NODE, TypeType.STRUCT):
        assert isinstance(value_packed, dict), f"{value_packed!r} is not a dict, expected {typ!r}"
        return unpack_builtin_object(value_packed, supergraph=supergraph)
    else:
        raise TypeError(f"cannot unpack value of type {typ!r}")


def unpack_value_scalar_data(value_packed: JsonValue, typ: "TypeBase") -> ScalarValueData:
    """
    Unpacks the given scalar value into its proto data representation. See above.
    """
    if typ.kind == TypeType.PRIMITIVE:
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
    elif typ.kind == TypeType.ENUM:
        enum_cls = ENUM_CLASS_BY_TYPE[cast(EnumType, typ.enum_type)]
        return enum_cls(cast(int, value_packed))
    elif typ.kind in (TypeType.NODE, TypeType.STRUCT):
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
    for prop in only if only is not None else object_cls.__wired_properties__.values():
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
    supergraph: Supergraph | None,
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
    for prop in object_cls.__wired_properties__.values():
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
    for prop in only if only is not None else builtin_object_cls.__wired_properties__.values():
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
    for prop in object_cls.__wired_properties__.values():
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


def pack_value(value: SomeValue | None, typ: "TypeBase", *, wrap: bool = False) -> JsonValue:
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


def pack_value_data(value: SomeValueData, typ: "TypeBase", wrap: bool = False) -> JsonValue:
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
    typ: "TypeBase",
    *,
    supergraph: Supergraph | None = None,
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
    value_packed: JsonValue, typ: "TypeBase", wrap: bool = False
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
