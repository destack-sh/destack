from datetime import datetime
from typing import TYPE_CHECKING, Optional

import structlog
from opentelemetry import trace

from destack.proto import OriginProto
from destack.utils.uuid import UUID

from ..builtin import (
    UNSET,
    Enum,
    EnumType,
    Node,
    StructFrozen,
    StructType,
    ValueFactory,
    builtin_enum,
    builtin_property,
    builtin_struct,
)
from ..builtin.relation import NodeReference, PropertyReference

if TYPE_CHECKING:
    from destack.language import ClientType, CustomProperty, IsSubject, Value

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@builtin_enum(EnumType.EDIT_TYPE)
class EditType(Enum):
    CREATE = 1
    UPSERT = 2
    UPDATE = 3
    MOVE = 4
    ARCHIVE = 5
    UNARCHIVE = 6
    DELETE = 7
    RESTORE = 8
    ERASE = 9


CASCADING_EDIT_TYPES: tuple[EditType, ...] = (
    EditType.ARCHIVE,
    EditType.UNARCHIVE,
    EditType.DELETE,
    EditType.RESTORE,
    EditType.ERASE,
)


@builtin_enum(EnumType.EDIT_OPERATION)
class EditOperation(Enum):
    # direct
    SET = 1
    CLEAR = 2

    # set
    # SET_ADD, SET_REMOVE, ...

    # list
    # LIST_APPEND, LIST_APPEND_IF_MISSING, LIST_REMOVE, ...

    # map
    # MAP_SET, MAP_REMOVE, ...

    # number
    # NUMBER_INCREMENT, NUMBER_DECREMENT, ...

    # string
    # STRING_INSERT, STRING_DELETE, STRING_REPLACE, ...

    # text
    # TEXT_INSERT, TEXT_DELETE, TEXT_REPLACE, TEXT_FORMAT, ...

    # bitmap
    # BITMAP_INSERT, BITMAP_DELETE, ...


@builtin_struct(StructType.EDIT, frozen=True)
class Edit(StructFrozen):
    """
    An Edit to a Node.
    """

    id: UUID = builtin_property(2, is_managed=True, is_repr=True, default_factory=ValueFactory.UUID)
    type: EditType = builtin_property(100, is_repr=True)
    operation: EditOperation | None = builtin_property(101, is_repr=True)
    node: Node = builtin_property(102, is_repr=True)
    prop_ptr: PropertyReference | None = builtin_property(103, is_repr=True)
    field: "CustomProperty | None" = builtin_property(104, is_repr=True)  # for IsExtensible.value
    key: "Value | None" = builtin_property(105, is_repr=True)  # for map operations
    if TYPE_CHECKING:
        node_ptr: NodeReference = UNSET
        field_ptr: NodeReference | None = None

    value: "Value | None" = builtin_property(110)

    undo: "Edit | None" = builtin_property(
        120,
        description="The inverse Edit (if it cannot be derived from the Edit itself).",
    )


@builtin_struct(StructType.ORIGIN, frozen=True)
class Origin(StructFrozen[OriginProto]):
    """Origin of something."""

    type: "ClientType" = builtin_property(100)
    id: Optional[UUID] = builtin_property(101)
    ck: Optional[UUID] = builtin_property(102)
    nonce: Optional[UUID] = builtin_property(103)


@builtin_enum(EnumType.CHANGE_STATUS)
class ChangeStatus(Enum):
    """The status of a Change."""

    # PENDING?
    COMPLETED = 10, "Completed", "The Change was applied"
    FAILED = 12, "Failed", "Could not apply the Change."
    REJECTED = 13, "Rejected", "Insufficient access."


@builtin_enum(EnumType.CHANGE_DEBOUNCE)
class ChangeDebounce(Enum):
    """How to debounce the Change. Only available for certain Changes."""

    LAZY = 10, "Lazy", "Lazy debounce the Change."


@builtin_struct(StructType.CHANGE, frozen=True)
class Change(StructFrozen):
    """A Change is an atomic sequence of Edits."""

    # meta
    id: UUID = builtin_property(2, is_managed=True, is_repr=True, default_factory=ValueFactory.UUID)
    name: str | None = builtin_property(101, is_repr=True)
    created_at: datetime = builtin_property(
        102, is_managed=True, is_repr=True, default_factory=ValueFactory.NOW
    )
    created_by: "IsSubject | None" = builtin_property(103, is_managed=True, is_repr=True)
    origin: "Origin | None" = builtin_property(104, is_managed=True, is_repr=True)
    debounce: "ChangeDebounce | None" = builtin_property(105, is_managed=True, is_repr=True)

    edits: list[Edit] = builtin_property(110)


@builtin_struct(StructType.CHANGE_RESULT, frozen=True)
class ChangeResult(StructFrozen):
    """The result of a Change. If rejected, edits/cascaded_edits are empty."""

    id: UUID = builtin_property(
        2,
        is_managed=True,
        is_repr=True,
        default_factory=ValueFactory.UUID,
        description="The id of the Change.",
    )
    created_at: datetime = builtin_property(
        20,
        is_managed=True,
        is_repr=True,
        description="The time the ChangeResult was created.",
        default_factory=ValueFactory.NOW,
    )
    debounce: "ChangeDebounce | None" = builtin_property(105, is_managed=True, is_repr=True)
    status: ChangeStatus = builtin_property(100, is_repr=True)

    edits: list[Edit] = builtin_property(110, description="The applied Edits (may differ).")
    cascaded_edits: list[Edit] = builtin_property(
        111, description="The Edits cascaded from the applied Edits."
    )
