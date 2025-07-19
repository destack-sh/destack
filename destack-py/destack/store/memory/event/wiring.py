from datetime import datetime

from destack.language import (
    Entity,
    Event,
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
NODE_PARENT_PTR_KEY = str(Entity.property("parent").id)
NODE_SPACE_PTR_KEY = str(Node.property("space").id)
ENTITY_DEFINITION_PTR_KEY = str(Entity.property("definition").id)

EVENT_CREATED_AT_KEY = str(Event.property("created_at").id)
EVENT_SNAPSHOT_PTR_KEY = str(Event.property("snapshot").id)

NODE_REFERENCE_ID_KEY = str(NodeReference.property("id").id)


def pack_event_row(value: Value) -> MemoryEventRow:
    """Pack a Value into a MemoryRow."""
    value_cson = value.value
    assert value_cson is not None, f"no value for {value!r}"
    node_type = NodeType(value_cson[NODE_METATYPE_KEY])
    id = UUID(value_cson[NODE_ID_KEY])
    ptr = NodeReference(
        type=node_type,
        id=UUID(value_cson[NODE_ID_KEY]),
        space_id=UUID(value_cson[NODE_SPACE_PTR_KEY][NODE_REFERENCE_ID_KEY])
        if NODE_SPACE_PTR_KEY in value_cson
        else None,
        definition_id=UUID(value_cson[ENTITY_DEFINITION_PTR_KEY][NODE_REFERENCE_ID_KEY])
        if ENTITY_DEFINITION_PTR_KEY in value_cson
        else None,
    )
    snapshot_ptr = value_cson.get(EVENT_SNAPSHOT_PTR_KEY)
    if snapshot_ptr is not None:
        snapshot_ptr = NodeReference.from_cson(snapshot_ptr)
    created_at = datetime.fromisoformat(value_cson[EVENT_CREATED_AT_KEY])
    row = MemoryEventRow(
        metatype=node_type,
        id=id,
        snapshot_id=snapshot_ptr.id if snapshot_ptr is not None else None,
        ptr=ptr,
        created_at=created_at,
        value=value_cson,
    )
    return row


def unpack_event_row(row: MemoryEventRow) -> Value:
    """Unpack a MemoryRow to a Value."""
    type = NODE_TYPE_SCALAR_BY_NODE_TYPE[row.metatype]
    return Value(type=type, value=row.value)
