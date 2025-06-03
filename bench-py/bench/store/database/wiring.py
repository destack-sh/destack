import base64
import textwrap
import uuid
from collections.abc import Sequence
from datetime import date, datetime, time, timedelta
from typing import Any, Callable, assert_never

import asyncpg
import orjson
import pytz

from bench.language import (
    IsInBench,
    Json,
    Node,
    NodeType,
    PrimitiveType,
    Property,
    ScalarType,
    StructType,
    Type,
    TypeCardinality,
    Value,
    expand_node_types,
)
from bench.language.registry import NODE_CLASS_BY_TYPE
from bench.utils.code import exec_
from bench.utils.time import timedelta_from_isoformat, timedelta_to_isoformat

from .core import DatabaseTable

# nocheckin :Performance: pregenerate Node row pack/unpack


NODE_ROW_PACK: dict[NodeType, Callable[[Json], Sequence[Any]]] = {}
NODE_ROW_UNPACK: dict[NodeType, Callable[[asyncpg.Record], Json]] = {}


def generate_pack_row(node_cls: type[Node]) -> tuple[str, dict[str, Any]]:
    """Generate a function to pack a Node Value into a row tuple."""
    pack_impl_str = textwrap.indent(_generate_node_row_pack(node_cls), "    ")
    unpack_impl_str = textwrap.indent(_generate_node_row_unpack(node_cls), "    ")

    impl_str = f"""\
def _pack_{node_cls.__name__}_row(node_value: "Json") -> Sequence[Any]:
{pack_impl_str}

def _unpack_{node_cls.__name__}_row(row: "asyncpg.Record") -> "Json":
{unpack_impl_str}
"""

    return impl_str, {
        "Any": Any,
        "Sequence": Sequence,
        "base64": base64,
        "orjson": orjson,
        "uuid": uuid,
        "pytz": pytz,
        "datetime": datetime,
        "date": date,
        "time": time,
        "timedelta": timedelta,
        "timedelta_from_isoformat": timedelta_from_isoformat,
        "timedelta_to_isoformat": timedelta_to_isoformat,
    }


def _generate_pack_scalar_value(prop: "Property", value_expr: str) -> str:
    """Generate code to pack a scalar value for a property."""
    assert prop.scalar_type != "node_reference", f"unhandled node ref: {prop!r}"
    if prop.scalar_type == "primitive":
        if prop.primitive_type == PrimitiveType.BYTES:
            return f"base64.b64decode({value_expr})"
        elif prop.primitive_type == PrimitiveType.UUID:
            return f"uuid.UUID({value_expr})"
        elif prop.primitive_type == PrimitiveType.DATETIME:
            return f"datetime.fromisoformat({value_expr}).replace(tzinfo=None)"
        elif prop.primitive_type == PrimitiveType.DATE:
            return f"date.fromisoformat({value_expr})"
        elif prop.primitive_type == PrimitiveType.TIME:
            return f"time.fromisoformat({value_expr}).replace(tzinfo=None)"
        elif prop.primitive_type == PrimitiveType.DURATION:
            return f"timedelta_from_isoformat({value_expr})"
        else:
            return value_expr
    elif prop.scalar_type == "enum":
        return value_expr
    elif prop.scalar_type == "node_value" or prop.scalar_type == "struct":
        return f"orjson.dumps({value_expr})"
    else:
        assert_never(prop.scalar_type)


def _generate_unpack_scalar_value(prop: "Property", value_expr: str) -> str:
    """Generate code to unpack a scalar value for a property."""
    assert prop.scalar_type != "node_reference", f"unhandled node ref: {prop!r}"
    if prop.scalar_type == "primitive":
        if prop.primitive_type == PrimitiveType.BYTES:
            return f"base64.b64encode({value_expr}).decode()"
        elif prop.primitive_type == PrimitiveType.UUID:
            return f"str({value_expr})"
        elif prop.primitive_type in (
            PrimitiveType.DATETIME,
            PrimitiveType.DATE,
            PrimitiveType.TIME,
        ):
            return f"{value_expr}.isoformat()"
        elif prop.primitive_type == PrimitiveType.DURATION:
            return f"timedelta_to_isoformat({value_expr})"
        else:
            return value_expr
    elif prop.scalar_type == "enum":
        return value_expr
    elif prop.scalar_type == "node_value" or prop.scalar_type == "struct":
        return f"orjson.loads({value_expr})"
    else:
        assert_never(prop.scalar_type)


