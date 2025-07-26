import base64
from datetime import UTC, date, datetime, time
from typing import TYPE_CHECKING, Any, assert_never

from destack.language.core import (
    BuiltinObject,
    Cson,
    Encoding,
    NodeReference,
    NodeType,
    PrimitiveType,
    ScalarType,
    Type,
    TypeCardinality,
)
from destack.language.registry import (
    ENUM_CLASS_BY_TYPE,
    NODE_CLASS_BY_TYPE,
    STRUCT_CLASS_BY_TYPE,
)
from destack.utils.log import get_logger
from destack.utils.telemetry import get_tracer
from destack.utils.time import timedelta_from_isoformat, timedelta_to_isoformat
from destack.utils.uuid import UUID

if TYPE_CHECKING:
    from destack.language import Session


# pyright: reportIncompatibleVariableOverride=false


logger = get_logger(__name__)
tracer = get_tracer(__name__)
type_ = type


def pack_cson(type: Type, value: Any) -> Cson:
    """Pack a generic typed value to a CSON object."""

    # scalar
    if type.cardinality == TypeCardinality.SCALAR:
        if value is None:
            return None
        else:
            return _pack_scalar_cson(type, value)

    # list
    elif type.cardinality == TypeCardinality.LIST:
        if value is None:
            return None
        else:
            assert type.value_type is not None, f"no value type for {type!r}"
            packed_list: list[Cson] = []
            for item in value:
                packed_list.append(_pack_scalar_cson(type.value_type, item))
            return packed_list

    # tuple
    elif type.cardinality == TypeCardinality.TUPLE:
        raise NotImplementedError(f"cannot pack tuple: {type!r}")

    # map
    elif type.cardinality == TypeCardinality.MAP:
        if value is None:
            return None
        else:
            assert type.key_type is not None, f"no key type for {type!r}"
            assert type.value_type is not None, f"no value type for {type!r}"
            packed_map: dict[str, Cson] = {}
            for key, val in value.items():
                packed_key = _pack_scalar_cson(type.key_type, key)
                packed_val = _pack_scalar_cson(type.value_type, val)
                packed_map[str(packed_key)] = packed_val
            return packed_map

    else:
        assert_never(type.cardinality)


def unpack_cson(type: Type, value: Cson, session: "Session | None") -> Any:
    """Unpack a CSON object to a generic typed value."""

    # scalar
    if type.cardinality == TypeCardinality.SCALAR:
        if value is None:
            return None
        else:
            return _unpack_scalar_cson(type, value, session)

    # list
    elif type.cardinality == TypeCardinality.LIST:
        if value is None:
            return None
        else:
            assert type.value_type is not None, f"no value type for {type!r}"
            unpacked_list = []
            for item in value:
                unpacked_list.append(_unpack_scalar_cson(type.value_type, item, session))
            return unpacked_list

    # tuple
    elif type.cardinality == TypeCardinality.TUPLE:
        raise NotImplementedError(f"cannot unpack tuple: {type!r}")

    # map
    elif type.cardinality == TypeCardinality.MAP:
        if value is None:
            return None
        else:
            assert type.key_type is not None, f"no key type for {type!r}"
            assert type.value_type is not None, f"no value type for {type!r}"
            unpacked_map = {}
            for key, val in value.items():
                unpacked_key = (
                    _unpack_scalar_cson(type.key_type, key, session) if type.key_type else key
                )
                unpacked_val = _unpack_scalar_cson(type.value_type, val, session)
                unpacked_map[unpacked_key] = unpacked_val
            return unpacked_map
    else:
        assert_never(type.cardinality)


