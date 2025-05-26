import base64
import textwrap
from datetime import date, datetime, time, timedelta
from typing import (
    TYPE_CHECKING,
    Any,
    Collection,
    Union,
    assert_never,
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

from bench.language.registry import BUILTIN_OBJECT_TYPE_BY_CLASS, ENUM_CLASS_BY_TYPE
from bench.pb2 import AnyNodeData, AnyStructData, Date, NodeReferenceData, TimeOfDay, ValueData
from bench.utils.time import timedelta_from_isoformat, timedelta_to_isoformat

from .const import (
    EnumType,
    PrimitiveType,
    PrimitiveValue,
    StructType,
)
from .graph import Supergraph
from .property import Property, property_
from .struct import Struct, struct_
from .type import Json, ScalarType

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
JsonPrimitive = Union[str, int, float, bool, None]
JsonValue = Union[JsonPrimitive, dict[str, "JsonValue"], list["JsonValue"]]


@struct_(StructType.VALUE)
class Value(Struct[ValueData]):
    """A value of any type."""

    value: dict[str, Json] = property_(35)


def to_value(value: Any) -> Value:
    """Convert an arbitrary value to a Value."""
    raise NotImplementedError


def pack_value_scalar(value: ScalarValue | ScalarValueData, typ: "TypeBase") -> JsonValue:
    """
    Packs the given scalar runtime or data value into a JSON-able representation.
    """
    if typ.scalar_type == ScalarType.PRIMITIVE:
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
    elif typ.scalar_type == ScalarType.NODE:
        if cast("Struct | AnyStructData", value).metatype != StructType.NODE_REFERENCE:
            ref = cast("Node", value).to_ref()
        else:
            ref = cast("NodeReference | NodeReferenceData", value)
        if isinstance(ref, BuiltinObject):
            ref = ref.to_proto()
        return pack_builtin_object_data(ref)
    elif typ.scalar_type == ScalarType.ENUM:
        return int(cast(Any, value))
    elif typ.scalar_type == ScalarType.STRUCT:
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
    if typ.scalar_type == ScalarType.PRIMITIVE:
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
    elif typ.scalar_type == ScalarType.ENUM:
        enum_cls = ENUM_CLASS_BY_TYPE[cast(EnumType, typ.enum_type)]
        return enum_cls(cast(int, value_packed))
    elif typ.scalar_type in (ScalarType.NODE, ScalarType.STRUCT):
        assert isinstance(value_packed, dict), f"{value_packed!r} is not a dict, expected {typ!r}"
        return unpack_builtin_object(value_packed, supergraph=supergraph)
    else:
        raise TypeError(f"cannot unpack value of type {typ!r}")


def pack_builtin_object(
    value: "BuiltinObject", only: Collection[Property] | None = None
) -> dict[str, JsonValue]:
    """Packs a BuiltinObject into the JSON representation."""
    raise NotImplementedError


def unpack_builtin_object[T: BuiltinObject = BuiltinObject](
    value_packed: dict[str, Any],
    *,
    supergraph: Supergraph | None,
    expect: type[T] | None = None,
    session: "Session | None" = None,
) -> T:
    """Unpacks a BuiltinObject from the JSON representation."""
    raise NotImplementedError


def pack_builtin_object_data(
    value: AnyStructData | AnyNodeData,
    only: Collection[Property] | None = None,
) -> dict[str, JsonValue]:
    """Packs a single struct/node data value into the JSON representation."""
    raise NotImplementedError


def unpack_builtin_object_data[T: AnyStructData | AnyNodeData](
    value_packed: dict[str, Any],
    expect: type[T] | None = None,
    into: T | None = None,
) -> AnyStructData | AnyNodeData:
    """Unpacks a single struct/node data value from the JSON representation."""
    raise NotImplementedError


def generate_pack_value_impl(cls: type["BuiltinObject"]) -> tuple[str, dict[str, Any]]:
    """Generate the BuiltinObject.__pack_value__/__unpack_value__ method implementations."""
    import textwrap

    pack_value = textwrap.indent(_generate_pack_value(cls), "    ")
    unpack_value = textwrap.indent(_generate_unpack_value(cls), "    ")

    value_impl = f"""
@classmethod
def __pack_value__(cls, object: "Self") -> dict[str, "JsonValue"]:
{pack_value}

@classmethod
def __unpack_value__(cls, object_value: dict[str, "JsonValue"]) -> "Self":
{unpack_value}

def to_value(self: "Self") -> dict[str, "JsonValue"]:
    return self.__pack_value__(self)

from_value = __unpack_value__
"""
    return value_impl, {
        "timedelta_from_isoformat": timedelta_from_isoformat,
        "timedelta_to_isoformat": timedelta_to_isoformat,
        "pack_proto_date": pack_proto_date,
        "unpack_proto_date": unpack_proto_date,
        "pack_proto_time": pack_proto_time,
        "unpack_proto_time": unpack_proto_time,
        "pack_proto_json": pack_proto_json,
        "unpack_proto_json": unpack_proto_json,
        "datetime": datetime,
        "timedelta": timedelta,
        "date": date,
        "time": time,
    }


def _generate_pack_value(cls: type["BuiltinObject"]) -> str:
    """Generate the BuiltinObject.__pack_value__ method implementation."""
    pack_method_parts: list[str] = []
    pack_method_parts.append("result = {}")

    wired_properties_in_order = list(cls.__wired_properties__.values())
    wired_properties_in_order.sort(key=lambda p: p.id or 0)
    for prop in wired_properties_in_order:
        if prop.name == "metatype":
            metatype = BUILTIN_OBJECT_TYPE_BY_CLASS[cls]
            pack_method_parts.append(f'result["{prop.id}"] = {metatype.value}')
            continue

        pack_code = _generate_pack_value_property(prop)
        if pack_code:
            pack_method_parts.extend(pack_code)
        else:
            # Simple assignment for properties that don't need special handling
            if prop.is_required:
                pack_method_parts.append(f'result["{prop.id}"] = object.{prop.name}')
            else:
                pack_method_parts.extend(
                    [
                        f"if object.{prop.name} is not None:",
                        f'    result["{prop.id}"] = object.{prop.name}',
                    ]
                )

    pack_method_parts.append("return result")
    return "\n".join(pack_method_parts)


def _generate_unpack_value(cls: type["BuiltinObject"]) -> str:
    """Generate the BuiltinObject.__unpack_value__ method implementation."""
    unpack_assignments: list[str] = []
    unpack_method_parts: list[str] = []

    for prop in cls.__wired_properties__.values():
        if prop.is_computed:
            continue  # set implicitly
        unpack_code = _generate_unpack_value_property(prop)
        if unpack_code:
            if len(unpack_code) == 1:
                unpack_assignments.append(f"{prop.name}={unpack_code[0].split(' = ', 1)[1]}")
            else:
                unpack_method_parts.extend(unpack_code)
                unpack_assignments.append(f"{prop.name}=_unpacked_{prop.name}")
        else:
            unpack_assignments.append(f'{prop.name}=object_value.get("{prop.id}")')

    unpack_method_parts.append("return cls(")
    for i, assignment in enumerate(unpack_assignments):
        comma = "," if i < len(unpack_assignments) - 1 else ""
        unpack_method_parts.append(f"    {assignment}{comma}")

    unpack_method_parts.append(")")

    return "\n".join(unpack_method_parts)


def _generate_pack_value_property(prop: "Property") -> list[str] | None:
    """Generate the packing code for a property value."""
    lines: list[str] = []
    obj_value = f"object.{prop.name}"

    if prop.cardinality == "scalar":
        scalar_lines = _generate_pack_value_scalar(prop, obj_value, f"_packed_{prop.name}")
        if scalar_lines:
            lines.extend(scalar_lines)
            if prop.is_required:
                lines.append(f'result["{prop.id}"] = _packed_{prop.name}')
            else:
                lines.extend(
                    [
                        f"if _packed_{prop.name} is not None:",
                        f'    result["{prop.id}"] = _packed_{prop.name}',
                    ]
                )
        else:
            return None
    elif prop.cardinality == "list":
        lines.extend(
            [
                f"if {obj_value}:",
                f"    _packed_{prop.name} = []",
                f"    for _item in {obj_value}:",
            ]
        )
        item_lines = _generate_pack_value_scalar(prop, "_item", "_packed_item")
        if item_lines:
            lines.extend(textwrap.indent("\n".join(item_lines), "        ").splitlines())
            lines.append(f"        _packed_{prop.name}.append(_packed_item)")
        else:
            lines.append(f"        _packed_{prop.name}.append(_item)")
        lines.append(f'    result["{prop.id}"] = _packed_{prop.name}')
    elif prop.cardinality == "map":
        lines.extend(
            [
                f"if {obj_value}:",
                f"    _packed_{prop.name} = {{}}",
                f"    for _key, _value in {obj_value}.items():",
            ]
        )
        key_lines = _generate_pack_value_scalar(prop, "_key", "_packed_key")
        value_lines = _generate_pack_value_scalar(prop, "_value", "_packed_value")
        if key_lines and value_lines:
            combined_lines = key_lines + value_lines
            lines.extend(textwrap.indent("\n".join(combined_lines), "        ").splitlines())
            lines.append(f"        _packed_{prop.name}[_packed_key] = _packed_value")
        elif key_lines:
            lines.extend(textwrap.indent("\n".join(key_lines), "        ").splitlines())
            lines.append(f"        _packed_{prop.name}[_packed_key] = _value")
        elif value_lines:
            lines.extend(textwrap.indent("\n".join(value_lines), "        ").splitlines())
            lines.append(f"        _packed_{prop.name}[_key] = _packed_value")
        else:
            lines.append(f"        _packed_{prop.name}[_key] = _value")
        lines.append(f'    result["{prop.id}"] = _packed_{prop.name}')
    else:
        assert_never(prop.cardinality)

    return lines


def _generate_unpack_value_property(prop: "Property") -> list[str] | None:
    """Generate the unpacking code for a property value."""
    lines: list[str] = []
    data_value = f'object_value.get("{prop.id}")'

    if prop.cardinality == "scalar":
        scalar_lines = _generate_unpack_value_scalar(prop, data_value, f"_unpacked_{prop.name}")
        if scalar_lines:
            lines.extend(scalar_lines)
        else:
            return None
    elif prop.cardinality == "list":
        lines.extend(
            [
                f"_unpacked_{prop.name} = []",
                f"if {data_value} is not None:",
                f"    for _item in {data_value}:",
            ]
        )
        item_lines = _generate_unpack_value_scalar(prop, "_item", "_unpacked_item")
        if item_lines:
            lines.extend(textwrap.indent("\n".join(item_lines), "        ").splitlines())
            lines.append(f"        _unpacked_{prop.name}.append(_unpacked_item)")
        else:
            lines.append(f"        _unpacked_{prop.name}.append(_item)")
    elif prop.cardinality == "map":
        lines.extend(
            [
                f"_unpacked_{prop.name} = {{}}",
                f"if {data_value} is not None:",
                f"    for _key, _value in {data_value}.items():",
            ]
        )
        key_lines = _generate_unpack_value_scalar(prop, "_key", "_unpacked_key")
        value_lines = _generate_unpack_value_scalar(prop, "_value", "_unpacked_value")
        if key_lines and value_lines:
            combined_lines = key_lines + value_lines
            lines.extend(textwrap.indent("\n".join(combined_lines), "        ").splitlines())
            lines.append(f"        _unpacked_{prop.name}[_unpacked_key] = _unpacked_value")
        elif key_lines:
            lines.extend(textwrap.indent("\n".join(key_lines), "        ").splitlines())
            lines.append(f"        _unpacked_{prop.name}[_unpacked_key] = _value")
        elif value_lines:
            lines.extend(textwrap.indent("\n".join(value_lines), "        ").splitlines())
            lines.append(f"        _unpacked_{prop.name}[_key] = _unpacked_value")
        else:
            lines.append(f"        _unpacked_{prop.name}[_key] = _value")
    else:
        assert_never(prop.cardinality)

    return lines


def _generate_pack_value_scalar(
    prop: "Property", value_expr: str, result_var: str
) -> list[str] | None:
    """Generate the packing code for a scalar value."""
    lines: list[str] = []

    def _wrap_with_null_check(code: str) -> str:
        """Wrap code with null check if property is optional."""
        if prop.is_required:
            return code
        return f"{code} if {value_expr} is not None else None"

    if prop.scalar_type == "primitive":
        if prop.primitive_type == PrimitiveType.BYTES:
            lines.append(
                f"{result_var} = {_wrap_with_null_check(f'base64.b64encode({value_expr}).decode()')}"
            )
        elif prop.primitive_type == PrimitiveType.UUID:
            lines.append(f"{result_var} = {_wrap_with_null_check(f'str({value_expr})')}")
        elif prop.primitive_type == PrimitiveType.JSON:
            lines.append(f"{result_var} = {value_expr}")
        elif prop.primitive_type in (
            PrimitiveType.DATE,
            PrimitiveType.TIME,
            PrimitiveType.DATETIME,
        ):
            lines.append(f"{result_var} = {_wrap_with_null_check(f'{value_expr}.isoformat()')}")
        elif prop.primitive_type == PrimitiveType.DURATION:
            lines.append(
                f"{result_var} = {_wrap_with_null_check(f'timedelta_to_isoformat({value_expr})')}"
            )
        else:
            return None
    elif prop.scalar_type == "enum":
        lines.append(f"{result_var} = {_wrap_with_null_check(f'{value_expr}.value')}")
    elif prop.scalar_type == "struct" or prop.scalar_type == "node":
        lines.append(f"{result_var} = {_wrap_with_null_check(f'{value_expr}.to_value()')}")
    else:
        assert_never(prop.scalar_type)

    return lines


def _generate_unpack_value_scalar(
    prop: "Property", value_expr: str, result_var: str
) -> list[str] | None:
    """Generate the unpacking code for a scalar value."""
    lines: list[str] = []

    def _wrap_with_null_check(code: str) -> str:
        """Wrap code with null check if property is optional."""
        if prop.is_required:
            return code
        return f"{code} if {value_expr} is not None else None"

    if prop.scalar_type == "primitive":
        if prop.primitive_type == PrimitiveType.BYTES:
            lines.append(
                f"{result_var} = {_wrap_with_null_check(f'base64.b64decode({value_expr})')}"
            )
        elif prop.primitive_type == PrimitiveType.UUID:
            lines.append(f"{result_var} = {_wrap_with_null_check(f'UUID({value_expr})')}")
        elif prop.primitive_type == PrimitiveType.JSON:
            lines.append(f"{result_var} = {value_expr}")
        elif prop.primitive_type == PrimitiveType.DATE:
            lines.append(
                f"{result_var} = {_wrap_with_null_check(f'date.fromisoformat({value_expr})')}"
            )
        elif prop.primitive_type == PrimitiveType.TIME:
            lines.append(
                f"{result_var} = {_wrap_with_null_check(f'time.fromisoformat({value_expr})')}"
            )
        elif prop.primitive_type == PrimitiveType.DATETIME:
            lines.append(
                f"{result_var} = {_wrap_with_null_check(f'datetime.fromisoformat({value_expr})')}"
            )
        elif prop.primitive_type == PrimitiveType.DURATION:
            lines.append(
                f"{result_var} = {_wrap_with_null_check(f'timedelta_from_isoformat({value_expr})')}"
            )
        else:
            return None
    elif prop.scalar_type == "enum":
        assert prop.enum_type is not None
        enum_type_name = prop.enum_type.bench_name
        lines.append(f"{result_var} = {_wrap_with_null_check(f'{enum_type_name}({value_expr})')}")
    elif prop.scalar_type == "struct":
        assert prop.struct_type is not None
        struct_cls_name = prop.struct_type.bench_name
        lines.append(
            f"{result_var} = {_wrap_with_null_check(f'{struct_cls_name}.from_value({value_expr})')}"
        )
    elif prop.scalar_type == "node":
        lines.append(
            f"{result_var} = {_wrap_with_null_check(f'unpack_builtin_object({value_expr}, supergraph=supergraph)')}"
        )
    else:
        assert_never(prop.scalar_type)

    return lines


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
from .object import BuiltinObject  # noqa: E402
