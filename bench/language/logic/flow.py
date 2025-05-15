from functools import cached_property
from typing import TYPE_CHECKING, Literal, Optional, Union
from uuid import UUID

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    FieldType,
    IsClaimable,
    IsModal,
    IsNamed,
    IsOrdered,
    IsOwnable,
    IsRunnable,
    IsTemplatable,
    LocalNodeList,
    NodeType,
    PackageNode,
    PageNode,
    RunType,
    TypeBase,
    TypeKind,
    enum_,
    node_,
    p_internal,
    p_node_children,
    p_node_parent,
    p_regular,
)
from bench.pb2 import FlowData, FlowEdgeData

if TYPE_CHECKING:
    from bench.language import (
        Action,
        Claim,
        Flow,
        NodeReference,
    )

# pyright: reportIncompatibleVariableOverride=false

if TYPE_CHECKING:
    from bench.language import Action, Agent, Claim, Field, FlowEdge, Page

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
    IsRunnable,
    PageNode[FlowData],
):
    """A Flow orchestrates a sequence of steps (like Actions)."""

    parent: Union["Page", "Agent", None] = p_node_parent(4, NodeType.PAGE, NodeType.AGENT)
    type: FlowType = p_regular(30, default=FlowType.ACTION, default_sql=None)

    actions: LocalNodeList["Action"] = p_node_children(NodeType.ACTION)
    transitions: LocalNodeList["FlowEdge"] = p_node_children(NodeType.FLOW_EDGE)
    fields: LocalNodeList["Field"] = p_node_children(NodeType.FIELD)
    claims: LocalNodeList["Claim"] = p_node_children(NodeType.CLAIM)

    def __content_str__(self):
        return ""

    def to_type_maybe(
        self,
        *,
        of: Literal["instance", "value"] = "instance",
        field_types: list[FieldType] | None = None,
    ) -> "TypeBase":
        """Get a type represented by this Block (if any)"""
        from bench.language.core import Type

        if of == "instance":
            return Type(kind=TypeKind.BASED_NODE, base_type=self, bench_type=NodeType.RUN)
        else:
            field_types = field_types or []
            return Type(
                kind=TypeKind.CUSTOM_OBJECT,
                base_type=self,
                base_field_types=field_types,
                property_field_types=field_types,
            )

    def to_type(
        self,
        *,
        of: Literal["instance", "value"] = "instance",
        field_types: list[FieldType] | None = None,
    ) -> "TypeBase":
        typ = self.to_type_maybe(of=of, field_types=field_types)
        if typ is None:
            raise ValueError(f"{self!r} does not have a type")
        return typ

    @cached_property  # :CachedTypeInfo
    def input_type(self) -> "TypeBase | None":
        return self.to_type_maybe(of="value", field_types=[FieldType.INPUT])

    @cached_property  # :CachedTypeInfo
    def output_type(self) -> "TypeBase | None":
        return self.to_type_maybe(of="value", field_types=[FieldType.OUTPUT])

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


SIGN_BY_LINK_TYPE: dict[FlowEdgeType, str] = {
    FlowEdgeType.MANUAL: "-!>",
    FlowEdgeType.DECIDE: "-*>",
    FlowEdgeType.REQUIRE: "-=>",
}
LINK_TYPES_BY_SIGN: dict[str, FlowEdgeType] = {v: k for k, v in SIGN_BY_LINK_TYPE.items()}


@node_(NodeType.FLOW_EDGE)
class FlowEdge(
    IsTemplatable,
    IsModal,
    IsRunnable,
    IsNamed,
    IsOrdered,
    PackageNode[FlowEdgeData],
):
    """
    A Transition between nodes in a Flow (source = outgoing, target = incoming).
    NOTE :Architecture: maybe add IsTransitionable trait?
    """

    parent: Union["Flow", None] = p_node_parent(4, NodeType.FLOW)

    # meta
    type: FlowEdgeType = p_internal(30)
    source: "Action" = p_regular(35, require=True, references=NodeType.ACTION, ckless=True)
    target: "Action" = p_regular(36, require=True, references=NodeType.ACTION, ckless=True)
    if TYPE_CHECKING:
        source_ptr: Optional[NodeReference] = None
        source_id: Optional[UUID] = None
        target_ptr: Optional[NodeReference] = None
        target_id: Optional[UUID] = None

    # modulation
    # is_automap? (dynamically generate inputs?)
    # is_streaming: bool = p_regular(80, default=False)

    def __content_str__(self) -> str:
        sign = SIGN_BY_LINK_TYPE.get(self.type, "???")
        source = self.source
        target = self.target
        return f"{source.absolute_path if source else '???'} {sign} {target.absolute_path if target else '???'}"

    @property
    def run_type(self) -> RunType:
        return RunType.TRANSITION

    @property
    def flow(self) -> "Flow | None":
        """Gets the containing ancestor Flow (if any)"""
        from bench.language import Flow

        parent = self.parent
        while parent is not None:
            if isinstance(parent, Flow):
                return parent
        return None

    @property
    def claims(self) -> tuple["Claim", ...]:
        return ()

    def to_type_maybe(
        self,
        of: Literal["instance", "value"] = "instance",
        field_types: list[FieldType] | None = None,
    ) -> "TypeBase | None":
        return None

    @property
    def resource_type(self) -> "TypeBase | None":
        return None  # Links don't have resources (?)

    @property
    def input_type(self) -> "TypeBase | None":
        return None  # Links don't have inputs (?)

    @property
    def output_type(self) -> "TypeBase | None":
        return None  # Links don't have outputs (?)

    @staticmethod
    def new(type: FlowEdgeType, name: str, **kwargs) -> "FlowEdge":
        return FlowEdge(type=type, name=name, **kwargs)