def _pack_scalar_cson(type: Type, value: Any) -> Cson:
    """Pack a scalar value to CSON."""
    assert type.scalar_type is not None, f"no scalar type for {type!r}"
    if type.scalar_type == ScalarType.PRIMITIVE:
        assert type.primitive_type is not None, f"no primitive type for {type!r}"
        if type.primitive_type == PrimitiveType.NONE:
            return None
        elif type.primitive_type == PrimitiveType.BOOLEAN:
            return value
        elif (
            type.primitive_type
            in (
                PrimitiveType.INT8,
                PrimitiveType.INT16,
                PrimitiveType.INT32,
                PrimitiveType.INT64,
                PrimitiveType.INT128,
            )
            or type.primitive_type
            in (
                PrimitiveType.UINT8,
                PrimitiveType.UINT16,
                PrimitiveType.UINT32,
                PrimitiveType.UINT64,
                PrimitiveType.UINT128,
            )
            or type.primitive_type
            in (
                PrimitiveType.FLOAT16,
                PrimitiveType.FLOAT32,
                PrimitiveType.FLOAT64,
            )
        ):
            return float(value)
        elif type.primitive_type == PrimitiveType.DATETIME:
            return value.astimezone(UTC).isoformat()
        elif type.primitive_type == PrimitiveType.DATE:
            return value.isoformat()
        elif type.primitive_type == PrimitiveType.TIME:
            return value.astimezone(UTC).replace(tzinfo=None).isoformat()
        elif type.primitive_type == PrimitiveType.DURATION:
            return timedelta_to_isoformat(value)
        elif type.primitive_type == PrimitiveType.STRING:
            return value
        elif type.primitive_type == PrimitiveType.UUID:
            return str(value)
        elif type.primitive_type == PrimitiveType.BYTES:
            return base64.b64encode(value).decode()
        elif type.primitive_type == PrimitiveType.JSON:
            return value
        else:
            assert_never(type.primitive_type)
    elif type.scalar_type == ScalarType.ENUM:
        return value.value
    elif type.scalar_type in (ScalarType.NODE_REFERENCE, ScalarType.NODE_VALUE, ScalarType.STRUCT):
        assert isinstance(value, BuiltinObject), (
            f"expected BuiltinObject for {type!r}, got {value!r}"
        )
        return value.pack(Encoding.CSON)
    elif type.scalar_type == ScalarType.LITERAL:
        raise NotImplementedError(f"cannot pack literal: {type!r}")
    elif type.scalar_type == ScalarType.UNION:
        raise NotImplementedError(f"cannot pack union: {type!r}")
    else:
        assert_never(type.scalar_type)


def _unpack_scalar_cson(type: Type, value: Cson, session: "Session | None") -> Any:
    """Unpack a scalar value from CSON."""
    assert type.scalar_type is not None, f"no scalar type for {type!r}"
    if type.scalar_type == ScalarType.PRIMITIVE:
        assert type.primitive_type is not None, f"no primitive type for {type!r}"
        if type.primitive_type == PrimitiveType.NONE:
            return None
        elif type.primitive_type == PrimitiveType.BOOLEAN:
            return value
        elif type.primitive_type in (
            PrimitiveType.INT8,
            PrimitiveType.INT16,
            PrimitiveType.INT32,
            PrimitiveType.INT64,
            PrimitiveType.INT128,
        ) or type.primitive_type in (
            PrimitiveType.UINT8,
            PrimitiveType.UINT16,
            PrimitiveType.UINT32,
            PrimitiveType.UINT64,
            PrimitiveType.UINT128,
        ):
            return int(value)
        elif type.primitive_type in (
            PrimitiveType.FLOAT16,
            PrimitiveType.FLOAT32,
            PrimitiveType.FLOAT64,
        ):
            return float(value)
        elif type.primitive_type == PrimitiveType.DATETIME:
            return datetime.fromisoformat(value).astimezone(UTC)
        elif type.primitive_type == PrimitiveType.DATE:
            return date.fromisoformat(value)
        elif type.primitive_type == PrimitiveType.TIME:
            return time.fromisoformat(value).replace(tzinfo=None)
        elif type.primitive_type == PrimitiveType.DURATION:
            return timedelta_from_isoformat(value)
        elif type.primitive_type == PrimitiveType.STRING:
            return value
        elif type.primitive_type == PrimitiveType.UUID:
            return UUID(value)
        elif type.primitive_type == PrimitiveType.BYTES:
            return base64.b64decode(value)
        elif type.primitive_type == PrimitiveType.JSON:
            return value
        else:
            assert_never(type.primitive_type)
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
        return struct_cls.unpack(Encoding.CSON, value, session)  #
    elif type.scalar_type == ScalarType.LITERAL:
        raise NotImplementedError(f"cannot unpack literal: {type!r}")
    elif type.scalar_type == ScalarType.UNION:
        raise NotImplementedError(f"cannot unpack union: {type!r}")
    else:
        assert_never(type.scalar_type)
