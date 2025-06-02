import base64
import textwrap
from datetime import date, datetime, time, timedelta
from typing import TYPE_CHECKING, Any, assert_never

import structlog
from fastuuid import UUID
from opentelemetry import trace

from bench.language.registry import (
    BUILTIN_OBJECT_TYPE_BY_CLASS,
    ENUM_CLASS_BY_TYPE,
    STRUCT_CLASS_BY_TYPE,
)
from bench.pb2 import ValueData
from bench.utils.time import timedelta_from_isoformat, timedelta_to_isoformat

from .const import PrimitiveType, StructType
from .node import Node
from .property import IntoType, Property, property_, property_runtime_
from .struct import NodeReference, StructFrozen, struct_
from .type import Json, ScalarType, Type, TypeCardinality, to_type

if TYPE_CHECKING:
    from .object import BuiltinObjectBase


# ruff: noqa: FURB113
# pyright: reportIncompatibleVariableOverride=false


logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@struct_(StructType.VALUE, frozen=True)
class Value(StructFrozen[ValueData]):
    """A generic Value of any Type."""

    type: Type = property_(30, is_repr=True)
    value: Json = property_(40)

    _unpacked: Any | None = property_runtime_()

    def unpack(self) -> Any:
        """Get the unpacked value of this generic Value."""
        if self._unpacked is None:
            value_unpacked = unpack_value(self.value, self.type)
            object.__setattr__(self, "_unpacked", value_unpacked)
        return self._unpacked


def to_value(value_unpacked: Any, type: "Type | None" = None, node_as_value: bool = False) -> Value:
    """
    Convert an arbitrary (legal) value to a Value.
    If Type isn't provided, it will be inferred from the value.
    """
    if type is None:
        type = to_type(value_unpacked, node_as_value=node_as_value)
    value_packed = pack_value(value_unpacked, type)
    value = Value(type=type, value=value_packed, _unpacked=value_unpacked)
    return value


def pack_value(value: Any, type: Type) -> Json:
    """Pack a generic typed value to a JSON object."""
    if type.cardinality == TypeCardinality.SCALAR:
        return _pack_scalar_value(value, type)
    elif type.cardinality == TypeCardinality.LIST:
        if not value:
            return []
        packed_list: list[Json] = []
        for item in value:
            packed_list.append(_pack_scalar_value(item, type))
        return packed_list
    elif type.cardinality == TypeCardinality.MAP:
        if not value:
            return {}
        assert type.key_type is not None, f"no key type for {type!r}"
        packed_map: dict[str, Json] = {}
        for key, val in value.items():
            packed_key = _pack_scalar_value(key, type.key_type)
            packed_val = _pack_scalar_value(val, type)
            packed_map[str(packed_key)] = packed_val
        return packed_map
    else:
        assert_never(type.cardinality)


def unpack_value(value: Json, type: Type) -> Any:
    """Unpack a JSON object to a generic typed value."""
    if type.cardinality == TypeCardinality.SCALAR:
        return _unpack_scalar_value(value, type)
    elif type.cardinality == TypeCardinality.LIST:
        if value is None:
            return []
        unpacked_list = []
        for item in value:
            unpacked_list.append(_unpack_scalar_value(item, type))
        return unpacked_list
    elif type.cardinality == TypeCardinality.MAP:
        if value is None:
            return {}
        unpacked_map = {}
        for key, val in value.items():
            unpacked_key = _unpack_scalar_value(key, type.key_type) if type.key_type else key
            unpacked_val = _unpack_scalar_value(val, type)
            unpacked_map[unpacked_key] = unpacked_val
        return unpacked_map
    else:
        assert_never(type.cardinality)


def _pack_scalar_value(value: Any, type: Type) -> Json:
    """Pack a scalar value to JSON."""
    if type.scalar_type == ScalarType.PRIMITIVE:
        if type.primitive_type == PrimitiveType.BYTES:
            return base64.b64encode(value).decode()
        elif type.primitive_type == PrimitiveType.UUID:
            return str(value)
        elif type.primitive_type == PrimitiveType.JSON:
            return value
        elif type.primitive_type in (
            PrimitiveType.DATE,
            PrimitiveType.TIME,
            PrimitiveType.DATETIME,
        ):
            return value.isoformat()
        elif type.primitive_type == PrimitiveType.DURATION:
            return timedelta_to_isoformat(value)
        else:
            return value
    elif type.scalar_type == ScalarType.ENUM:
        return value.value
    elif type.scalar_type in (ScalarType.NODE_REFERENCE, ScalarType.NODE_VALUE, ScalarType.STRUCT):
        return value.to_value()
    else:
        assert_never(type.scalar_type)


def _unpack_scalar_value(value: Json, type: Type) -> Any:
    """Unpack a scalar value from JSON."""
    if type.scalar_type == ScalarType.PRIMITIVE:
        if type.primitive_type == PrimitiveType.BYTES:
            return base64.b64decode(value)
        elif type.primitive_type == PrimitiveType.UUID:
            return UUID(value)
        elif type.primitive_type == PrimitiveType.JSON:
            return value
        elif type.primitive_type == PrimitiveType.DATE:
            return date.fromisoformat(value)
        elif type.primitive_type == PrimitiveType.TIME:
            return time.fromisoformat(value)
        elif type.primitive_type == PrimitiveType.DATETIME:
            return datetime.fromisoformat(value)
        elif type.primitive_type == PrimitiveType.DURATION:
            return timedelta_from_isoformat(value)
        elif type.primitive_type in (PrimitiveType.INT16, PrimitiveType.INT32, PrimitiveType.INT64):
            return int(value)  # cast JSON floats to ints
        else:
            return value
    elif type.scalar_type == ScalarType.ENUM:
        assert type.enum_type is not None, f"no enum type for {type!r}"
        enum_cls = ENUM_CLASS_BY_TYPE[type.enum_type]
        return enum_cls(int(value))
    elif type.scalar_type == ScalarType.NODE_REFERENCE:
        return NodeReference.from_value(value)
    elif type.scalar_type == ScalarType.NODE_VALUE:
        return Node.from_value(value)
    elif type.scalar_type == ScalarType.STRUCT:
        assert type.struct_type is not None, f"no struct type for {type!r}"
        struct_cls = STRUCT_CLASS_BY_TYPE[type.struct_type]
        return struct_cls.from_value(value)
    else:
        assert_never(type.scalar_type)


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
    elif prop.scalar_type in ("struct", "node_reference", "node_value"):
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
            return f"int({value_expr})"  # cast JSON floats to ints
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
    elif prop.scalar_type == "node_reference":
        return f"NodeReference.from_value({value_expr})"
    elif prop.scalar_type == "node_value":
        return f"Node.from_value({value_expr})"
    else:
        assert_never(prop.scalar_type)