def _generate_column_pack(prop: "Property") -> str:
    """Generate code to pack a property value into row columns."""
    assert prop.id is not None, f"no id for {prop!r}"

    if prop.scalar_type == "node_reference":
        # node references fan out to multiple columns
        assert prop.cardinality == "scalar", f"non-scalar node ref: {prop!r}"
        if prop.is_required:
            pack_lines = [f"_node_ref = node_value['{prop.id}']"]
            pack_lines.append("row_values.append(uuid.UUID(_node_ref['32']))  # id")
            if prop.node_has_type:
                pack_lines.append("row_values.append(_node_ref['31'])  # node_type")
            if prop.node_has_bench:
                pack_lines.append(
                    "row_values.append(uuid.UUID(_node_ref['34']) if _node_ref.get('34') else None)  # bench_id"
                )
            if prop.node_has_definition:
                pack_lines.append(
                    "row_values.append(uuid.UUID(_node_ref['35']) if _node_ref.get('35') else None)  # definition_id"
                )
            return "\n".join(pack_lines)
        else:
            pack_lines = [f"if (_node_ref := node_value.get('{prop.id}')) is not None:"]
            pack_lines.append("    row_values.append(uuid.UUID(_node_ref['32']))  # id")
            if prop.node_has_type:
                pack_lines.append("    row_values.append(_node_ref['31'])  # node_type")
            if prop.node_has_bench:
                pack_lines.append(
                    "    row_values.append(uuid.UUID(_node_ref['34']) if _node_ref.get('34') else None)  # bench_id"
                )
            if prop.node_has_definition:
                pack_lines.append(
                    "    row_values.append(uuid.UUID(_node_ref['35']) if _node_ref.get('35') else None)  # definition_id"
                )
            pack_lines.append("else:")
            pack_lines.append("    row_values.append(None)  # id")
            if prop.node_has_type:
                pack_lines.append("    row_values.append(None)  # node_type")
            if prop.node_has_bench:
                pack_lines.append("    row_values.append(None)  # bench_id")
            if prop.node_has_definition:
                pack_lines.append("    row_values.append(None)  # definition_id")
            return "\n".join(pack_lines)
    else:
        # regular properties
        if prop.cardinality == "scalar":
            pack_expr = _generate_pack_scalar_value(prop, f"node_value['{prop.id}']")
            if prop.is_required:
                return f"row_values.append({pack_expr})"
            else:
                pack_expr_opt = _generate_pack_scalar_value(prop, "_value")
                return f"""if (_value := node_value.get('{prop.id}')) is not None:
    row_values.append({pack_expr_opt})
else:
    row_values.append(None)"""
        elif prop.cardinality == "list":
            pack_expr = _generate_pack_scalar_value(prop, "v")
            return f"""_value = node_value.get('{prop.id}')
if _value is not None:
    row_values.append([{pack_expr} for v in _value])
else:
    row_values.append(None)"""
        elif prop.cardinality == "map":
            # Maps are stored as JSON
            return f"""_value = node_value.get('{prop.id}')
row_values.append(orjson.dumps(_value) if _value else None)"""
        else:
            assert_never(prop.cardinality)


def _generate_column_unpack(prop: "Property") -> str:
    """Generate code to unpack row columns into a property value."""
    assert prop.id is not None, f"no id for {prop!r}"

    if prop.scalar_type == "node_reference":
        # node references fan out from multiple columns
        assert prop.cardinality == "scalar", f"non-scalar node ref: {prop!r}"
        node_types = expand_node_types(prop.node_types or ())
        unpack_lines = [
            f"if (_node_id := row['{prop.name}_id']) is not None:",
            "    _node_ref = {",
            f"        '1': {StructType.NODE_REFERENCE.value},",
            "        '32': str(_node_id),",
            "    }",
        ]
        # node_type
        if prop.node_has_type:
            unpack_lines.append(f"    _node_ref['31'] = row['{prop.name}_type']")
        else:
            assert prop.node_types, f"no node types for {prop!r}"
            node_type = prop.node_types[0]
            assert isinstance(node_type, NodeType), f"unexpected node property: {prop!r}"
            unpack_lines.append(f"    _node_ref['31'] = {node_type.value}")
        # bench_id
        if prop.node_has_bench:
            unpack_lines.append(f"    _node_ref['34'] = row['{prop.name}_bench_id']")
        elif any(issubclass(NODE_CLASS_BY_TYPE[node_type], IsInBench) for node_type in node_types):
            unpack_lines.append("    _node_ref['34'] = row.get('bench_id')")
        # definition_id
        if prop.node_has_definition:
            unpack_lines.append(f"    _node_ref['35'] = row['{prop.name}_definition_id']")

        unpack_lines.append(f"    node_value['{prop.id}'] = _node_ref")
        return "\n".join(unpack_lines)
    else:
        # regular properties
        if prop.cardinality == "scalar":
            unpack_expr = _generate_unpack_scalar_value(prop, f"row['{prop.name}']")
            if prop.is_required:
                return f"node_value['{prop.id}'] = {unpack_expr}"
            else:
                unpack_expr_opt = _generate_unpack_scalar_value(prop, "_value")
                return f"""\
if (_value := row['{prop.name}']) is not None:
    node_value['{prop.id}'] = {unpack_expr_opt}"""
        elif prop.cardinality == "list":
            unpack_expr = _generate_unpack_scalar_value(prop, "v")
            return f"""\
if (_value := row['{prop.name}']) and _value:
    node_value['{prop.id}'] = [{unpack_expr} for v in _value]"""
        elif prop.cardinality == "map":
            return f"""\
if (_value := row['{prop.name}']) and _value and (_unpacked_value := orjson.loads(_value)):
    node_value['{prop.id}'] = _unpacked_value"""
        else:
            assert_never(prop.cardinality)


