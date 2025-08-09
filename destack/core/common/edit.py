from typing import TYPE_CHECKING, Optional

from ..builtin import (
    Entity,
    EnumType,
    Event,
    ImmutableStruct,
    NodeType,
    OptionEnum,
    StructType,
    UInt8,
    declare_enum,
    declare_event,
    declare_option,
    declare_property,
    declare_struct,
)

if TYPE_CHECKING:
    from destack import Value


@declare_enum(EnumType.EDIT_TYPE)
class EditType(OptionEnum):
    """The type of Edit."""

    CREATE = declare_option(1, description="Create a new Entity")
    UPSERT = declare_option(
        2, description="Upsert an Entity (create if not exists, update if exists)"
    )
    # INSTANTIATE, MATERIALIZE, ...?
    UPDATE = declare_option(10, description="Update an existing Entity")
    MOVE = declare_option(11, description="Move an Entity to a new parent Entity (or detach)")
    DELETE = declare_option(20, description="Delete an Entity (and its descendants)")
    RESTORE = declare_option(21, description="Restore a deleted Entity (and its descendants)")
    # TODO :Incomplete: EditType.CHECK/ASSERT (for transaction/change safety)


@declare_enum(EnumType.EDIT_OPERATION_TYPE)
class EditOperationType(OptionEnum):
    """The update operation to perform on a Node."""

    # direct
    SET = declare_option(1, description="Set a Property to a value (may be an empty value)")
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


@declare_struct(StructType.EDIT_OPERATION, frozen=True)
class EditOperation(ImmutableStruct):
    """A recorded Edit of an Entity."""

    type: "EditOperationType" = declare_property(100, description="The type of Edit operation.")
    # TODO :Incomplete: EditOperation.path?
    property_id: Optional[UInt8] = declare_property(
        103,
        description="""\
The id of the builtin Property being edited.
If it's a custom Property, this just refers to Entity.custom_values.
""",
    )
    custom_property_name: str | None = declare_property(
        104, is_repr=True, description="The name of the custom Property being edited."
    )
    key: Optional["Value"] = declare_property(
        105, is_repr=True, description="The key being edited."
    )
    # path?
    value: Optional["Value"] = declare_property(120)

    # reverse
    # EditEvent.type in reverse is derivable
    # EditEvent.node is same
    reverse_operation: Optional["EditOperation"] = declare_property(
        202, description="The specific reverse Edit operation."
    )
    # EditEvent.attribute/key is same
    reverse_value: Optional["Value"] = declare_property(
        210, description="The value of the reverse Edit."
    )


@declare_event(NodeType.EDIT_EVENT)
class EditEvent(Event):
    """A recorded Edit of an Entity."""

    # NOTE :Incomplete: would be cool to support custom EditTypes/Operations/Events somehow...

    # change: Optional[ChangeEvent]? (bigger ChangeEvent this is a part of)

    # forward
    type: "EditType" = declare_property(100, is_repr=True, description="The type of Edit.")
    node: "Entity" = declare_property(101, is_repr=True, description="The Entity being edited.")
    value: Optional["Value"] = declare_property(
        110,
        is_repr=True,
        description="The full Node (for create/upsert).",
    )
    reverse_value: Optional["Value"] = declare_property(
        111,
        description="The reverse value (for delete/restore).",
    )
    operations: Optional[list["EditOperation"]] = declare_property(
        112,
        description="The update operations (for update/move).",
    )
