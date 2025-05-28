import calendar
import json
import textwrap
from base64 import b64decode, b64encode
from datetime import datetime, timedelta
from itertools import chain
from typing import TYPE_CHECKING, Any, Mapping, Union, assert_never, cast

import pytz
import structlog
from google.protobuf.duration_pb2 import Duration
from google.protobuf.timestamp_pb2 import Timestamp
from opentelemetry import trace

from bench import pb2
from bench.language.core import (
    BuiltinObjectBase,
    IntoType,
    NodeType,
    PrimitiveType,
    Property,
    StructType,
    Variable,
    pack_proto_json,
    unpack_proto_json,
)
from bench.language.registry import (
    BUILTIN_OBJECT_CLASS_BY_TYPE,
    BUILTIN_OBJECT_TYPE_BY_CLASS,
    STRUCT_CLASS_BY_TYPE,
)
from bench.pb2 import AnyNodeData, AnyStructData, RpcMetadata
from bench.utils.string import Casing, to_casing

if TYPE_CHECKING:
    pass

# ruff: noqa: FURB113

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


PROTO_CLASS_BY_TYPE: dict[NodeType | StructType, type[Union[AnyNodeData, AnyStructData]]] = {
    object_type: getattr(pb2, object_type.bench_name + "Data")
    for object_type in chain(NodeType, StructType)
    if hasattr(pb2, object_type.bench_name + "Data")  # may just be creating a new class
}
OBJECT_TYPE_BY_PROTO_CLASS: dict[type[Union[AnyNodeData, AnyStructData]], NodeType | StructType] = {
    cls: object_type for object_type, cls in PROTO_CLASS_BY_TYPE.items()
}
BENCH_CLASS_BY_PROTO_CLASS: dict[
    type[Union[AnyNodeData, AnyStructData]], type[BuiltinObjectBase]
] = {
    cls: BUILTIN_OBJECT_CLASS_BY_TYPE[object_type]
    for cls, object_type in OBJECT_TYPE_BY_PROTO_CLASS.items()
}


def generate_pack_proto_impl(cls: type["BuiltinObjectBase"]) -> tuple[str, dict[str, Any]]:
    """
    Generate BuiltinObject.__pack_proto__ and __unpack_proto__ class methods.
    """
    pack_proto = textwrap.indent(_generate_pack_proto(cls), "    ")
    unpack_proto = textwrap.indent(_generate_unpack_proto(cls), "    ")

    if cls.__is_frozen__:
        to_proto = """\
def to_proto(self: "Self") -> "StructDataT":
    if self._proto is None:
        self._proto = self.__pack_proto__(self)
    return self._proto
"""
    else:
        to_proto = """\
def to_proto(self: "Self") -> "StructDataT":
    return self.__pack_proto__(self)
"""

    proto_impl = f"""
@classmethod
def __pack_proto__(cls, _object: "Self") -> "{cls.__name__}Data":
{pack_proto}

@classmethod
def __unpack_proto__(cls, _object_data: "{cls.__name__}Data") -> "Self":
{unpack_proto}

{to_proto}

from_proto = __unpack_proto__
"""
    return proto_impl, {
        "Variable": Variable,
        "Timestamp": Timestamp,
        "Duration": Duration,
        "pytz": pytz,
        "pack_proto_json": pack_proto_json,
        "unpack_proto_json": unpack_proto_json,
        "pack_proto_timestamp": pack_proto_timestamp,
        "unpack_proto_timestamp": unpack_proto_timestamp,
        "pack_proto_duration": pack_proto_duration,
        "unpack_proto_duration": unpack_proto_duration,
    }


def _generate_pack_proto(cls: type["BuiltinObjectBase"]) -> str:
    """Generate the BuiltinObject.__pack_proto__ method implementation."""
    metatype = BUILTIN_OBJECT_TYPE_BY_CLASS[cls]
    pack_method_parts: list[str] = []
    pack_method_parts.append(f"_object_data = {cls.__name__}Data(metatype={metatype.value})")

    for prop in cls.__wired_properties__.values():
        if prop.name == "metatype":
            continue  # already set
        pack_code = _generate_pack_property(prop)
        if pack_code:
            pack_method_parts.extend(pack_code)

    pack_method_parts.append("return _object_data")

    return "\n".join(pack_method_parts)


