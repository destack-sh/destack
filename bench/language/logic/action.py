from typing import TYPE_CHECKING, Optional, Union, cast
from uuid import UUID

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    IsClaimable,
    IsExtensible,
    IsInstantiable,
    IsModal,
    IsNamed,
    IsOrdered,
    IsRunnable,
    NodeReference,
    NodeType,
    PackageNode,
    RunType,
    enum_,
    node_,
    p_internal,
    p_node_parent,
    p_regular,
)
from bench.pb2 import ActionData

if TYPE_CHECKING:
    from bench.language import (
        Action,
        Agent,
        Code,
        Flow,
        FlowEdge,
        FlowEdgeType,
        Icon,
        NodeReference,
        Service,
        Text,
    )

# pyright: reportIncompatibleVariableOverride=false

_type = type


@enum_(EnumType.ACTION_TYPE)
class ActionType(BuiltinEnum):
    # orchestrate
    START = 10, "Start", "Begin the Flow", "fas fa-circle-play"
    END = 20, "End", "Complete the Flow", "fas fa-flag-checkered"
    # WAIT = 30, "Wait", "Wait for some trigger"

    # action
    TOOL = 100, "Tool", "Delegate to a specific tool", "fas fa-screwdriver-wrench"
    CODE = 101, "Code", "Run some Code", "fas fa-code"
    BUILTIN = 102, "Builtin", "Run a builtin", "fas fa-cogs"

    # containers
    # GROUP, LOOP, ...

    @property
    def is_boundary(self) -> bool:
        return self < 40


@node_(NodeType.ACTION)
class Action(
    IsClaimable,
    IsInstantiable,
    IsExtensible,
    IsNamed,
    IsModal,
    IsOrdered,
    IsRunnable,
    PackageNode[ActionData],
):
    """
    An implementation of a unit of work, usually expressed with Code or some tool.
    May defer to a builtin or some other service in a separate system.
    """

    parent: Union["Flow", "Service", None] = p_node_parent(4, NodeType.FLOW, NodeType.SERVICE)

    # common
    type: ActionType = p_regular(30, description="Type of this Action. Only dynamic for tools.")
    icon: Optional["Icon"] = p_regular(35)
    text: Optional["Text"] = p_regular(36)

    # content
    code: Optional["Code"] = p_internal(
        52,
        default=None,
        description="The implementation code for this action.",
    )
    tool: Union["Agent", "Flow", "Action", None] = p_regular(
        53,
        description="The implementation for this action.",
    )
    if TYPE_CHECKING:
        tool_ptr: "NodeReference | None" = None
        tool_id: Optional[UUID] = None
        tool_ck: Optional[UUID] = None

    @property
    def flow(self) -> "Flow | None":
        """Gets the containing ancestor Flow (if any)"""
        from bench.language import Flow

        parent = self.parent
        while parent is not None:
            if isinstance(parent, Flow):
                return parent
            parent = parent.parent
        return None

    @property
    def service(self) -> "Service | None":
        """Gets the containing ancestor Service (if any)"""
        from bench.language import Service

        parent = self.parent
        while parent is not None:
            if isinstance(parent, Service):
                return parent
            parent = parent.parent
        return None

    def run_type(self) -> RunType:
        return RunType.ACTION

    def connect(
        self,
        type: "FlowEdgeType",
        target: "Action",
        name: str | None = None,
        *,
        parent: Union["Flow", "Service", None] = None,
    ) -> "FlowEdge":
        """Connects a target Action to this Action."""
        from bench.language import Flow, FlowEdge

        parent = parent or self.parent
        assert parent is not None, f"{self!r} is not attached to a parent"
        assert isinstance(parent, Flow), f"{parent!r} is not valid for {self!r}"

        # assign next name like Pipe1, .. in parent :AutoNaming
        if name is None:
            siblings = parent.get_children(FlowEdge)
            count = len(siblings) + 1
            name = f"{type.bench_name}{count}"
            while any(p.name == name for p in siblings):
                count += 1
                name = f"{type.bench_name}{count}"

        transition = FlowEdge(
            type=type,
            name=name,
            source=self,
            target=target,
            parent=parent,
        )
        parent.add_child(transition)
        return transition

    @staticmethod
    def new[ActionT: "Action" = "Action"](
        typ: Union["Action", ActionType, _type[ActionT]],
        name: str | None = None,
        **kwargs,
    ) -> ActionT:
        """Creates a new Action of the given type."""
        if isinstance(typ, Action):
            action = Action(type=ActionType.TOOL, tool=typ, name=name or typ.name, **kwargs)
            return cast(ActionT, action)
        elif isinstance(typ, type):
            typ = Action.__subtype_by_subclass__[typ]  # type: ignore
        action = Action(
            type=cast(ActionType, typ), name=name or cast(ActionType, typ).bench_name, **kwargs
        )
        return cast(ActionT, action)
