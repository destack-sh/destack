import base64
import datetime
from typing import (
    Any,
    cast,
)

from google.protobuf.duration_pb2 import Duration
from google.protobuf.timestamp_pb2 import Timestamp
from psycopg.types.json import Jsonb

from bench.language import (
    PrimitiveType,
    Property,
    pack_builtin_object_data,
    pack_proto_date,
    pack_proto_json,
    pack_proto_time,
    unpack_builtin_object_data,
    unpack_proto_date,
    unpack_proto_json,
    unpack_proto_time,
)
from bench.pb2 import Date, TimeOfDay
from bench.utils.func import to_uuid
from bench.utils.time import timedelta_from_isoformat

from .core import SqlPrimitive
from .core import SqlTable as SqlTable

# nocheckin: generate sql pack/unpack for BuiltinObjects


def _pack_builtin_object_data_prop_scalar(prop: Property, value: Any) -> SqlPrimitive:
    """Packs the value of a BuiltinObject property for storage in Postgres."""
    if value is None:
        return None
    elif prop.is_struct:
        value = pack_builtin_object_data(value)
        return Jsonb(value)  # type: ignore
    elif prop.primitive_type == PrimitiveType.UUID:
        return to_uuid(value)
    elif prop.primitive_type == PrimitiveType.JSON:
        return Jsonb(unpack_proto_json(value))  # type: ignore
    elif prop.primitive_type == PrimitiveType.DATETIME:
        return cast(Timestamp, value).ToDatetime()
    elif prop.primitive_type == PrimitiveType.DATE:
        return unpack_proto_date(cast(Date, value))
    elif prop.primitive_type == PrimitiveType.TIME:
        return unpack_proto_time(cast(TimeOfDay, value))
    elif prop.primitive_type == PrimitiveType.DURATION:
        return value.ToTimedelta()
    else:
        return value


def _pack_builtin_object_data_prop(prop: Property, value: Any) -> SqlPrimitive:
    """Packs the value of a BuiltinObject property for storage in Postgres."""
    if value is None:
        return None
    elif not prop.is_list:
        return _pack_builtin_object_data_prop_scalar(prop, value)
    else:
        return [_pack_builtin_object_data_prop_scalar(prop, v) for v in value]


def _pack_builtin_object_value_prop_scalar(prop: Property, value: Any) -> SqlPrimitive:
    """Packs the JSON-value-packed value of a BuiltinObject for storage in Postgres."""
    if prop.is_struct:
        return Jsonb(value)
    elif prop.primitive_type == PrimitiveType.BYTES:
        return base64.b64decode(value)
    elif prop.primitive_type == PrimitiveType.UUID:
        return to_uuid(value)
    elif prop.primitive_type == PrimitiveType.JSON:
        return Jsonb(value)
    elif prop.primitive_type == PrimitiveType.DATETIME:
        return datetime.datetime.fromisoformat(value)
    elif prop.primitive_type == PrimitiveType.DATE:
        return datetime.date.fromisoformat(value)
    elif prop.primitive_type == PrimitiveType.TIME:
        return datetime.time.fromisoformat(value)
    elif prop.primitive_type == PrimitiveType.DURATION:
        return timedelta_from_isoformat(value)
    else:
        return value


def _pack_builtin_object_value_prop(prop: Property, value: Any) -> SqlPrimitive:
    """Packs the JSON-value-packed value of a BuiltinObject for storage in Postgres."""
    if value is None:
        return None
    elif not prop.is_list:
        return _pack_builtin_object_value_prop_scalar(prop, value)
    else:
        return [_pack_builtin_object_value_prop_scalar(prop, v) for v in value]


def _unpack_builtin_object_data_prop_scalar(
    prop: Property, value_packed: Any, into: Any | None = None
) -> Any:
    """Unpacks the value of a BuiltinObject property from Postgres."""
    if value_packed is None:
        return None
    elif prop.struct_type:
        return unpack_builtin_object_data(value_packed, into=into)
    elif prop.primitive_type == PrimitiveType.UUID:
        return str(value_packed)
    elif prop.primitive_type == PrimitiveType.JSON:
        return pack_proto_json(value_packed)
    elif prop.primitive_type == PrimitiveType.DATETIME:
        ts = Timestamp()
        ts.FromDatetime(value_packed)
        return ts
    elif prop.primitive_type == PrimitiveType.DATE:
        return pack_proto_date(value_packed)
    elif prop.primitive_type == PrimitiveType.TIME:
        return pack_proto_time(value_packed)
    elif prop.primitive_type == PrimitiveType.DURATION:
        dur = Duration()
        dur.FromTimedelta(value_packed)
        return dur
    else:
        return value_packed
