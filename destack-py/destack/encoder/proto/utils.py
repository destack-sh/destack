import calendar
from collections.abc import Mapping
from datetime import UTC, datetime, timedelta
from typing import TYPE_CHECKING, Any, cast

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
    NodeType,
)
from destack.proto import AnyNodeProto, RpcMetadata
from destack.utils.string import Casing, to_casing

if TYPE_CHECKING:
    pass


logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


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