def _generate_unpack_proto(cls: type["BuiltinObjectBase"]) -> str:
    """Generate the BuiltinObject.__unpack_proto__ method implementation."""
    unpack_assignments: list[str] = []
    unpack_method_parts: list[str] = []

    for prop in cls.__wired_properties__.values():
        if prop.is_computed:
            continue  # set implicitly
        unpack_code = _generate_unpack_property(prop)
        if unpack_code:
            if len(unpack_code) == 1:
                unpack_assignments.append(f"{prop.name}={unpack_code[0].split(' = ', 1)[1]}")
            else:
                unpack_method_parts.extend(unpack_code)
                unpack_assignments.append(f"{prop.name}=_unpacked_{prop.name}")
        else:
            unpack_assignments.append(f"{prop.name}=_object_data.{prop.name}")
    if cls.__is_frozen__:
        unpack_assignments.append("_proto=_object_data")

    unpack_method_parts.append("return cls(")
    for i, assignment in enumerate(unpack_assignments):
        comma = "," if i < len(unpack_assignments) - 1 else ""
        unpack_method_parts.append(f"    {assignment}{comma}")
    unpack_method_parts.append(")")

    return "\n".join(unpack_method_parts)


def _is_proto_primitive(prop: "Property | IntoType") -> bool:
    """Check if a property is a proto primitive type."""
    return prop.cardinality == "scalar" and (
        prop.scalar_type == "enum"
        or (
            prop.scalar_type == "primitive"
            and prop.primitive_type
            not in (
                PrimitiveType.DATE,
                PrimitiveType.TIME,
                PrimitiveType.DATETIME,
                PrimitiveType.DURATION,
            )
        )
    )


def _generate_pack_property(prop: "Property") -> list[str] | None:
    """Generate the packing code for a property value."""
    lines: list[str] = []
    obj_value = f"_object.{prop.name}"

    if prop.cardinality == "scalar":
        if prop.is_variable:
            # write to _variable if it's a Variable, else write to _value
            lines.append(f"if isinstance({obj_value}, Variable):")
            lines.append(f"    _object_data.{prop.name}.CopyFrom({obj_value}.to_proto())")
            lines.append(f"elif ({prop.name} := {obj_value}) is not None:")
            scalar_expr = _generate_pack_scalar(prop, prop.name)
            if _is_proto_primitive(prop):
                lines.append(f"    _object_data.{prop.name}_value = {scalar_expr}")
            else:
                lines.append(f"    _object_data.{prop.name}_value.CopyFrom({scalar_expr})")
        elif prop.is_optional:
            lines.append(f"if ({prop.name} := {obj_value}) is not None:")
            scalar_expr = _generate_pack_scalar(prop, prop.name)
            if _is_proto_primitive(prop):
                lines.append(f"    _object_data.{prop.name} = {scalar_expr}")
            else:
                lines.append(f"    _object_data.{prop.name}.CopyFrom({scalar_expr})")
        else:
            scalar_expr = _generate_pack_scalar(prop, obj_value)
            if _is_proto_primitive(prop):
                lines.append(f"_object_data.{prop.name} = {scalar_expr}")
            else:
                lines.append(f"_object_data.{prop.name}.CopyFrom({scalar_expr})")
    elif prop.cardinality == "list":
        item_expr = _generate_pack_scalar(prop, "_item")
        if _is_proto_primitive(prop):
            lines.extend(
                f"""\
if {obj_value}:
    _packed_{prop.name} = []
    for _item in {obj_value}:
        _packed_{prop.name}.append({item_expr})
    _object_data.{prop.name} = _packed_{prop.name}""".splitlines()
            )
        else:
            lines.extend(
                f"""\
if {obj_value}:
    for _item in {obj_value}:
        _object_data.{prop.name}.append({item_expr})""".splitlines()
            )
    elif prop.cardinality == "map":
        lines.extend(
            f"""\
if {obj_value}:
    for _key, _value in {obj_value}.items():""".splitlines()
        )
        assert prop.key_type is not None, f"no key_type for map: {prop!r}"
        key_expr = _generate_pack_scalar(prop.key_type, "_key")
        value_expr = _generate_pack_scalar(prop, "_value")
        if _is_proto_primitive(prop):
            lines.append(f"        _object_data.{prop.name}[{key_expr}] = {value_expr}")
        else:
            lines.append(f"        _object_data.{prop.name}[{key_expr}].CopyFrom({value_expr})")
    else:
        assert_never(prop.cardinality)

    return lines


