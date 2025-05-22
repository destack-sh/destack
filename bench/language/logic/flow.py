from typing import TYPE_CHECKING, Optional, Union

from fastuuid import UUID

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    IsArchivable,
    IsBlockable,
    IsClaimable,
    IsDeletable,
    IsExtensible,
    IsInPackage,
    IsModal,
    IsNamed,
    IsOrdered,
    IsOwnable,
    IsRunnable,
    IsTemplatable,
    Node,
    NodeType,
    enum_,
    node_,
    p_internal,
    p_node_parent,
    p_regular,
)
from bench.pb2 import FlowData, FlowEdgeData

if TYPE_CHECKING:
    from bench.language import (
        Action,
        Flow,
        NodeReference,
    )

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
    IsClaimable,
    IsModal,
    IsNamed,
    IsExtensible,
    IsRunnable,
    IsBlockable,
    IsDeletable,
    IsArchivable,
    Node[FlowData],
):
    """A Flow orchestrates a sequence of steps (like Actions)."""

    parent: Union["Page", "Agent", None] = p_node_parent()
    type: FlowType = p_regular(30, default=FlowType.ACTION)

    def __content_str__(self):
        return ""

    @staticmethod
    def new(name: str, **kwargs) -> "Flow":
        flow = Flow(name=name, **kwargs)
        return flow


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
    IsNamed,
    IsOrdered,
    IsInPackage,
    Node[FlowEdgeData],
):
    """
    A Transition between nodes in a Flow (source = outgoing, target = incoming).
    """

    parent: Union["Flow", None] = p_node_parent()

    # meta
    type: FlowEdgeType = p_internal(30)
    source: "Action" = p_regular(35)
    target: "Action" = p_regular(36)
    if TYPE_CHECKING:
        source_ptr: Optional[NodeReference] = None
        source_id: Optional[UUID] = None
        target_ptr: Optional[NodeReference] = None
        target_id: Optional[UUID] = None

    # modulation
    # is_automap? (dynamically generate inputs?)
    # is_streaming: bool = p_regular(80, default=False)

    def __content_str__(self) -> str:
        source = self.source
        target = self.target
        return f"{source.absolute_path if source else '???'} {self.type.name} {target.absolute_path if target else '???'}"

    @property
    def flow(self) -> "Flow | None":
        """Gets the containing ancestor Flow (if any)"""
        from bench.language import Flow

        parent = self.parent
        while parent is not None:
            if isinstance(parent, Flow):
                return parent
        return None

    @staticmethod
    def new(type: FlowEdgeType, name: str, **kwargs) -> "FlowEdge":
        return FlowEdge(type=type, name=name, **kwargs)
