from datetime import datetime

from destack.language import (
    Event,
    IsExtensible,
    Node,
    NodeReference,
    NodeType,
    Value,
)
from destack.language.registry import NODE_TYPE_SCALAR_BY_NODE_TYPE
from destack.utils.uuid import UUID

from .core import MemoryEventRow

NODE_METATYPE_KEY = str(Node.property("metatype").id)
NODE_ID_KEY = str(Node.property("id").id)
NODE_PARENT_PTR_KEY = str(Node.property("parent").id)
NODE_SPACE_PTR_ID = str(Node.property("space").id)
NODE_DEFINITION_PTR_ID = str(IsExtensible.property("definition").id)

EVENT_CREATED_AT_KEY = str(Event.property("created_at").id)
EVENT_SNAPSHOT_PTR_KEY = str(Event.property("snapshot").id)

NODE_REFERENCE_ID_KEY = str(NodeReference.property("id").id)


def pack_event_row(value: Value) -> MemoryEventRow:
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
    snapshot_ptr = value_packed.get(EVENT_SNAPSHOT_PTR_KEY)
    if snapshot_ptr is not None:
        snapshot_ptr = NodeReference.from_value(snapshot_ptr)
    created_at = datetime.fromisoformat(value_packed[EVENT_CREATED_AT_KEY])
    row = MemoryEventRow(
        metatype=node_type,
        id=id,
        snapshot_id=snapshot_ptr.id if snapshot_ptr is not None else None,
        ptr=ptr,
        created_at=created_at,
        value=value_packed,
    )
    return row


def unpack_event_row(row: MemoryEventRow) -> Value:
    """Unpack a MemoryRow to a Value."""
    type = NODE_TYPE_SCALAR_BY_NODE_TYPE[row.metatype]
    return Value(type=type, value=row.value)
