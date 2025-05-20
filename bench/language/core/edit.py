from datetime import datetime
from typing import TYPE_CHECKING, Collection

import structlog
from fastuuid import UUID
from opentelemetry import trace

from bench.pb2 import (
    AnyNodeData,
    EditData,
    EditOperationData,
)

from .const import EditOperationType, EditType, StructType
from .graph import Graph, GraphData
from .node import Node
from .property import p_regular, p_system
from .struct import Struct, struct_

if TYPE_CHECKING:
    from bench.language import Supergraph, Value

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@struct_(StructType.EDIT_OPERATION)
class EditOperation(Struct):
    """An edit operation."""

    type: EditOperationType = p_system(30)
    key: str = p_system(31)
    new_value: "Value | None" = p_regular(35)


@struct_(StructType.EDIT)
class Edit(Struct):
    """
    An Edit to a Node.
    """

    # core
    id: UUID = p_system(
        2,
        description="Unique identifier for the Edit within a Session.",
    )
    type: EditType = p_system(30, description="Type of Edit.")
    node: Node = p_system(31, description="Which Node.")
    edited_at: datetime = p_system(33, description="When the Edit was made.")
    change_key: UUID | None = p_system(34, description="The Change that this Edit is part of.")

    # content
    node_data: AnyNodeData | None = p_system(40, primitive_type=None, is_node_data=True)
    operations: list[EditOperation] = p_system(
        41,
        description="The operations to perform on the Node",
    )


def apply_edit_operation(node: Node, op: EditOperationData, validate: bool) -> None:
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


def apply_edit_operation_data(node: AnyNodeData, op: EditOperationData, is_prepass: bool) -> None:
    raise NotImplementedError


@tracer.start_as_current_span("graph.edit_data_graph")
def edit_data_graph(
    graph: GraphData,
    edits: Collection[EditData],
    *,
    include_removed: bool,
    is_prepass: bool = False,
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
