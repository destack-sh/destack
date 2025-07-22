import calendar
import textwrap
from collections.abc import Mapping
from datetime import UTC, datetime, timedelta
from typing import TYPE_CHECKING, Any, assert_never, cast

import structlog
from google.protobuf.duration_pb2 import Duration
from google.protobuf.json_format import MessageToDict
from google.protobuf.struct_pb2 import NULL_VALUE as PROTO_NULL_VALUE
from google.protobuf.struct_pb2 import ListValue as ProtoList
from google.protobuf.struct_pb2 import Struct as ProtoStruct
from google.protobuf.struct_pb2 import Value as ProtoValue
from google.protobuf.timestamp_pb2 import Timestamp
from opentelemetry import trace

from destack import proto
from destack.language.core import (
    BuiltinObject,
    NodeType,
    PrimitiveType,
    PropertyDeclaration,
    ScalarType,
    TypeCardinality,
    TypeDeclaration,
)
from destack.language.registry import STRUCT_CLASS_BY_TYPE, get_builtin_type
from destack.proto import AnyNodeProto, RpcMetadata
from destack.utils.string import Casing, to_casing

if TYPE_CHECKING:
    pass

# ruff: noqa: FURB113

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


def generate_pack_proto_impl(cls: type["BuiltinObject"]) -> tuple[str, dict[str, Any]]:
    """
    Generate BuiltinObject.__pack_proto__ and __unpack_proto__ class methods.
    """
    if cls.__is_abstract__:
        pack_proto = "raise RuntimeError('cannot pack abstract {cls.__name__}')"
        unpack_proto = "raise RuntimeError('cannot unpack abstract {cls.__name__}')"
    else:
        pack_proto = _generate_pack_proto(cls)
        unpack_proto = _generate_unpack_proto(cls)

    if cls.__is_frozen__ and not cls.__is_node__:
        to_proto = """\
def to_proto(self: "Self") -> "StructProtoT":
    if self._proto is None:
        self._proto = self.__pack_proto__(self)
    return self._proto
"""
    else:
        to_proto = """\
def to_proto(self: "Self") -> "StructProtoT":
    return self.__pack_proto__(self)
"""

    proto_impl = f"""
@classmethod
def __pack_proto__(cls, _object: "Self") -> "{cls.__name__}Proto":
{textwrap.indent(pack_proto, "  ")}

@classmethod
def __unpack_proto__(cls, 
    _object_proto: "{cls.__name__}Proto",
    _session: "Session | None" = None,
    _graph: "Graph | None" = None,
    _connection: "GraphConnection | None" = None,
) -> "Self":
{textwrap.indent(unpack_proto, "  ")}

{to_proto}

from_proto = __unpack_proto__
"""
    return proto_impl, {
        "Timestamp": Timestamp,
        "Duration": Duration,
        "UTC": UTC,
        "pack_proto_json": pack_proto_json,
        "unpack_proto_json": unpack_proto_json,
        "pack_proto_timestamp": pack_proto_timestamp,
        "unpack_proto_timestamp": unpack_proto_timestamp,
        "pack_proto_duration": pack_proto_duration,
        "unpack_proto_duration": unpack_proto_duration,
    }


def _generate_pack_proto(cls: type["BuiltinObject"]) -> str:
    """Generate the BuiltinObject.__pack_proto__ method implementation."""
    metatype = get_builtin_type(cls)
    pack_method_parts: list[str] = []
    pack_method_parts.append(f"_object_proto = {cls.__name__}Proto(metatype={metatype.value})")

    for prop in cls.__wired_properties__.values():
        if prop.name == "metatype":
            continue  # already set
        pack_code = _generate_pack_property(prop)
        if pack_code:
            pack_method_parts.extend(pack_code)

    pack_method_parts.append("return _object_proto")

    return "\n".join(pack_method_parts)


