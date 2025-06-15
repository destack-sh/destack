from destack.language import NodeReference, ScalarType, Type, TypeCardinality, Value
from destack.utils.uuid import UUID

from .core import MemoryRow, MemoryTable


def pack_node_row(table: MemoryTable, value: Value) -> MemoryRow:
    """Pack a Value into a MemoryRow."""
    value_packed = value.value
    id = UUID(value_packed["2"])
    ptr = NodeReference(
        node_type=table.node_type,
        id=UUID(value_packed["2"]),
        definition_id=UUID(value_packed["17"]["32"]) if "17" in value_packed else None,
        space_id=UUID(value_packed["7"]["32"]) if "7" in value_packed else None,
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
    """Unpack a MemoryRow to a Value."""
    type_info = Type(
        cardinality=TypeCardinality.SCALAR,
        scalar_type=ScalarType.NODE_VALUE,
        node_type=row.metatype,
    )
    return Value(type=type_info, value=row.value)
