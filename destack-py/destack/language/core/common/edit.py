from collections.abc import Collection
from datetime import datetime
from typing import TYPE_CHECKING

import structlog
from opentelemetry import trace

from destack.utils.uuid import UUID

from ..builtin import (
    UNSET,
    DefaultFactory,
    Enum,
    EnumType,
    Node,
    NodeType,
    Property,
    StructFrozen,
    StructType,
    bittuple,
    builtin_enum,
    builtin_struct,
    property_,
)
from .relation import NodeReference, PropertyReference

if TYPE_CHECKING:
    from destack.language import Field, Graph, IsSubject, Origin, Value

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


CASCADING_EDIT_TYPES: bittuple[EditType] = bittuple(
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

    id: UUID = property_(2, is_managed=True, is_repr=True, default_factory=DefaultFactory.UUID)
    type: EditType = property_(30, is_repr=True)
    operation: EditOperation | None = property_(31, is_repr=True)
    node: Node = property_(32, is_repr=True)
    prop: Property | None = property_(33, is_repr=True)
    field: "Field | None" = property_(34, is_repr=True)  # for IsExtensible.value
    key: "Value | None" = property_(35, is_repr=True)  # for map operations
    if TYPE_CHECKING:
        node_id: UUID = UNSET
        node_ptr: NodeReference = UNSET
        node_type: NodeType = UNSET
        field_id: UUID | None = None
        field_ptr: NodeReference | None = None
        prop_ptr: PropertyReference | None = None

    value: "Value | None" = property_(40)
    undo: "Edit | None" = property_(
        50,
        description="The inverse Edit (if it cannot be derived from the Edit itself).",
    )


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
    id: UUID = property_(2, is_managed=True, is_repr=True, default_factory=DefaultFactory.UUID)
    name: str | None = property_(31, is_repr=True)
    created_at: datetime = property_(
        32, is_managed=True, is_repr=True, default_factory=DefaultFactory.NOW
    )
    created_by: "IsSubject | None" = property_(33, is_managed=True, is_repr=True)
    origin: "Origin | None" = property_(34, is_managed=True, is_repr=True)
    debounce: "ChangeDebounce | None" = property_(35, is_managed=True, is_repr=True)

    edits: list[Edit] = property_(40)


@builtin_struct(StructType.CHANGE_RESULT, frozen=True)
class ChangeResult(StructFrozen):
    """The result of a Change. If rejected, edits/cascaded_edits are empty."""

    id: UUID = property_(
        2,
        is_managed=True,
        is_repr=True,
        default_factory=DefaultFactory.UUID,
        description="The id of the Change.",
    )
    created_at: datetime = property_(
        10,
        is_managed=True,
        is_repr=True,
        description="The time the ChangeResult was created.",
        default_factory=DefaultFactory.NOW,
    )
    debounce: "ChangeDebounce | None" = property_(35, is_managed=True, is_repr=True)
    status: ChangeStatus = property_(40, is_repr=True)
    edits: list[Edit] = property_(41, description="The applied Edits (may differ).")
    cascaded_edits: list[Edit] = property_(
        42, description="The Edits cascaded from the applied Edits."
    )


def edit_node(node: Node, edit: Edit) -> None:
    """Applies the Edit to the Node."""
    raise NotImplementedError


def edit_graph(graph: "Graph", edits: Collection[Edit]) -> None:
    """Applies the Edits to the graph."""
    raise NotImplementedError
