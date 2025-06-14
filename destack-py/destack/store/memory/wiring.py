from typing import Any

from fastuuid import UUID

from destack.language import Field, Json, NodeReference, Type, Value

from .core import MemoryRow, MemoryTable


def pack_node_row(table: MemoryTable, value: Value) -> MemoryRow:
    value_packed = value.value
    id = value_packed["2"]
    ptr = NodeReference(
        node_type=table.node_type,
        id=UUID(value_packed["2"]),
        definition_id=UUID(value_packed["17"]["32"]) if "17" in value_packed else None,
        space_id=UUID(value_packed["7"]["33"]) if "7" in value_packed else None,
    )
    parent_ptr = value_packed.get("3")
    if parent_ptr is not None:
        parent_ptr = NodeReference.from_value(parent_ptr)
    row = MemoryRow(
        table=table,
        metatype=table.node_type,
        id=id,
        ptr=ptr,
        parent_ptr=parent_ptr,
        value=value_packed,
    )
    return row


def unpack_node_row(table: MemoryTable, row: MemoryRow) -> Value:
    raise NotImplementedError


def pack_column(type: "Type | Field", value: Json) -> Any:
    raise NotImplementedError


def unpack_column(type: "Type | Field", value: Any) -> Json:
    raise NotImplementedError
