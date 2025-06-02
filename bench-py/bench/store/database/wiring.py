import base64
import uuid
from collections.abc import Sequence
from datetime import date, datetime, time
from types import NoneType
from typing import Any, assert_never

import asyncpg
import orjson

from bench.language import (
    IsInBench,
    Json,
    NodeType,
    PrimitiveType,
    ScalarType,
    Type,
    TypeCardinality,
    Value,
    expand_node_types,
)
from bench.language.registry import NODE_CLASS_BY_TYPE
from bench.utils.time import timedelta_from_isoformat, timedelta_to_isoformat

from .core import DatabaseTable

# nocheckin :Performance: pregenerate Node row pack/unpack


def pack_node_value_to_row(table: DatabaseTable, value: Value) -> Sequence[Any]:
    """Pack a Node Value into an asyncpg row (tuple)."""
    type = value.type
    assert type.scalar_type == ScalarType.NODE_VALUE, f"unexpected node value: {value!r}"
    node_type = table.node_type
    assert node_type is not None, f"no node type for {table!r}"
    assert node_type == type.node_type, f"node type mismatch: {node_type!r} != {type.node_type!r}"
    node_cls = NODE_CLASS_BY_TYPE[node_type]

    value_packed = value.value
    values_packed: list[Any] = []
    for prop in node_cls.__properties_in_order__:
        if not prop.is_stored:
            continue
        prop_key = str(prop.id)
        prop_value = value_packed.get(prop_key)

        if prop.scalar_type == "node_reference":
            assert isinstance(prop_value, (dict, NoneType)), (
                f"unexpected value for {prop!r}: {prop_value!r}"
            )
            node_types = expand_node_types(prop.node_types or ())
            has_node_type = len(node_types) > 1
            has_bench_id = prop.node_bench_from is None and any(
                issubclass(NODE_CLASS_BY_TYPE[node_type], IsInBench) for node_type in node_types
            )
            has_definition_id = (
                prop.node_is_customizable and NodeType.CUSTOM_NODE_INSTANCE in node_types
            )
            # id
            if prop_value is not None and "32" in prop_value:
                values_packed.append(uuid.UUID(prop_value["32"]))
            else:
                values_packed.append(None)
            # node_type
            if has_node_type:
                if prop_value is not None and "31" in prop_value:
                    values_packed.append(int(prop_value["31"]))
                else:
                    values_packed.append(None)
            # bench_id
            if has_bench_id:
                if prop_value is not None and "34" in prop_value:
                    values_packed.append(uuid.UUID(prop_value["34"]))
                else:
                    values_packed.append(None)
            # definition_id
            if has_definition_id:
                if prop_value is not None and "35" in prop_value:
                    values_packed.append(uuid.UUID(prop_value["35"]))
                else:
                    values_packed.append(None)
        else:
            if prop_value is not None:
                prop_value_packed = pack_node_value(prop.type, prop_value)
                values_packed.append(prop_value_packed)
            else:
                values_packed.append(None)
            if prop.is_variable:
                ...
    assert len(values_packed) == len(table._columns_by_name), (
        f"unexpected values: {len(values_packed)} != {len(table._columns_by_name)} in {table!r} ({values_packed!r} for {list(table._columns_by_name.keys())!r})"
    )

    return values_packed


def unpack_row_to_node_value(table: DatabaseTable, row: asyncpg.Record) -> Value:
    """Unpack an asyncpg row into a Node Value."""
    type = Type(cardinality=TypeCardinality.SCALAR, scalar_type=ScalarType.NODE_VALUE)
    value_packed: dict[str, Any] = {}

    raise NotImplementedError(row)

    value = Value(type=type, value=value_packed)
    return value


def _pack_node_value_scalar(type: Type, value: Json) -> Any:
    assert type.scalar_type != ScalarType.NODE_REFERENCE, f"unhandled node ref: {type!r}"
    if type.scalar_type == ScalarType.PRIMITIVE:
        if type.primitive_type == PrimitiveType.BYTES:
            return base64.b64decode(value)
        elif type.primitive_type == PrimitiveType.UUID:
            return uuid.UUID(value)
        elif type.primitive_type == PrimitiveType.DATETIME:
            return datetime.fromisoformat(value).replace(tzinfo=None)
        elif type.primitive_type == PrimitiveType.DATE:
            return date.fromisoformat(value)
        elif type.primitive_type == PrimitiveType.TIME:
            return time.fromisoformat(value).replace(tzinfo=None)
        elif type.primitive_type == PrimitiveType.DURATION:
            return timedelta_from_isoformat(value)
        else:
            return value
    elif type.scalar_type == ScalarType.ENUM:
        return value
    elif type.scalar_type == ScalarType.NODE_VALUE:  # noqa: SIM114
        return orjson.dumps(value)  # keep json
    elif type.scalar_type == ScalarType.STRUCT:
        return orjson.dumps(value)  # keep json
    else:
        assert_never(type.scalar_type)


def pack_node_value(type: Type, value_packed: Json) -> Any:
    if type.cardinality == TypeCardinality.SCALAR:
        return _pack_node_value_scalar(type, value_packed)
    elif type.cardinality == TypeCardinality.LIST:
        return [_pack_node_value_scalar(type, v) for v in value_packed]
    elif type.cardinality == TypeCardinality.MAP:
        return orjson.dumps(value_packed)  # keep json
    else:
        assert_never(type.cardinality)


def _unpack_node_value_scalar(type: Type, value: Any) -> Json:
    assert type.scalar_type != ScalarType.NODE_REFERENCE, f"unhandled node ref: {type!r}"
    if type.scalar_type == ScalarType.PRIMITIVE:
        if type.primitive_type == PrimitiveType.BYTES:
            return base64.b64encode(value).decode()
        elif type.primitive_type == PrimitiveType.UUID:
            return str(value)
        elif (
            type.primitive_type == PrimitiveType.DATETIME
            or type.primitive_type == PrimitiveType.DATE
            or type.primitive_type == PrimitiveType.TIME
        ):
            return value.isoformat()
        elif type.primitive_type == PrimitiveType.DURATION:
            return timedelta_to_isoformat(value)
        else:
            return value
    elif type.scalar_type == ScalarType.ENUM:
        return value
    elif type.scalar_type == ScalarType.NODE_VALUE:  # noqa: SIM114
        return orjson.loads(value)  # keep json
    elif type.scalar_type == ScalarType.STRUCT:
        return orjson.loads(value)  # keep json
    else:
        assert_never(type.scalar_type)


def unpack_node_value(type: Type, value: Any) -> Json:
    if type.cardinality == TypeCardinality.SCALAR:
        return _unpack_node_value_scalar(type, value)
    elif type.cardinality == TypeCardinality.LIST:
        return [_unpack_node_value_scalar(type, v) for v in value]
    elif type.cardinality == TypeCardinality.MAP:
        return orjson.loads(value)  # keep json
    else:
        assert_never(type.cardinality)
