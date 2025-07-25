import base64
import json
import textwrap
from typing import TYPE_CHECKING, Any, assert_never

from destack.language import (
    Encoding,
    NodeReference,
    PrimitiveType,
    PropertyDeclaration,
    ScalarType,
    Struct,
    Type,
    TypeCardinality,
    TypeDeclaration,
)
from destack.language.registry import ENUM_CLASS_BY_TYPE
from destack.utils.log import get_logger
from destack.utils.telemetry import get_tracer

if TYPE_CHECKING:
    pass

# ruff: noqa: SIM114
# pyright: reportIncompatibleVariableOverride=false

logger = get_logger(__name__)
tracer = get_tracer(__name__)
type_ = type


def generate_value(type: Type | TypeDeclaration | PropertyDeclaration, value: Any) -> str:
    """Generate a Typescript value literal."""
    if type.cardinality == TypeCardinality.SCALAR:
        return _generate_value_scalar(type, value)
    elif type.cardinality == TypeCardinality.LIST:
        elements_str = [
            textwrap.indent(_generate_value_scalar(type, element), "  ") for element in value
        ]
        return f"[\n{',\n'.join(elements_str)}\n]"
    elif type.cardinality == TypeCardinality.MAP:
        raise ValueError(f"unsupported value type {type.cardinality!r}: {type!r}")
    else:
        assert_never(type.cardinality)


def _generate_value_scalar(type: Type | TypeDeclaration | PropertyDeclaration, value: Any) -> str:
    """Generate a Typescript scalar value literal."""
    if type.scalar_type == ScalarType.PRIMITIVE:
        assert type.primitive_type is not None, f"no primitive_type for {type!r}"
        if type.primitive_type == PrimitiveType.NONE:
            return "null"
        elif type.primitive_type == PrimitiveType.BOOLEAN:
            return "true" if value else "false"
        elif type.primitive_type in (
            PrimitiveType.SINT8,
            PrimitiveType.SINT16,
            PrimitiveType.SINT32,
            PrimitiveType.SINT64,
            PrimitiveType.SINT128,
        ):
            return str(value)
        elif type.primitive_type in (
            PrimitiveType.UINT8,
            PrimitiveType.UINT16,
            PrimitiveType.UINT32,
            PrimitiveType.UINT64,
            PrimitiveType.UINT128,
        ):
            return str(value)
        elif type.primitive_type in (
            PrimitiveType.FLOAT16,
            PrimitiveType.FLOAT32,
            PrimitiveType.FLOAT64,
        ):
            return str(value)
        elif type.primitive_type == PrimitiveType.DATETIME:
            return f"Temporal.Instant.from(\"{value}\").toZonedDateTimeISO('UTC')"
        elif type.primitive_type == PrimitiveType.DATE:
            return f'Temporal.PlainDate.from("{value}")'
        elif type.primitive_type == PrimitiveType.TIME:
            return f'Temporal.PlainTime.from("{value}")'
        elif type.primitive_type == PrimitiveType.DURATION:
            return f'Temporal.Duration.from("{value}")'
        elif type.primitive_type == PrimitiveType.STRING:
            return f'"{value}"'
        elif type.primitive_type == PrimitiveType.UUID:
            return f'"{value}"'
        elif type.primitive_type == PrimitiveType.BYTES:
            return f"base64Decode({base64.b64encode(value).decode()})"
        elif type.primitive_type == PrimitiveType.JSON:
            return json.dumps(value)
        else:
            assert_never(type.primitive_type)
    elif type.scalar_type == ScalarType.ENUM:
        assert type.enum_type is not None, f"no enum_type for {type!r}"
        enum_cls = ENUM_CLASS_BY_TYPE[type.enum_type]
        return f"({int(value)} /* {enum_cls.__name__}.{value.name} */)"
    elif type.scalar_type == ScalarType.STRUCT:
        assert type.struct_type is not None, f"no struct_type for {type!r}"
        assert isinstance(value, Struct), f"value is not a Struct for {type!r}: {value!r}"
        value_cson = value.pack(Encoding.CSON)
        value_cson_str = json.dumps(value_cson, separators=(",", ":"))
        return f"{value.__class__.__name__}.unpack({Encoding.CSON.value}, {value_cson_str})"
    elif type.scalar_type == ScalarType.NODE_REFERENCE:
        assert isinstance(value, NodeReference), (
            f"value is not a NodeReference for {type!r}: {value!r}"
        )
        value_cson = value.pack(Encoding.CSON)
        value_cson_str = json.dumps(value_cson, separators=(",", ":"))
        return f"NodeReference.unpack({Encoding.CSON.value}, {value_cson_str})"
    elif type.scalar_type == ScalarType.NODE_VALUE:
        raise ValueError(f"unsupported value type {type.scalar_type!r}: {type!r}")
    else:
        assert_never(type.scalar_type)
