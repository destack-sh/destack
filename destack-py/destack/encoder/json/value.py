import base64
from datetime import UTC, date, datetime, time
from typing import TYPE_CHECKING, Any, assert_never

from destack.language.core import (
    BuiltinObject,
    EncoderOptions,
    Json,
    NodeType,
    ObjectKind,
    PrimitiveType,
    ScalarType,
    StructType,
    Type,
    TypeCardinality,
)
from destack.language.registry import (
    ENUM_CLASS_BY_TYPE,
)
from destack.utils.log import get_logger
from destack.utils.telemetry import get_tracer
from destack.utils.time import timedelta_from_isoformat, timedelta_to_isoformat
from destack.utils.uuid import UUID

if TYPE_CHECKING:
    from destack.language import Session

    from .encoder import JsonEncoder

# pyright: reportIncompatibleVariableOverride=false


logger = get_logger(__name__)
tracer = get_tracer(__name__)
type_ = type


def pack_json(encoder: "JsonEncoder", type: Type, value: Any, options: EncoderOptions) -> Json:
    """Pack a generic typed value to a JSON object."""

    # scalar
    if type.cardinality == TypeCardinality.SCALAR:
        if value is None:
            return None
        else:
            return _pack_scalar_json(encoder, type, value, options)

    # list
    elif type.cardinality == TypeCardinality.LIST:
        if value is None:
            return None
        else:
            assert type.value_type is not None, f"no value type for {type!r}"
            packed_list: list[Any] = []
            for item in value:
                packed_list.append(_pack_scalar_json(encoder, type.value_type, item, options))
            return packed_list

    # tuple
    elif type.cardinality == TypeCardinality.TUPLE:
        if value is None:
            return None
        else:
            assert type.element_types is not None, f"no element types for {type!r}"
            packed_tuple: list[Any] = []
            for item, element_type in zip(value, type.element_types):
                packed_tuple.append(_pack_scalar_json(encoder, element_type, item, options))
            return packed_tuple

    # map
    elif type.cardinality == TypeCardinality.MAP:
        if value is None:
            return None
        else:
            assert type.key_type is not None, f"no key type for {type!r}"
            assert type.value_type is not None, f"no value type for {type!r}"
            packed_map: dict[str, Any] = {}
            for key, val in value.items():
                # always use string keys in JSON
                packed_key = str(_pack_scalar_json(encoder, type.key_type, key, options))
                packed_val = _pack_scalar_json(encoder, type.value_type, val, options)
                packed_map[packed_key] = packed_val
            return packed_map

    else:
        assert_never(type.cardinality)


def unpack_json(
    encoder: "JsonEncoder",
    type: Type,
    value: Json,
    session: "Session | None",
    options: EncoderOptions,
) -> Any:
    """Unpack a JSON object to a generic typed value."""

    # scalar
    if type.cardinality == TypeCardinality.SCALAR:
        if value is None:
            return None
        else:
            return _unpack_scalar_json(encoder, type, value, session, options)

    # list
    elif type.cardinality == TypeCardinality.LIST:
        if value is None:
            return None
        else:
            assert type.value_type is not None, f"no value type for {type!r}"
            unpacked_list: list[Any] = []
            for item in value:
                unpacked_list.append(
                    _unpack_scalar_json(encoder, type.value_type, item, session, options)
                )
            return unpacked_list

    # tuple
    elif type.cardinality == TypeCardinality.TUPLE:
        if value is None:
            return None
        else:
            assert type.element_types is not None, f"no element types for {type!r}"
            unpacked_tuple: list[Any] = []
            for item, element_type in zip(value, type.element_types):
                unpacked_tuple.append(
                    _unpack_scalar_json(encoder, element_type, item, session, options)
                )
            return unpacked_tuple

    # map
    elif type.cardinality == TypeCardinality.MAP:
        if value is None:
            return None
        else:
            assert type.key_type is not None, f"no key type for {type!r}"
            assert type.value_type is not None, f"no value type for {type!r}"
            unpacked_map = {}
            assert isinstance(value, dict), f"expected dict for map type, got {type_(value)}"
            for key, val in value.items():
                unpacked_key = (
                    _unpack_scalar_json(encoder, type.key_type, key, session, options)
                    if type.key_type
                    else key
                )
                unpacked_val = _unpack_scalar_json(encoder, type.value_type, val, session, options)
                unpacked_map[unpacked_key] = unpacked_val
            return unpacked_map

    # literal
    else:
        assert_never(type.cardinality)


def _pack_scalar_json(
    encoder: "JsonEncoder", type: Type, value: Any, options: EncoderOptions
) -> Any:
    """Pack a scalar value to JSON."""
    assert type.cardinality == TypeCardinality.SCALAR, f"expected scalar type, got {type!r}"
    assert type.scalar_type is not None, f"no scalar type for {type!r}"

    # primitive
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

    # enum
    elif type.scalar_type == ScalarType.ENUM:
        # use the enum name in CONSTANT_UPPER_CASE for JSON
        return value.name

    # node reference, node value, struct
    elif type.scalar_type in (ScalarType.NODE_REFERENCE, ScalarType.NODE_VALUE, ScalarType.STRUCT):
        assert type.struct_type is not None, f"no struct type for {type!r}"
        assert isinstance(value, BuiltinObject), (
            f"expected BuiltinObject for {type!r}, got {value!r}"
        )
        return encoder.pack_object(ObjectKind.STRUCT, type.struct_type, value, options)

    #
    else:
        assert_never(type.scalar_type)


def _unpack_scalar_json(
    encoder: "JsonEncoder",
    type: Type,
    value: Any,
    session: "Session | None",
    options: EncoderOptions,
) -> Any:
    """Unpack a scalar value from JSON."""
    assert type.cardinality == TypeCardinality.SCALAR, f"expected scalar type, got {type!r}"
    assert type.scalar_type is not None, f"no scalar type for {type!r}"

    # primitive
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

    # enum
    elif type.scalar_type == ScalarType.ENUM:
        assert type.enum_type is not None, f"no enum type for {type!r}"
        enum_cls = ENUM_CLASS_BY_TYPE[type.enum_type]
        # enum name is in CONSTANT_UPPER_CASE
        return enum_cls[value]

    # node reference
    elif type.scalar_type == ScalarType.NODE_REFERENCE:
        return encoder.unpack_object(
            ObjectKind.STRUCT, StructType.NODE_REFERENCE, value, session, options
        )

    # node value
    elif type.scalar_type == ScalarType.NODE_VALUE:
        # metatype is stored as the enum name
        node_type = NodeType[value["metatype"]]
        return encoder.unpack_object(ObjectKind.NODE, node_type, value, session, options)

    # struct
    elif type.scalar_type == ScalarType.STRUCT:
        assert type.struct_type is not None, f"no struct type for {type!r}"
        return encoder.unpack_object(ObjectKind.STRUCT, type.struct_type, value, session, options)

    #
    else:
        assert_never(type.scalar_type)
