from datetime import datetime
from typing import TYPE_CHECKING, Collection

import structlog
from fastuuid import UUID
from opentelemetry import trace

from bench.pb2 import AnyNodeData, EditData

from .const import BuiltinEnum, EnumType, StructType, bittuple, enum_
from .graph import Graph, GraphData
from .node import Node
from .property import property_
from .struct import Struct, struct_

if TYPE_CHECKING:
    from bench.language import Supergraph, Value

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


@enum_(EnumType.EDIT_OPERATION)
class EditOperation(BuiltinEnum):
    """The type of edit operation."""

    # direct
    SET = 1
    CLEAR = 2

    # list
    # LIST_APPEND, LIST_APPEND_IF_MISSING, LIST_REMOVE, ...

    # map
    MAP_SET = 20
    MAP_REMOVE = 21
    # MAP_SET, MAP_REMOVE, ...

    # math
    # NUMBER_ADD, NUMBER_SUBTRACT, ...

    # text
    # ...


@struct_(StructType.EDIT, is_frozen=True)
class Edit(Struct):
    """
    An Edit to a Node.
    """

    # core
    id: UUID = property_(
        2,
        description="Unique identifier for the Edit within a Session.",
    )
    type: EditType = property_(30, description="Type of Edit.")
    node: Node = property_(31)
    key: str | None = property_(40)
    operation: EditOperation | None = property_(41)
    # node_data: AnyNodeData | None = property_(40, primitive_type=None, is_node_data=True)
    new_value: "Value | None" = property_(42)
    key_value: "Value | None" = property_(43)  # for map operations
    new_parent: Node | None = property_(44)  # for move

    edited_at: datetime = property_(50, description="When the Edit was made.")
    # change_key?


def apply_edit_operation(node: Node, op: EditData, validate: bool) -> None:
    raise NotImplementedError


@tracer.start_as_current_span("graph.edit_graph")
def edit_graph(
    graph: Graph,
    supergraph: "Supergraph",
    edits: Collection[EditData],
    *,
    include_removed: bool,
    validate: bool,
    ignore_missing: bool = False,
) -> None:
    """Applies the edits to the graph (in place!)."""
    raise NotImplementedError


def apply_edit_operation_data(node: AnyNodeData, op: EditData) -> None:
    raise NotImplementedError


@tracer.start_as_current_span("graph.edit_data_graph")
def edit_data_graph(
    graph: GraphData,
    edits: Collection[EditData],
    *,
    include_removed: bool,
    ignore_missing: bool = False,
) -> list[EditData] | None:
    """
    Applies the edits to the data graph (edited nodes are copied before update).
    During prepass, we do some extra work (before applying the edits within in the system):
      1. Update edits (and edit operations) with the ground truth state.
      2. Simplify hierarchical edits into flat set/clear edits (for storage).
    """
    trace.get_current_span().set_attribute("edits", len(edits))

    raise NotImplementedError
