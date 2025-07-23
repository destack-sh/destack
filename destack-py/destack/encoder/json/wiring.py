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
    Encoding,
    Json,
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


def pack_json(value: Any, type: Type) -> Json:
    """Pack a generic typed value to a JSON object."""
    if type.cardinality == TypeCardinality.SCALAR:
        return _pack_scalar_json(value, type)
    elif type.cardinality == TypeCardinality.LIST:
        if not value:
            return []
        packed_list: list[Any] = []
        for item in value:
            packed_list.append(_pack_scalar_json(item, type))
        return packed_list
    elif type.cardinality == TypeCardinality.MAP:
        if not value:
            return {}
        assert type.key_type is not None, f"no key type for {type!r}"
        packed_map: dict[str, Any] = {}
        for key, val in value.items():
            # always use string keys in JSON
            packed_key = str(_pack_scalar_json(key, type.key_type))
            packed_val = _pack_scalar_json(val, type)
            packed_map[packed_key] = packed_val
        return packed_map
    else:
        assert_never(type.cardinality)


def unpack_json(
    value: Json,
    type: Type,
    session: "Session | None",
) -> Any:
    """Unpack a JSON object to a generic typed value."""
    if type.cardinality == TypeCardinality.SCALAR:
        return _unpack_scalar_json(value, type, session)
    elif type.cardinality == TypeCardinality.LIST:
        if value is None:
            return []
        unpacked_list = []
        for item in value:
            unpacked_list.append(_unpack_scalar_json(item, type, session))
        return unpacked_list
    elif type.cardinality == TypeCardinality.MAP:
        if value is None:
            return {}
        unpacked_map = {}
        assert isinstance(value, dict), f"expected dict for map type, got {type_(value)}"
        for key, val in value.items():
            unpacked_key = (
                _unpack_scalar_json(key, type.key_type, session) if type.key_type else key
            )
            unpacked_val = _unpack_scalar_json(val, type, session)
            unpacked_map[unpacked_key] = unpacked_val
        return unpacked_map
    else:
        assert_never(type.cardinality)


def _pack_scalar_json(value: Any, type: Type) -> Any:
    """Pack a scalar value to JSON."""
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
        else:
            return value  # as is
    elif type.scalar_type == ScalarType.ENUM:
        # use the enum name in CONSTANT_UPPER_CASE for JSON
        return value.name
    elif type.scalar_type in (ScalarType.NODE_REFERENCE, ScalarType.NODE_VALUE, ScalarType.STRUCT):
        assert isinstance(value, BuiltinObject), (
            f"expected BuiltinObject for {type!r}, got {value!r}"
        )
        return value.pack(Encoding.JSON)
    else:
        assert_never(type.scalar_type)


def _unpack_scalar_json(value: Any, type: Type, session: "Session | None") -> Any:
    """Unpack a scalar value from JSON."""
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
        else:
            return value
    elif type.scalar_type == ScalarType.ENUM:
        assert type.enum_type is not None, f"no enum type for {type!r}"
        enum_cls = ENUM_CLASS_BY_TYPE[type.enum_type]
        # enum name is in CONSTANT_UPPER_CASE
        return enum_cls[value]
    elif type.scalar_type == ScalarType.NODE_REFERENCE:
        return NodeReference.unpack(Encoding.JSON, value, session)
    elif type.scalar_type == ScalarType.NODE_VALUE:
        # metatype is stored as the enum name
        node_type = NodeType[value["metatype"]]
        node_cls = NODE_CLASS_BY_TYPE[node_type]
        return node_cls.unpack(Encoding.JSON, value, session)
    elif type.scalar_type == ScalarType.STRUCT:
        assert type.struct_type is not None, f"no struct type for {type!r}"
        struct_cls = STRUCT_CLASS_BY_TYPE[type.struct_type]
        return struct_cls.unpack(Encoding.JSON, value, session)
    else:
        assert_never(type.scalar_type)