def _generate_node_row_pack(node_cls: type[Node]) -> str:
    """Generate a function to pack a Node Value into a row tuple."""
    lines = ["row_values = []"]

    for prop in node_cls.__properties_in_order__:
        if not prop.is_stored:
            continue
        pack_code = _generate_column_pack(prop)
        lines.append(pack_code)

    lines.append("return row_values")
    return "\n".join(lines)


def _generate_node_row_unpack(node_cls: type[Node]) -> str:
    """Generate a function to unpack a row tuple into a Node Value."""
    lines = ["node_value = {}"]

    for prop in node_cls.__properties_in_order__:
        if not prop.is_stored:
            continue
        unpack_code = _generate_column_unpack(prop)
        lines.append(unpack_code)

    lines.append("return node_value")
    return "\n".join(lines)


# generate dynamic implementations
for node_type in NodeType:
    node_cls = NODE_CLASS_BY_TYPE[node_type]
    # row pack/unpack
    node_pack_str, extra_glbls = generate_pack_row(node_cls)
    locals: dict[str, Any] = {}
    print("=" * 80)
    print(node_cls.__name__ + ":pack")
    print("=" * 80)
    print(node_pack_str)
    print("=" * 80)
    exec_(node_pack_str, extra_glbls, locals, f"{node_cls.__name__}:row_pack")
    NODE_ROW_PACK[node_type] = locals[f"_pack_{node_cls.__name__}_row"]
    NODE_ROW_UNPACK[node_type] = locals[f"_unpack_{node_cls.__name__}_row"]


def pack_node_value_to_row(table: DatabaseTable, value: Value) -> Sequence[Any]:
    """Pack a Node Value into an asyncpg row (tuple)."""
    type = value.type
    assert type.scalar_type == ScalarType.NODE_VALUE, f"unexpected node value: {value!r}"
    node_type = table.node_type
    assert node_type is not None, f"no node type for {table!r}"
    assert node_type == type.node_type, f"node type mismatch: {node_type!r} != {type.node_type!r}"

    node_pack = NODE_ROW_PACK[node_type]
    return node_pack(value.value)


def unpack_row_to_node_value(table: DatabaseTable, row: asyncpg.Record) -> Value:
    """Unpack an asyncpg row into a Node Value."""
    node_type = table.node_type
    assert node_type is not None, f"no node type for {table!r}"
    node_unpack = NODE_ROW_UNPACK[node_type]
    node_value = node_unpack(row)
    type = Type(
        cardinality=TypeCardinality.SCALAR, scalar_type=ScalarType.NODE_VALUE, node_type=node_type
    )
    value = Value(type=type, value=node_value)
    return value


def _pack_column_scalar(type: Type, value: Json) -> Any:
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


def pack_column(type: Type, value_packed: Json) -> Any:
    if type.cardinality == TypeCardinality.SCALAR:
        return _pack_column_scalar(type, value_packed)
    elif type.cardinality == TypeCardinality.LIST:
        return [_pack_column_scalar(type, v) for v in value_packed]
    elif type.cardinality == TypeCardinality.MAP:
        return orjson.dumps(value_packed)  # keep json
    else:
        assert_never(type.cardinality)


def _unpack_column_scalar(type: Type, value: Any) -> Json:
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


def unpack_column(type: Type, value: Any) -> Json:
    if type.cardinality == TypeCardinality.SCALAR:
        return _unpack_column_scalar(type, value)
    elif type.cardinality == TypeCardinality.LIST:
        return [_unpack_column_scalar(type, v) for v in value]
    elif type.cardinality == TypeCardinality.MAP:
        return orjson.loads(value)  # keep json
    else:
        assert_never(type.cardinality)