def _generate_unpack_property(prop: "Property") -> list[str] | None:
    """Generate the unpacking code for a property value."""

    def _wrap_with_null_check(code: str) -> str:
        """Wrap code with null check if property is optional."""
        if prop.is_required:
            return code
        else:
            return f"{code} if _object_data.HasField('{prop.name}') else None"

    lines: list[str] = []
    data_value = f"_object_data.{prop.name}"

    if prop.cardinality == "scalar":
        if prop.is_variable:
            # read from _variable if it's a Variable, else read from _value
            lines.append(f"if _object_data.HasField('{prop.name}_variable'):")
            scalar_expr_variable = _generate_unpack_scalar(prop, f"{data_value}_variable")
            lines.append(f"    _unpacked_{prop.name} = {scalar_expr_variable}")
            lines.append(f"elif _object_data.HasField('{prop.name}_value'):")
            scalar_expr_value = _generate_unpack_scalar(prop, f"{data_value}_value")
            lines.append(f"    _unpacked_{prop.name} = {scalar_expr_value}")
            lines.append("else:")
            lines.append(f"    _unpacked_{prop.name} = None")
        elif prop.is_optional:
            scalar_expr = _generate_unpack_scalar(prop, data_value)
            lines.append(f"_unpacked_{prop.name} = {_wrap_with_null_check(scalar_expr)}")
        else:
            scalar_expr = _generate_unpack_scalar(prop, data_value)
            lines.append(f"_unpacked_{prop.name} = {scalar_expr}")
    elif prop.cardinality == "list":
        lines.extend(
            f"""\
_unpacked_{prop.name} = []
for _item in {data_value}:""".splitlines()
        )
        item_expr = _generate_unpack_scalar(prop, "_item")
        lines.append(f"    _unpacked_{prop.name}.append({item_expr})")
    elif prop.cardinality == "map":
        lines.extend(
            f"""\
_unpacked_{prop.name} = {{}}
for _key, _value in {data_value}.items():""".splitlines()
        )
        assert prop.key_type is not None, f"no key_type for map: {prop!r}"
        key_expr = _generate_unpack_scalar(prop.key_type, "_key")
        value_expr = _generate_unpack_scalar(prop, "_value")
        lines.append(f"    _unpacked_{prop.name}[{key_expr}] = {value_expr}")
    else:
        assert_never(prop.cardinality)

    return lines


def _generate_pack_scalar(prop: "Property | IntoType", value_expr: str) -> str:
    """Generate the packing code for a scalar value."""

    if prop.scalar_type == "primitive":
        if prop.primitive_type == PrimitiveType.UUID:
            return f"str({value_expr})"
        elif prop.primitive_type == PrimitiveType.JSON:
            return f"pack_proto_json({value_expr})"
        elif prop.primitive_type == PrimitiveType.DATETIME:
            return f"pack_proto_timestamp({value_expr})"
        elif prop.primitive_type == PrimitiveType.DURATION:
            return f"pack_proto_duration({value_expr})"
        else:
            return value_expr
    elif prop.scalar_type == "enum":
        return f"{value_expr}.value"
    elif prop.scalar_type == "struct":
        assert prop.struct_type is not None
        struct_cls = STRUCT_CLASS_BY_TYPE[prop.struct_type]
        if struct_cls.__is_frozen__:
            return f"{value_expr}.to_proto()"  # use cached method
        else:
            return f"{struct_cls.__name__}.__pack_proto__({value_expr})"
    elif prop.scalar_type == "node":
        return f"{value_expr}.to_proto()"
    else:
        assert_never(prop.scalar_type)


