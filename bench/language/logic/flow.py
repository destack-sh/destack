from typing import TYPE_CHECKING, Optional, Union

from fastuuid import UUID

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    HasName,
    IsArchivable,
    IsBlockable,
    IsDeletable,
    IsInPackage,
    IsModal,
    IsOrdered,
    IsOwnable,
    IsRunnable,
    IsTemplatable,
    Node,
    NodeType,
    enum_,
    node_,
    property_,
    property_parent_,
)
from bench.pb2 import FlowData, FlowEdgeData

if TYPE_CHECKING:
    from bench.language import Action, Flow, NodeReference

# pyright: reportIncompatibleVariableOverride=false

if TYPE_CHECKING:
    from bench.language import Action, Agent, FlowEdge, Page

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.FLOW_TYPE)
class FlowType(BuiltinEnum):
    ACTION = 10, "Action", "Link Actions into a procedural Flow"
    # PLAN/TASK? (lay out a sequence of Tasks declaratively)
    # MESSAGE? (define communication links between agents)
    # ESCALATION/AUTH?


@node_(NodeType.FLOW)
class Flow(
    IsTemplatable,
    IsOwnable,
    IsModal,
    HasName,
    IsRunnable,
    IsBlockable,
    IsDeletable,
    IsArchivable,
    Node[FlowData],
):
    """A Flow orchestrates a sequence of steps (like Actions)."""

    parent: Union["Page", "Agent", None] = property_parent_()
    type: FlowType = property_(30, default=FlowType.ACTION)


@enum_(EnumType.FLOW_EDGE_TYPE)
class FlowEdgeType(BuiltinEnum):
    MANUAL = 10, "Manual", "Manually triggered", "fas fa-link"
    DECIDE = 20, "Decide", "Determine when and how to call", "far fa-shuffle"
    REQUIRE = 30, "Require", "Determine how to call", "fas fa-arrow-right-long"
    # MESSAGE? WAIT? STREAM?


@node_(NodeType.FLOW_EDGE)
class FlowEdge(
    IsTemplatable,
    IsModal,
    IsRunnable,
    HasName,
    IsOrdered,
    IsInPackage,
    Node[FlowEdgeData],
):
    """
    A Transition between nodes in a Flow (source = outgoing, target = incoming).
    """

    parent: Union["Flow", None] = property_parent_()

    # meta
    type: FlowEdgeType = property_(30)
    source: "Action" = property_(35)
    target: "Action" = property_(36)
    if TYPE_CHECKING:
        source_ptr: Optional[NodeReference] = None
        source_id: Optional[UUID] = None
        target_ptr: Optional[NodeReference] = None
        target_id: Optional[UUID] = None

    # modulation
    # is_automap? (dynamically generate inputs?)
    # is_streaming: bool = property_(80, default=False)

    @property
    def flow(self) -> "Flow | None":
        """Gets the containing ancestor Flow (if any)"""
        from bench.language import Flow

        parent = self.parent
        while parent is not None:
            if isinstance(parent, Flow):
                return parent
        return None
