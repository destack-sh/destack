from destack.language import (
    Entity,
    IsExtensible,
    IsSpatial,
    Materialization,
    Node,
    NodeReference,
    NodeType,
    ScalarType,
    Type,
    TypeCardinality,
    Value,
)
from destack.utils.uuid import UUID

from .core import MemoryEntityRow

NODE_METATYPE_KEY = str(Node.property("metatype").id)
NODE_ID_KEY = str(Node.property("id").id)
NODE_PARENT_PTR_KEY = str(Node.property("parent").id)
NODE_SPACE_PTR_ID = str(IsSpatial.property("space").id)
NODE_DEFINITION_PTR_ID = str(IsExtensible.property("definition").id)

ENTITY_SNAPSHOT_PTR_KEY = str(Entity.property("snapshot").id)
ENTITY_MATERIALIZATION_KEY = str(Entity.property("materialization").id)

NODE_REFERENCE_ID_KEY = str(NodeReference.property("id").id)


def pack_entity_row(value: Value) -> MemoryEntityRow:
    """Pack a Value into a MemoryRow."""
    value_packed = value.value
    assert value_packed is not None, f"no value for {value!r}"
    node_type = NodeType(value_packed[NODE_METATYPE_KEY])
    id = UUID(value_packed[NODE_ID_KEY])
    ptr = NodeReference(
        type=node_type,
        id=UUID(value_packed[NODE_ID_KEY]),
        space_id=UUID(value_packed[NODE_SPACE_PTR_ID][NODE_REFERENCE_ID_KEY])
        if NODE_SPACE_PTR_ID in value_packed
        else None,
        definition_id=UUID(value_packed[NODE_DEFINITION_PTR_ID][NODE_REFERENCE_ID_KEY])
        if NODE_DEFINITION_PTR_ID in value_packed
        else None,
    )
    parent_ptr = value_packed.get(NODE_PARENT_PTR_KEY)
    if parent_ptr is not None:
        parent_ptr = NodeReference.from_value(parent_ptr)
    snapshot_ptr = value_packed.get(ENTITY_SNAPSHOT_PTR_KEY)
    if snapshot_ptr is not None:
        snapshot_ptr = NodeReference.from_value(snapshot_ptr)
    materialization = value_packed.get(ENTITY_MATERIALIZATION_KEY)
    if materialization is not None:
        materialization = Materialization(materialization)
    row = MemoryEntityRow(
        metatype=node_type,
        id=id,
        snapshot_id=snapshot_ptr.id if snapshot_ptr is not None else None,
        materialization=materialization,
        ptr=ptr,
        parent_ptr=parent_ptr,
        value=value_packed,
    )
    return row


def unpack_entity_row(row: MemoryEntityRow) -> Value:
    """Unpack a MemoryRow to a Value."""
    type_info = Type(
        cardinality=TypeCardinality.SCALAR,
        scalar_type=ScalarType.NODE_VALUE,
        node_type=row.metatype,
    )
    return Value(type=type_info, value=row.value)