def _generate_unpack_scalar(prop: "Property | IntoType", value_expr: str) -> str:
    """Generate the unpacking code for a scalar value."""

    if prop.scalar_type == "primitive":
        if prop.primitive_type == PrimitiveType.UUID:
            return f"UUID({value_expr})"
        elif prop.primitive_type == PrimitiveType.JSON:
            return f"unpack_proto_json({value_expr})"
        elif prop.primitive_type == PrimitiveType.DATETIME:
            return f"unpack_proto_timestamp({value_expr})"
        elif prop.primitive_type == PrimitiveType.DURATION:
            return f"unpack_proto_duration({value_expr})"
        else:
            return f"{value_expr}"
    elif prop.scalar_type == "enum":
        assert prop.enum_type is not None
        enum_type_name = prop.enum_type.bench_name
        return f"{enum_type_name}({value_expr})"
    elif prop.scalar_type == "struct":
        assert prop.struct_type is not None
        struct_cls_name = prop.struct_type.bench_name
        return f"{struct_cls_name}.__unpack_proto__({value_expr})"
    elif prop.scalar_type == "node":
        return f"NodeReference.__unpack_proto__({value_expr})"
    else:
        assert_never(prop.scalar_type)


_EPOCH_DATETIME_NAIVE = datetime(1970, 1, 1, tzinfo=None)  # noqa: DTZ001


def pack_proto_timestamp(dt: datetime) -> Timestamp:
    seconds = calendar.timegm(dt.utctimetuple())
    nanos = dt.microsecond * 1000
    return Timestamp(seconds=seconds, nanos=nanos)


def unpack_proto_timestamp(timestamp: Timestamp) -> datetime:
    delta = timedelta(seconds=timestamp.seconds, microseconds=timestamp.nanos // 1000)
    return (_EPOCH_DATETIME_NAIVE + delta).replace(tzinfo=pytz.utc)


def pack_proto_duration(td: timedelta) -> Duration:
    seconds = round(td.total_seconds() - (td.microseconds / 1000000))
    nanos = td.microseconds * 1000
    return Duration(seconds=seconds, nanos=nanos)


def unpack_proto_duration(duration: Duration) -> timedelta:
    return timedelta(seconds=duration.seconds, microseconds=duration.nanos // 1000)


def wrap_some_node(node: AnyNodeData) -> pb2.SomeNodeData:
    """Wraps a concrete node type into a generic node message."""
    wrapper = pb2.SomeNodeData()
    field_name = to_casing(cast(str, NodeType(node.metatype).name), Casing.SNAKE)
    getattr(wrapper, field_name).CopyFrom(node)
    return wrapper


def unwrap_some_node(node: pb2.SomeNodeData) -> AnyNodeData:
    """Unwraps a generic node type into a concrete node type."""
    node_key = node.WhichOneof("node")
    assert node_key is not None, f"node not set in {node!r}"
    wrapped_node = getattr(node, node_key)
    assert wrapped_node is not None, f"node not set in {node!r}"
    return wrapped_node


def pack_rpc_headers(metadata: RpcMetadata) -> dict[str, str]:
    # flat encoding with prefix, messages as base64 :RpcMetadataEncoding
    packed = {
        "2": str(int(metadata.client_type)) if metadata.client_type is not None else None,
        "3": metadata.client_id or None,
        "4": metadata.client_nonce or None,
        "5": metadata.client_access_token or None,
    }
    packed_badges = [
        {
            "2": badge.id,
            "3": badge.key or None,
            "4": badge.password or None,
        }
        for badge in metadata.badges
    ]
    if packed_badges:
        packed["6"] = b64encode(json.dumps(packed_badges).encode("utf-8")).decode("utf-8")
    return {"x-bench-" + k: v for k, v in packed.items() if v is not None}


def unpack_rpc_headers(headers: Mapping) -> RpcMetadata:
    # flat encoding with prefixy, messages as base64 :RpcMetadataEncoding
    metadata = RpcMetadata()
    if headers.get("x-bench-2"):
        metadata.client_type = cast(pb2.ClientType, int(headers["x-bench-2"]))
    if headers.get("x-bench-3"):
        metadata.client_id = headers.get("x-bench-3")  # type: ignore
    if headers.get("x-bench-4"):
        metadata.client_nonce = headers.get("x-bench-4")  # type: ignore
    if headers.get("x-bench-5"):
        metadata.client_access_token = headers.get("x-bench-5")  # type: ignore
    if headers.get("6"):
        unpacked_badges = json.loads(b64decode(headers.get("x-bench-6")).decode("utf-8"))  # type: ignore
        for unpacked_badge in unpacked_badges:
            metadata_badge = metadata.badges.add()
            metadata_badge.id = unpacked_badge.get("2")
            metadata_badge.key = unpacked_badge.get("3")
            metadata_badge.password = unpacked_badge.get("4")
    return metadata
