import base64
import json
import textwrap
from typing import TYPE_CHECKING, Any, assert_never

from destack.language import (
    Encoding,
    NodeReference,
    PrimitiveType,
    ScalarType,
    Struct,
    TypeCardinality,
    TypeDeclaration,
)
from destack.language.core.common.type import Type
from destack.language.registry import ENUM_CLASS_BY_TYPE

if TYPE_CHECKING:
    pass

# ruff: noqa: SIM114
# pyright: reportIncompatibleVariableOverride=false

type_ = type


def generate_value(type: Type | TypeDeclaration, value: Any) -> str:
    """Generate a Typescript value literal."""

    # scalar
    if type.cardinality == TypeCardinality.SCALAR:
        return _generate_value_scalar(type, value)

    # list
    elif type.cardinality == TypeCardinality.LIST:
        assert type.value_type is not None, f"no value_type for {type!r}"
        elements_str = [
            textwrap.indent(_generate_value_scalar(type.value_type, element), "  ")
            for element in value
        ]
        return f"[\n{',\n'.join(elements_str)}\n]"

    # tuple
    elif type.cardinality == TypeCardinality.TUPLE:
        assert type.element_types is not None, f"no element_types for {type!r}"
        elements_str = [
            textwrap.indent(_generate_value_scalar(element_type, element), "  ")
            for element_type, element in zip(type.element_types, value)
        ]
        return f"[\n{',\n'.join(elements_str)}\n]"

    # map
    elif type.cardinality == TypeCardinality.MAP:
        raise ValueError(f"unsupported value type {type.cardinality!r}: {type!r}")

    #
    else:
        assert_never(type.cardinality)


def _generate_value_scalar(type: Type | TypeDeclaration, value: Any) -> str:
    """Generate a Typescript scalar value literal."""

    assert type.cardinality == TypeCardinality.SCALAR, (
        f"cannot generate value for non-scalar: {type!r}"
    )
    assert type.scalar_type is not None, f"no scalar_type for {type!r}"

    # primitive
    if type.scalar_type == ScalarType.PRIMITIVE:
        assert type.primitive_type is not None, f"no primitive_type for {type!r}"
        if type.primitive_type == PrimitiveType.NONE:
            return "null"
        elif type.primitive_type == PrimitiveType.BOOLEAN:
            return "true" if value else "false"
        elif type.primitive_type in (
            PrimitiveType.INT8,
            PrimitiveType.INT16,
            PrimitiveType.INT32,
            PrimitiveType.INT64,
            PrimitiveType.INT128,
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

    # enum
    elif type.scalar_type == ScalarType.ENUM:
        assert type.enum_type is not None, f"no enum_type for {type!r}"
        enum_cls = ENUM_CLASS_BY_TYPE[type.enum_type]
        return f"({int(value)} /* {enum_cls.__name__}.{value.name} */)"

    # struct
    elif type.scalar_type == ScalarType.STRUCT:
        assert type.struct_type is not None, f"no struct_type for {type!r}"
        assert isinstance(value, Struct), f"value is not a Struct for {type!r}: {value!r}"
        value_jsonc = value.pack(Encoding.JSONC)
        value_jsonc_str = json.dumps(value_jsonc, separators=(",", ":"))
        return f"{value.__class__.__name__}.unpack({Encoding.JSONC.value}, {value_jsonc_str})"

    # node reference
    elif type.scalar_type == ScalarType.NODE_REFERENCE:
        assert isinstance(value, NodeReference), (
            f"value is not a NodeReference for {type!r}: {value!r}"
        )
        value_jsonc = value.pack(Encoding.JSONC)
        value_jsonc_str = json.dumps(value_jsonc, separators=(",", ":"))
        return f"NodeReference.unpack({Encoding.JSONC.value}, {value_jsonc_str})"

    # node value
    elif type.scalar_type == ScalarType.NODE_VALUE:
        raise ValueError(f"unsupported value type {type.scalar_type!r}: {type!r}")

    # handle
    elif type.scalar_type == ScalarType.HANDLE:
        raise ValueError(f"unsupported value type {type.scalar_type!r}: {type!r}")

    #
    else:
        assert_never(type.scalar_type)
