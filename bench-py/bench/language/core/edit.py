from collections.abc import Collection
from datetime import datetime
from typing import TYPE_CHECKING

import structlog
from fastuuid import UUID
from opentelemetry import trace

from .const import (
    UNSET,
    BuiltinEnum,
    DefaultFactory,
    EnumType,
    NodeType,
    StructType,
    bittuple,
    enum_,
)
from .graph import Graph
from .node import Node
from .property import Property, property_
from .struct import NodeReference, PropertyReference, StructFrozen, struct_

if TYPE_CHECKING:
    from bench.language import Field, IsSubject, Origin, Value

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@enum_(EnumType.EDIT_TYPE)
class EditType(BuiltinEnum):
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


@enum_(EnumType.EDIT_OPERATION)
class EditOperation(BuiltinEnum):
    # direct
    SET = 1
    CLEAR = 2

    # number
    # NUMBER_INCREMENT, NUMBER_DECREMENT, ...

    # string
    # STRING_INSERT, STRING_DELETE, STRING_FORMAT, ...

    # list
    # LIST_APPEND_IF_MISSING, LIST_REMOVE, ...

    # map
    MAP_SET = 110
    MAP_REMOVE = 111
    # MAP_INCREMENT, MAP_DECREMENT, ...


@struct_(StructType.EDIT, frozen=True)
class Edit(StructFrozen):
    """An Edit to a Node."""

    # meta
    id: UUID = property_(2, is_managed=True, is_repr=True, default_factory=DefaultFactory.UUID)

    # key
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
        property_ptr: PropertyReference | None = None

    # value
    value: "Value | None" = property_(40)


@struct_(StructType.CHANGE, frozen=True)
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

    edits: list[Edit] = property_(40)


@enum_(EnumType.CHANGE_STATUS)
class ChangeStatus(BuiltinEnum):
    """The status of a Change."""

    # PENDING?
    COMPLETED = 2
    FAILED = 3


@struct_(StructType.CHANGE_RESULT, frozen=True)
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
    status: ChangeStatus = property_(40, is_repr=True)
    edits: list[Edit] = property_(41)
    cascaded_edits: list[Edit] = property_(42)


def edit_node(node: Node, edit: Edit) -> None:
    """Applies the Edit to the Node."""
    raise NotImplementedError


def edit_graph(graph: Graph, edits: Collection[Edit]) -> None:
    """Applies the Edits to the graph."""
    raise NotImplementedError
