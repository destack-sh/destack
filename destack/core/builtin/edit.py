from typing import TYPE_CHECKING

from .builtin import EnumType, NodeType
from .common import UInt8
from .const import UNSET
from .entity import Entity
from .enum import Enum, declare_enum
from .event import Event, declare_event
from .property import declare_property

if TYPE_CHECKING:
    from destack import NodeReference, Value


@declare_enum(EnumType.EDIT_TYPE)
class EditType(Enum):
    """The type of Edit."""

    CREATE = 1, "Create a new Entity"
    UPSERT = 2, "Upsert an Entity (create if not exists, update if exists)"
    # INSTANTIATE, MATERIALIZE, ...?
    UPDATE = 10, "Update an existing Entity"
    MOVE = 11, "Move an Entity to a new parent Entity (or detach)"
    DELETE = 20, "Delete an Entity (and its descendants)"
    RESTORE = 21, "Restore a deleted Entity (and its descendants)"


@declare_enum(EnumType.EDIT_OPERATION)
class EditOperation(Enum):
    """The update operation to perform on a Node."""

    # direct
    SET = 1, "Set a Property to a value (may be an empty value)"
    # UNSET = 3, "Unset a Property (remove it from the override)"

    # sequence
    # ADD, REMOVE, ...

    # math/operation (more general than number with custom types, vectors, etc.)
    # OPERATION_ADD, OPERATION_SUBTRACT, OPERATION_MULTIPLY, OPERATION_DIVIDE, ...

    # string
    # STRING_INSERT, STRING_DELETE, STRING_REPLACE, ...

    # text
    # TEXT_INSERT, TEXT_DELETE, TEXT_REPLACE, TEXT_FORMAT, ...

    # bitmap
    # BITMAP_INSERT, BITMAP_DELETE, ...


@declare_event(NodeType.EDIT_EVENT)
class EditEvent(Event):
    """A recorded Edit of an Entity."""

    # NOTE :Incomplete: would be cool to support custom EditTypes/Operations/Events somehow...

    # change: Optional[ChangeEvent]? (bigger ChangeEvent this is a part of)

    # forward
    type: "EditType" = declare_property(100, is_repr=True, description="The type of Edit.")
    node: "Entity" = declare_property(101, is_repr=True, description="The Entity being edited.")
    operation: "EditOperation | None" = declare_property(
        102, is_repr=True, description="The specific Edit operation."
    )
    property_id: UInt8 | None = declare_property(
        103,
        is_repr=True,
        description="""\
The id of the builtin Property being edited.
If it's a custom Property, this just refers to Entity.custom_values.
""",
    )
    custom_property_name: str | None = declare_property(
        104, is_repr=True, description="The name of the custom Property being edited."
    )
    key: "Value | None" = declare_property(105, is_repr=True, description="The key being edited.")
    # path?
    value: "Value | None" = declare_property(120)
    if TYPE_CHECKING:
        node_ptr: "NodeReference" = UNSET

    # reverse
    # EditEvent.type in reverse is derivable
    # EditEvent.node is same
    reverse_operation: "EditOperation | None" = declare_property(
        202, description="The specific reverse Edit operation."
    )
    # EditEvent.attribute/key is same
    reverse_value: "Value | None" = declare_property(
        210, description="The value of the reverse Edit."
    )