def _generate_unpack_proto(cls: type["BuiltinObject"]) -> str:
    """Generate the BuiltinObject.__unpack_proto__ method implementation."""
    unpack_assignments: list[str] = []
    unpack_method_parts: list[str] = []

    for prop in cls.__wired_properties__.values():
        if prop.is_computed:
            continue  # set implicitly
        prop_name = (
            prop.name if prop.scalar_type != ScalarType.NODE_REFERENCE else f"{prop.name}_ptr"
        )
        unpack_code = _generate_unpack_property(prop)
        if unpack_code:
            if len(unpack_code) == 1:
                unpack_assignments.append(f"{prop_name}={unpack_code[0].split(' = ', 1)[1]}")
            else:
                unpack_method_parts.extend(unpack_code)
                unpack_assignments.append(f"{prop_name}=_unpacked_{prop_name}")
        else:
            unpack_assignments.append(f"{prop_name}=_object_proto.{prop_name}")
    if cls.__is_frozen__ and not cls.__is_node__:
        unpack_assignments.append("_proto=_object_proto")

    unpack_method_parts.append("return cls(")
    for assignment in unpack_assignments:
        unpack_method_parts.append(f"    {assignment},")
    if cls.__is_node__:
        unpack_method_parts.append("    _session=_session,")
        unpack_method_parts.append("    _graph=_graph,")
        unpack_method_parts.append("    _connection=_connection,")
    else:
        unpack_method_parts.append("    _graph=_graph,")
    unpack_method_parts.append(")")

    return "\n".join(unpack_method_parts)


"""'Primitives' that actually map to Proto structs (Messages)"""
_PROTO_PRIMITIVE_MESSAGE_TYPES = (
    PrimitiveType.DATE,
    PrimitiveType.TIME,
    PrimitiveType.DATETIME,
    PrimitiveType.DURATION,
    PrimitiveType.JSON,
    PrimitiveType.CSON,
)


def _is_proto_primitive(prop: "PropertyDeclaration | TypeDeclaration") -> bool:
    """Check if a property is a proto primitive type."""
    return prop.cardinality == TypeCardinality.SCALAR and (
        prop.scalar_type == ScalarType.ENUM
        or (
            prop.scalar_type == ScalarType.PRIMITIVE
            and prop.primitive_type not in _PROTO_PRIMITIVE_MESSAGE_TYPES
        )
    )


def _generate_pack_property(prop: "PropertyDeclaration") -> list[str] | None:
    """Generate the packing code for a property value."""
    lines: list[str] = []
    prop_name = prop.name if prop.scalar_type != ScalarType.NODE_REFERENCE else f"{prop.name}_ptr"
    obj_value = f"_object.{prop_name}"

    if prop.cardinality == TypeCardinality.SCALAR:
        if prop.is_optional:
            lines.append(f"if ({prop_name} := {obj_value}) is not None:")
            scalar_expr = _generate_pack_scalar(prop, prop_name)
            if _is_proto_primitive(prop):
                lines.append(f"    _object_proto.{prop_name} = {scalar_expr}")
            else:
                lines.append(f"    _object_proto.{prop_name}.CopyFrom({scalar_expr})")
        else:
            scalar_expr = _generate_pack_scalar(prop, obj_value)
            if _is_proto_primitive(prop):
                lines.append(f"_object_proto.{prop_name} = {scalar_expr}")
            else:
                lines.append(f"_object_proto.{prop_name}.CopyFrom({scalar_expr})")
    elif prop.cardinality == TypeCardinality.LIST:
        item_expr = _generate_pack_scalar(prop, "_item")
        if _is_proto_primitive(prop):
            lines.extend(
                f"""\
if {obj_value}:
    _packed_{prop_name} = []
    for _item in {obj_value}:
        _packed_{prop_name}.append({item_expr})
    _object_proto.{prop_name} = _packed_{prop_name}""".splitlines()
            )
        else:
            lines.extend(
                f"""\
if {obj_value}:
    for _item in {obj_value}:
        _object_proto.{prop_name}.append({item_expr})""".splitlines()
            )
    elif prop.cardinality == TypeCardinality.MAP:
        lines.extend(
            f"""\
if {obj_value}:
    for _key, _value in {obj_value}.items():""".splitlines()
        )
        assert prop.key_type is not None, f"no key_type for map: {prop!r}"
        key_expr = _generate_pack_scalar(prop.key_type, "_key")
        value_expr = _generate_pack_scalar(prop, "_value")
        if _is_proto_primitive(prop):
            lines.append(f"        _object_proto.{prop_name}[{key_expr}] = {value_expr}")
        else:
            lines.append(f"        _object_proto.{prop_name}[{key_expr}].CopyFrom({value_expr})")
    else:
        assert_never(prop.cardinality)

    return lines


