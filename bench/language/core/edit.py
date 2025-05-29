from datetime import datetime
from typing import TYPE_CHECKING, Collection

import structlog
from fastuuid import UUID
from opentelemetry import trace

from .const import BuiltinEnum, EnumType, StructType, bittuple, enum_
from .graph import Graph
from .node import Node
from .property import property_
from .struct import StructFrozen, StructMutable, struct_

if TYPE_CHECKING:
    from bench.language import Value

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@enum_(EnumType.EDIT_TYPE)
class EditType(BuiltinEnum):
    """Ways to edit nodes."""

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


@enum_(EnumType.UPDATE_TYPE)
class UpdateType(BuiltinEnum):
    """The type of update operation."""

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
    id: UUID = property_(2, default_factory="uuid")

    # key
    type: EditType = property_(30)
    operation: UpdateType | None = property_(31)
    node: Node = property_(32)
    path: str | None = property_(33)
    key: "Value | None" = property_(34)  # for map operations

    # value
    # node_data: "NodeData | None" = property_(40)
    value: "Value | None" = property_(41)
    parent: Node | None = property_(42)  # for move


@struct_(StructType.CHANGE)
class Change(StructMutable):
    """A Change is an atomic sequence of Edits."""

    id: UUID = property_(2, is_managed=True, default_factory="uuid")
    created_at: datetime = property_(10, is_managed=True)
    edits: list[Edit] = property_(40)


@struct_(StructType.CHANGE_RESULT, frozen=True)
class ChangeResult(StructFrozen):
    """The result of a Change."""

    id: UUID = property_(2, default_factory="uuid")
    edits: list[Edit] = property_(40)
    cascaded_edits: list[Edit] = property_(41)
    epoch: int = property_(42)


def edit_node(node: Node, edit: Edit) -> None:
    """Applies the Edit to the Node."""
    raise NotImplementedError


def edit_graph(graph: Graph, edits: Collection[Edit]) -> None:
    """Applies the Edits to the graph."""
    raise NotImplementedError
