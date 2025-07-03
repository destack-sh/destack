from destack.language import (
    IsExtensible,
    IsSpatial,
    Node,
    NodeReference,
    ScalarType,
    Type,
    TypeCardinality,
    Value,
)
from destack.utils.uuid import UUID

from .core import MemoryRow, MemoryTable

NODE_ID_KEY = str(Node.property("id").id)
NODE_SPACE_PTR_ID = str(IsSpatial.property("space").id)
NODE_DEFINITION_PTR_ID = str(IsExtensible.property("definition").id)

NODE_REFERENCE_ID_KEY = str(NodeReference.property("id").id)


def pack_node_row(table: MemoryTable, value: Value) -> MemoryRow:
    """Pack a Value into a MemoryRow."""
    value_packed = value.value
    assert value_packed is not None, f"no value for {value!r}"
    id = UUID(value_packed[NODE_ID_KEY])
    ptr = NodeReference(
        type=table.node_type,
        id=UUID(value_packed[NODE_ID_KEY]),
        space_id=UUID(value_packed[NODE_SPACE_PTR_ID][NODE_REFERENCE_ID_KEY])
        if NODE_SPACE_PTR_ID in value_packed
        else None,
        definition_id=UUID(value_packed[NODE_DEFINITION_PTR_ID][NODE_REFERENCE_ID_KEY])
        if NODE_DEFINITION_PTR_ID in value_packed
        else None,
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
