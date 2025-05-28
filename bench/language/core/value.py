import base64
import textwrap
from datetime import date, datetime, time, timedelta
from typing import (
    TYPE_CHECKING,
    Any,
    Union,
    assert_never,
)

import structlog
from fastuuid import UUID
from google.protobuf.json_format import MessageToDict
from google.protobuf.struct_pb2 import NULL_VALUE as PROTO_NULL_VALUE
from google.protobuf.struct_pb2 import ListValue as ProtoList
from google.protobuf.struct_pb2 import Struct as ProtoStruct
from google.protobuf.struct_pb2 import Value as ProtoValue
from opentelemetry import trace

from bench.language.registry import BUILTIN_OBJECT_TYPE_BY_CLASS
from bench.pb2 import Date, TimeOfDay, ValueData
from bench.utils.time import timedelta_from_isoformat, timedelta_to_isoformat

from .const import PrimitiveType, StructType
from .property import IntoType, Property, property_
from .struct import StructFrozen, struct_
from .type import Json

if TYPE_CHECKING:
    from .object import BuiltinObjectBase


# ruff: noqa: FURB113
# pyright: reportIncompatibleVariableOverride=false


logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

JsonPrimitive = Union[str, int, float, bool, None]
JsonValue = Union[JsonPrimitive, dict[str, "JsonValue"], list["JsonValue"]]


@struct_(StructType.VALUE, frozen=True)
class Value(StructFrozen[ValueData]):
    """A value of any type."""

    value: dict[str, Json] = property_(35)


def to_value(value: Any) -> Value:
    """Convert an arbitrary value to a Value."""
    raise NotImplementedError


