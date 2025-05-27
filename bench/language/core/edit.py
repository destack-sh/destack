from typing import TYPE_CHECKING, Collection

import structlog
from fastuuid import UUID
from opentelemetry import trace

from .const import BuiltinEnum, EnumType, StructType, bittuple, enum_
from .graph import Graph
from .node import Node
from .property import property_
from .struct import Struct, struct_

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


@struct_(StructType.EDIT, is_frozen=True)
class Edit(Struct):
    """
    An Edit to a Node.
    """

    # meta
    id: UUID = property_(2, default_factory="uuid")
    # change_key?

    # key
    type: EditType = property_(30)
    operation: UpdateType | None = property_(31)
    node: Node = property_(32)
    path: str | None = property_(33)
    key: "Value | None" = property_(34)  # for map operations

    # value
    value: "Value | None" = property_(40)
    parent: Node | None = property_(41)  # for move


def edit_node(node: Node, edit: Edit) -> None:
    """Applies the Edit to the Node."""
    raise NotImplementedError


def edit_graph(graph: Graph, edits: Collection[Edit]) -> None:
    """Applies the Edits to the graph."""
    raise NotImplementedError