def _generate_unpack_property(prop: "PropertyDeclaration") -> list[str] | None:
    """Generate the unpacking code for a property value."""

    def _wrap_with_null_check(code: str) -> str:
        """Wrap code with null check if property is optional."""
        if prop.is_required:
            return code
        else:
            return f"{code} if _object_proto.HasField('{prop_name}') else None"

    lines: list[str] = []
    prop_name = prop.name if prop.scalar_type != ScalarType.NODE_REFERENCE else f"{prop.name}_ptr"
    proto_value = f"_object_proto.{prop_name}"

    if prop.cardinality == TypeCardinality.SCALAR:
        if prop.is_optional:
            scalar_expr = _generate_unpack_scalar(prop, proto_value)
            lines.append(f"_unpacked_{prop_name} = {_wrap_with_null_check(scalar_expr)}")
        else:
            scalar_expr = _generate_unpack_scalar(prop, proto_value)
            lines.append(f"_unpacked_{prop_name} = {scalar_expr}")
    elif prop.cardinality == TypeCardinality.LIST:
        lines.extend(
            f"""\
_unpacked_{prop_name} = []
for _item in {proto_value}:""".splitlines()
        )
        item_expr = _generate_unpack_scalar(prop, "_item")
        lines.append(f"    _unpacked_{prop_name}.append({item_expr})")
    elif prop.cardinality == TypeCardinality.MAP:
        lines.extend(
            f"""\
_unpacked_{prop_name} = {{}}
for _key, _value in {proto_value}.items():""".splitlines()
        )
        assert prop.key_type is not None, f"no key_type for map: {prop!r}"
        key_expr = _generate_unpack_scalar(prop.key_type, "_key")
        value_expr = _generate_unpack_scalar(prop, "_value")
        lines.append(f"    _unpacked_{prop_name}[{key_expr}] = {value_expr}")
    else:
        assert_never(prop.cardinality)

    return lines


def _generate_pack_scalar(prop: "PropertyDeclaration | TypeDeclaration", value_expr: str) -> str:
    """Generate the packing code for a scalar value."""

    if prop.scalar_type == ScalarType.PRIMITIVE:
        if prop.primitive_type == PrimitiveType.UUID:
            return f"str({value_expr})"
        elif prop.primitive_type == PrimitiveType.DATETIME:
            return f"pack_proto_timestamp({value_expr})"
        elif prop.primitive_type == PrimitiveType.DURATION:
            return f"pack_proto_duration({value_expr})"
        elif prop.primitive_type == PrimitiveType.JSON or prop.primitive_type == PrimitiveType.CSON:
            return f"pack_proto_json({value_expr})"
        else:
            return value_expr
    elif prop.scalar_type == ScalarType.ENUM:
        return f"{value_expr}.value"
    elif prop.scalar_type == ScalarType.NODE_REFERENCE:
        return f"{value_expr}.to_proto()"
    elif prop.scalar_type == ScalarType.NODE_VALUE:
        raise RuntimeError(f"node_value cannot be wired directly: {prop!r}")
    elif prop.scalar_type == ScalarType.STRUCT:
        assert prop.struct_type is not None
        struct_cls = STRUCT_CLASS_BY_TYPE[prop.struct_type]
        if struct_cls.__is_frozen__:
            return f"{value_expr}.to_proto()"  # use cached method
        else:
            return f"{struct_cls.__name__}.__pack_proto__({value_expr})"
    else:
        assert_never(prop.scalar_type)