def generate_pack_value_impl(cls: type["BuiltinObjectBase"]) -> tuple[str, dict[str, Any]]:
    """Generate the BuiltinObject.__pack_value__/__unpack_value__ method implementations."""

    pack_value = textwrap.indent(_generate_pack_value(cls), "    ")
    unpack_value = textwrap.indent(_generate_unpack_value(cls), "    ")

    value_impl = f"""
@classmethod
def __pack_value__(cls, _object: "Self") -> dict[str, "JsonValue"]:
{pack_value}

@classmethod
def __unpack_value__(cls, _object_value: dict[str, "JsonValue"]) -> "Self":
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
        "base64": base64,
        "UUID": UUID,
    }


def _generate_pack_value(cls: type["BuiltinObjectBase"]) -> str:
    """Generate the BuiltinObject.__pack_value__ method implementation."""
    pack_method_parts: list[str] = []
    pack_method_parts.append("_object_value = {}")

    wired_properties_in_order = list(cls.__wired_properties__.values())
    wired_properties_in_order.sort(key=lambda p: p.id or 0)
    for prop in wired_properties_in_order:
        if prop.name == "metatype":
            metatype = BUILTIN_OBJECT_TYPE_BY_CLASS[cls]
            pack_method_parts.append(f'_object_value["{prop.id}"] = {metatype.value}')
            continue

        pack_code = _generate_pack_value_property(prop)
        if pack_code:
            pack_method_parts.extend(pack_code)

    pack_method_parts.append("return _object_value")
    return "\n".join(pack_method_parts)


def _generate_unpack_value(cls: type["BuiltinObjectBase"]) -> str:
    """Generate the BuiltinObject.__unpack_value__ method implementation."""
    unpack_assignments: list[str] = []
    unpack_method_parts: list[str] = []

    for prop in cls.__wired_properties__.values():
        if prop.is_computed:
            continue  # set implicitly
        unpack_code = _generate_unpack_value_property(prop)
        if len(unpack_code) == 1:
            unpack_assignments.append(f"{prop.name}={unpack_code[0].split(' = ', 1)[1]}")
        else:
            unpack_method_parts.extend(unpack_code)
            unpack_assignments.append(f"{prop.name}=_unpacked_{prop.name}")

    unpack_method_parts.append("return cls(")
    for i, assignment in enumerate(unpack_assignments):
        comma = "," if i < len(unpack_assignments) - 1 else ""
        unpack_method_parts.append(f"    {assignment}{comma}")

    unpack_method_parts.append(")")

    return "\n".join(unpack_method_parts)


def _generate_pack_value_property(prop: "Property") -> list[str]:
    """Generate the packing code for a property value."""
    lines: list[str] = []
    obj_value = f"_object.{prop.name}"

    if prop.cardinality == "scalar":
        if prop.is_required:
            value_expr = _generate_pack_value_scalar(prop, obj_value)
            lines.append(f'_object_value["{prop.id}"] = {value_expr}')
        else:
            lines.append(f"if ({prop.name} := {obj_value}) is not None:")
            value_expr = _generate_pack_value_scalar(prop, prop.name)
            lines.append(f'    _object_value["{prop.id}"] = {value_expr}')
    elif prop.cardinality == "list":
        lines.append(f"if {obj_value}:")
        lines.append(f"    _packed_{prop.name} = []")
        lines.append(f"    for _item in {obj_value}:")
        item_expr = _generate_pack_value_scalar(prop, "_item")
        lines.append(f"        _packed_{prop.name}.append({item_expr})")
        lines.append(f'    _object_value["{prop.id}"] = _packed_{prop.name}')
    elif prop.cardinality == "map":
        assert prop.key_type is not None, f"no key type for {prop!r}"
        lines.append(f"if {obj_value}:")
        lines.append(f"    _packed_{prop.name} = {{}}")
        lines.append(f"    for _key, _value in {obj_value}.items():")
        key_expr = _generate_pack_value_scalar(prop.key_type, "_key")
        value_expr = _generate_pack_value_scalar(prop, "_value")
        lines.append(f"        _packed_{prop.name}[str({key_expr})] = {value_expr}")
        lines.append(f'    _object_value["{prop.id}"] = _packed_{prop.name}')
    else:
        assert_never(prop.cardinality)

    return lines


def _generate_unpack_value_property(prop: "Property") -> list[str]:
    """Generate the unpacking code for a property value."""
    lines: list[str] = []
    data_value = f'_object_value.get("{prop.id}")'

    if prop.cardinality == "scalar":
        if prop.is_required:
            value_expr = _generate_unpack_value_scalar(prop, data_value)
            lines.append(f"_unpacked_{prop.name} = {value_expr}")
        else:
            value_expr = _generate_unpack_value_scalar(prop, prop.name)
            lines.append(
                f"_unpacked_{prop.name} = {value_expr} if ({prop.name} := {data_value}) is not None else None"
            )
    elif prop.cardinality == "list":
        lines.append(f"_unpacked_{prop.name} = []")
        lines.append(f"if {data_value} is not None:")
        lines.append(f"    for _item in {data_value}:")
        item_expr = _generate_unpack_value_scalar(prop, "_item")
        lines.append(f"        _unpacked_{prop.name}.append({item_expr})")
    elif prop.cardinality == "map":
        assert prop.key_type is not None, f"no key type for {prop!r}"
        lines.append(f"_unpacked_{prop.name} = {{}}")
        lines.append(f"if {data_value} is not None:")
        lines.append(f"    for _key, _value in {data_value}.items():")
        key_expr = _generate_unpack_value_scalar(prop.key_type, "_key")
        value_expr = _generate_unpack_value_scalar(prop, "_value")
        lines.append(f"        _unpacked_{prop.name}[{key_expr}] = {value_expr}")
    else:
        assert_never(prop.cardinality)

    return lines


def _generate_pack_value_scalar(prop: "Property | IntoType", value_expr: str) -> str:
    """Generate the packing code for a scalar value."""

    if prop.scalar_type == "primitive":
        if prop.primitive_type == PrimitiveType.BYTES:
            return f"base64.b64encode({value_expr}).decode()"
        elif prop.primitive_type == PrimitiveType.UUID:
            return f"str({value_expr})"
        elif prop.primitive_type == PrimitiveType.JSON:
            return value_expr
        elif prop.primitive_type in (
            PrimitiveType.DATE,
            PrimitiveType.TIME,
            PrimitiveType.DATETIME,
        ):
            return f"{value_expr}.isoformat()"
        elif prop.primitive_type == PrimitiveType.DURATION:
            return f"timedelta_to_isoformat({value_expr})"
        else:
            return value_expr
    elif prop.scalar_type == "enum":
        return f"{value_expr}.value"
    elif prop.scalar_type == "struct" or prop.scalar_type == "node":
        return f"{value_expr}.to_value()"
    else:
        assert_never(prop.scalar_type)


def _generate_unpack_value_scalar(prop: "Property | IntoType", value_expr: str) -> str:
    """Generate the unpacking code for a scalar value."""

    if prop.scalar_type == "primitive":
        if prop.primitive_type == PrimitiveType.BYTES:
            return f"base64.b64decode({value_expr})"
        elif prop.primitive_type == PrimitiveType.UUID:
            return f"UUID({value_expr})"
        elif prop.primitive_type == PrimitiveType.JSON:
            return value_expr
        elif prop.primitive_type == PrimitiveType.DATE:
            return f"date.fromisoformat({value_expr})"
        elif prop.primitive_type == PrimitiveType.TIME:
            return f"time.fromisoformat({value_expr})"
        elif prop.primitive_type == PrimitiveType.DATETIME:
            return f"datetime.fromisoformat({value_expr})"
        elif prop.primitive_type == PrimitiveType.DURATION:
            return f"timedelta_from_isoformat({value_expr})"
        elif prop.primitive_type in (PrimitiveType.INT16, PrimitiveType.INT32, PrimitiveType.INT64):
            return f"int({value_expr})"
        else:
            return value_expr
    elif prop.scalar_type == "enum":
        assert prop.enum_type is not None, f"no enum type for {prop!r}"
        enum_type_name = prop.enum_type.bench_name
        return f"{enum_type_name}(int({value_expr}))"
    elif prop.scalar_type == "struct":
        assert prop.struct_type is not None, f"no struct type for {prop!r}"
        struct_cls_name = prop.struct_type.bench_name
        return f"{struct_cls_name}.from_value({value_expr})"
    elif prop.scalar_type == "node":
        return f"NodeReference.from_value({value_expr})"
    else:
        assert_never(prop.scalar_type)


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
