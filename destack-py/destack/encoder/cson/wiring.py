import base64
from datetime import UTC, date, datetime, time
from typing import TYPE_CHECKING, Any, assert_never

import structlog
from opentelemetry import trace

from destack.language.registry import (
    ENUM_CLASS_BY_TYPE,
    NODE_CLASS_BY_TYPE,
    STRUCT_CLASS_BY_TYPE,
)
from destack.utils.time import timedelta_from_isoformat, timedelta_to_isoformat
from destack.utils.uuid import UUID

from ...language.core.builtin import (
    BuiltinObject,
    Cson,
    Encoding,
    NodeReference,
    NodeType,
    PrimitiveType,
)
from ...language.core.common.type import ScalarType, Type, TypeCardinality

if TYPE_CHECKING:
    from destack.language import Session


# pyright: reportIncompatibleVariableOverride=false


logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)
type_ = type


def pack_cson(value: Any, type: Type) -> Cson:
    """Pack a generic typed value to a CSON object."""
    if type.cardinality == TypeCardinality.SCALAR:
        return _pack_scalar_cson(value, type)
    elif type.cardinality == TypeCardinality.LIST:
        if not value:
            return []
        packed_list: list[Cson] = []
        for item in value:
            packed_list.append(_pack_scalar_cson(item, type))
        return packed_list
    elif type.cardinality == TypeCardinality.MAP:
        if not value:
            return {}
        assert type.key_type is not None, f"no key type for {type!r}"
        packed_map: dict[str, Cson] = {}
        for key, val in value.items():
            packed_key = _pack_scalar_cson(key, type.key_type)
            packed_val = _pack_scalar_cson(val, type)
            packed_map[str(packed_key)] = packed_val
        return packed_map
    else:
        assert_never(type.cardinality)


def unpack_cson(
    value: Cson,
    type: Type,
    session: "Session | None",
) -> Any:
    """Unpack a CSON object to a generic typed value."""
    if type.cardinality == TypeCardinality.SCALAR:
        return _unpack_scalar_cson(value, type, session)
    elif type.cardinality == TypeCardinality.LIST:
        if value is None:
            return []
        unpacked_list = []
        for item in value:
            unpacked_list.append(_unpack_scalar_cson(item, type, session))
        return unpacked_list
    elif type.cardinality == TypeCardinality.MAP:
        if value is None:
            return {}
        unpacked_map = {}
        for key, val in value.items():
            unpacked_key = (
                _unpack_scalar_cson(key, type.key_type, session) if type.key_type else key
            )
            unpacked_val = _unpack_scalar_cson(val, type, session)
            unpacked_map[unpacked_key] = unpacked_val
        return unpacked_map
    else:
        assert_never(type.cardinality)


def _pack_scalar_cson(value: Any, type: Type) -> Cson:
    """Pack a scalar value to CSON."""
    if type.scalar_type == ScalarType.PRIMITIVE:
        if type.primitive_type == PrimitiveType.BYTES:
            return base64.b64encode(value).decode()
        elif type.primitive_type == PrimitiveType.UUID:
            return str(value)
        elif type.primitive_type == PrimitiveType.DATETIME:
            return value.astimezone(UTC).isoformat()
        elif type.primitive_type == PrimitiveType.DATE:
            return value.isoformat()
        elif type.primitive_type == PrimitiveType.TIME:
            return value.astimezone(UTC).replace(tzinfo=None).isoformat()
        elif type.primitive_type == PrimitiveType.DURATION:
            return timedelta_to_isoformat(value)
        elif type.primitive_type in (PrimitiveType.INT16, PrimitiveType.INT32, PrimitiveType.INT64):
            return float(value)  # cast ints to CSON floats
        else:
            return value  # as is
    elif type.scalar_type == ScalarType.ENUM:
        return value.value
    elif type.scalar_type in (ScalarType.NODE_REFERENCE, ScalarType.NODE_VALUE, ScalarType.STRUCT):
        assert isinstance(value, BuiltinObject), (
            f"expected BuiltinObject for {type!r}, got {value!r}"
        )
        return value.pack(Encoding.CSON)
    else:
        assert_never(type.scalar_type)


def _unpack_scalar_cson(
    value: Cson,
    type: Type,
    session: "Session | None",
) -> Any:
    """Unpack a scalar value from CSON."""
    if type.scalar_type == ScalarType.PRIMITIVE:
        if type.primitive_type == PrimitiveType.BYTES:
            return base64.b64decode(value)
        elif type.primitive_type == PrimitiveType.UUID:
            return UUID(value)
        elif type.primitive_type == PrimitiveType.DATETIME:
            return datetime.fromisoformat(value).astimezone(UTC)
        elif type.primitive_type == PrimitiveType.DATE:
            return date.fromisoformat(value)
        elif type.primitive_type == PrimitiveType.TIME:
            return time.fromisoformat(value).replace(tzinfo=None)
        elif type.primitive_type == PrimitiveType.DURATION:
            return timedelta_from_isoformat(value)
        elif type.primitive_type in (PrimitiveType.INT16, PrimitiveType.INT32, PrimitiveType.INT64):
            return int(value)  # cast CSON floats to ints
        else:
            return value
    elif type.scalar_type == ScalarType.ENUM:
        assert type.enum_type is not None, f"no enum type for {type!r}"
        enum_cls = ENUM_CLASS_BY_TYPE[type.enum_type]
        return enum_cls(int(value))
    elif type.scalar_type == ScalarType.NODE_REFERENCE:
        return NodeReference.unpack(Encoding.CSON, value, session)
    elif type.scalar_type == ScalarType.NODE_VALUE:
        node_type = NodeType(value["1"])
        node_cls = NODE_CLASS_BY_TYPE[node_type]
        return node_cls.unpack(Encoding.CSON, value, session)
    elif type.scalar_type == ScalarType.STRUCT:
        assert type.struct_type is not None, f"no struct type for {type!r}"
        struct_cls = STRUCT_CLASS_BY_TYPE[type.struct_type]
        return struct_cls.unpack(Encoding.CSON, value, session)
    else:
        assert_never(type.scalar_type)