def _generate_unpack_scalar(prop: "PropertyDeclaration | TypeDeclaration", value_expr: str) -> str:
    """Generate the unpacking code for a scalar value."""

    if prop.scalar_type == ScalarType.PRIMITIVE:
        if prop.primitive_type == PrimitiveType.UUID:
            return f"UUID({value_expr})"
        elif prop.primitive_type == PrimitiveType.DATETIME:
            return f"unpack_proto_timestamp({value_expr})"
        elif prop.primitive_type == PrimitiveType.DURATION:
            return f"unpack_proto_duration({value_expr})"
        elif prop.primitive_type == PrimitiveType.JSON or prop.primitive_type == PrimitiveType.CSON:
            return f"unpack_proto_json({value_expr})"
        else:
            return f"{value_expr}"
    elif prop.scalar_type == ScalarType.ENUM:
        assert prop.enum_type is not None
        enum_type_name = prop.enum_type.camel_name
        return f"{enum_type_name}({value_expr})"
    elif prop.scalar_type == ScalarType.NODE_REFERENCE:
        return f"NodeReference.__unpack_proto__({value_expr}, _graph=_graph)"
    elif prop.scalar_type == ScalarType.NODE_VALUE:
        raise RuntimeError(f"node_value cannot be wired directly: {prop!r}")
    elif prop.scalar_type == ScalarType.STRUCT:
        assert prop.struct_type is not None
        struct_cls = STRUCT_CLASS_BY_TYPE[prop.struct_type]
        return f"{struct_cls.__name__}.__unpack_proto__({value_expr}, _graph=_graph)"
    else:
        assert_never(prop.scalar_type)


_EPOCH_DATETIME_NAIVE = datetime(1970, 1, 1, tzinfo=UTC)


def pack_proto_timestamp(dt: datetime) -> Timestamp:
    seconds = calendar.timegm(dt.utctimetuple())
    nanos = dt.microsecond * 1000
    return Timestamp(seconds=seconds, nanos=nanos)


def unpack_proto_timestamp(timestamp: Timestamp) -> datetime:
    delta = timedelta(seconds=timestamp.seconds, microseconds=timestamp.nanos // 1000)
    return (_EPOCH_DATETIME_NAIVE + delta).astimezone(UTC)


def pack_proto_duration(td: timedelta) -> Duration:
    seconds = round(td.total_seconds() - (td.microseconds / 1000000))
    nanos = td.microseconds * 1000
    return Duration(seconds=seconds, nanos=nanos)


def unpack_proto_duration(duration: Duration) -> timedelta:
    return timedelta(seconds=duration.seconds, microseconds=duration.nanos // 1000)


def wrap_some_node(node: AnyNodeProto) -> proto.SomeNodeProto:
    """Wraps a concrete node type into a generic node message."""
    wrapper = proto.SomeNodeProto()
    field_name = to_casing(cast(str, NodeType(node.metatype).name), Casing.SNAKE)
    getattr(wrapper, field_name).CopyFrom(node)
    return wrapper


def unwrap_some_node(node: proto.SomeNodeProto) -> AnyNodeProto:
    """Unwraps a generic node type into a concrete node type."""
    node_key = node.WhichOneof("node")
    assert node_key is not None, f"node not set in {node!r}"
    wrapped_node = getattr(node, node_key)
    assert wrapped_node is not None, f"node not set in {node!r}"
    return wrapped_node


def pack_proto_json(value: Any) -> ProtoValue:
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


def unpack_proto_json(value: ProtoValue) -> Any:
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


def pack_rpc_headers(metadata: RpcMetadata) -> dict[str, str]:
    # flat encoding with prefix, messages as base64 :RpcMetadataEncoding
    packed = {
        "2": str(int(metadata.client_type)) if metadata.client_type is not None else None,
        "3": metadata.client_id or None,
        "4": metadata.client_nonce or None,
        "5": metadata.client_access_token or None,
    }
    return {"x-destack-" + k: v for k, v in packed.items() if v is not None}


def unpack_rpc_headers(headers: Mapping) -> RpcMetadata:
    # flat encoding with prefixy, messages as base64 :RpcMetadataEncoding
    metadata = RpcMetadata()
    if headers.get("x-destack-2"):
        metadata.client_type = cast(proto.ClientTypeProto, int(headers["x-destack-2"]))
    if headers.get("x-destack-3"):
        metadata.client_id = headers.get("x-destack-3")  # type: ignore
    if headers.get("x-destack-4"):
        metadata.client_nonce = headers.get("x-destack-4")  # type: ignore
    if headers.get("x-destack-5"):
        metadata.client_access_token = headers.get("x-destack-5")  # type: ignore
    return metadata
