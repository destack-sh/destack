from typing import TYPE_CHECKING

import structlog
from opentelemetry import trace

from .common import EnumType, NodeType
from .const import UNSET
from .constant import register_constant
from .entity import Entity
from .enum import Enum, builtin_enum
from .event import Event
from .node import builtin_node
from .property import builtin_property

if TYPE_CHECKING:
    from destack.language import (
        NodeReference,
        PropertyReference,
        Value,
    )

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@builtin_enum(EnumType.EDIT_TYPE)
class EditType(Enum):
    """The type of Edit."""

    CREATE = 1, "Create a new Entity"
    UPSERT = 2, "Upsert an Entity (create if not exists, update if exists)"
    UPDATE = 3, "Update an existing Entity"
    MOVE = 4, "Move an Entity to a new parent Entity (or detach)"
    ARCHIVE = 5, "Archive an Entity (and its descendants)"
    UNARCHIVE = 6, "Unarchive an Entity (and its descendants)"
    DELETE = 7, "Delete an Entity (and its descendants)"
    RESTORE = 8, "Restore a deleted Entity (and its descendants)"
    ERASE = 9, "Erase an Entity forever (and its descendants)"


CASCADING_EDIT_TYPES: tuple[EditType, ...] = (
    EditType.ARCHIVE,
    EditType.UNARCHIVE,
    EditType.DELETE,
    EditType.RESTORE,
    EditType.ERASE,
)
register_constant("CASCADING_EDIT_TYPES", CASCADING_EDIT_TYPES)


@builtin_enum(EnumType.EDIT_OPERATION)
class EditOperation(Enum):
    """The update operation to perform on a Node."""

    # direct
    SET = 1, "Set a Property to a value"
    CLEAR = 2, "Clear a Property"

    # collection

    # list
    # LIST_APPEND, LIST_REMOVE, ...

    # set
    # SET_ADD, SET_REMOVE, ...

    # map
    # MAP_SET, MAP_REMOVE, ...

    # scalar

    # number
    # NUMBER_INCREMENT, NUMBER_DECREMENT, ...

    # math/operation (more general than number with custom types, vectors, etc.)
    # OPERATION_ADD, OPERATION_SUBTRACT, OPERATION_MULTIPLY, OPERATION_DIVIDE, ...

    # string
    # STRING_INSERT, STRING_DELETE, STRING_REPLACE, ...

    # text
    # TEXT_INSERT, TEXT_DELETE, TEXT_REPLACE, TEXT_FORMAT, ...

    # bitmap
    # BITMAP_INSERT, BITMAP_DELETE, ...


@builtin_node(NodeType.EDIT_EVENT, frozen=True)
class EditEvent(Event):
    """A recorded Edit of an Entity."""

    # meta
    # Event.snapshot: 11
    # Event.created_at: 20
    # Event.created_by: 21
    # Event.status
    cascaded_from: "EditEvent | None" = builtin_property(
        85, is_repr=True, description="The EditEvent that caused this EditEvent."
    )
    if TYPE_CHECKING:
        cascaded_from_ptr: "NodeReference | None" = None

    # forward
    type: "EditType" = builtin_property(100, is_repr=True, description="The type of Edit.")
    node: "Entity" = builtin_property(101, is_repr=True, description="The Entity being edited.")
    operation: "EditOperation | None" = builtin_property(
        102, is_repr=True, description="The specific Edit operation."
    )
    attribute: "PropertyReference | None" = builtin_property(
        103, is_repr=True, description="The builtin or custom Property being edited."
    )
    key: "Value | None" = builtin_property(
        105, is_repr=True, description="The key for map operations."
    )
    value: "Value | None" = builtin_property(110)
    if TYPE_CHECKING:
        node_ptr: "NodeReference" = UNSET

    # reverse
    # EditEvent.type in reverse is derivable
    # EditEvent.node is same
    reverse_operation: "EditOperation | None" = builtin_property(
        202, is_repr=True, description="The specific reverse Edit operation."
    )
    # EditEvent.attribute/key is same
    reverse_value: "Value | None" = builtin_property(
        210, is_repr=True, description="The value of the reverse Edit."
    )
