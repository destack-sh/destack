from typing import TYPE_CHECKING, Optional, Union

from fastuuid import UUID

from bench.language.core import (
    BuiltinObject,
    NodeType,
    Property,
    bittuple,
    node_component_,
    p_internal,
)

if TYPE_CHECKING:
    from bench.language import (
        Action,
        Agent,
        Flow,
        Message,
        NodeReference,
        Service,
        Span,
        Task,
    )

# pyright: reportIncompatibleVariableOverride=false

RunTarget = Union["Message", "Task"]
RUN_TARGET_TYPES = bittuple(NodeType.MESSAGE, NodeType.TASK)


@node_component_()
class IsRun(BuiltinObject):
    """Context for a Node that's in a Run."""

    # context
    agent: Optional["Agent"] = p_internal(
        70,
        node_bench_from="self",
        description="The Agent we're running as.",
    )
    flow: Optional["Flow"] = p_internal(
        71,
        node_bench_from="self",
        description="The Flow the Action is in.",
    )
    service: Optional["Service"] = p_internal(
        72,
        node_bench_from="self",
        description="The Service the Action is in.",
    )
    action: Optional["Action"] = p_internal(
        73,
        node_bench_from="self",
        description="The Action this Run is executing.",
    )
    task: Optional["Task"] = p_internal(
        78,
        node_bench_from="self",
        description="The Task this Run is executing.",
    )
    if TYPE_CHECKING:
        agent_ptr: Optional[NodeReference] = None
        agent_id: Optional[UUID] = None
        flow_ptr: Optional[NodeReference] = None
        flow_id: Optional[UUID] = None
        service_ptr: Optional[NodeReference] = None
        service_id: Optional[UUID] = None
        action_ptr: Optional[NodeReference] = None
        action_id: Optional[UUID] = None
        action_ck: Optional[UUID] = None
        task_ptr: Optional[NodeReference] = None
        task_id: Optional[UUID] = None

    def _copy_context_to(self, span: "Span"):
        """Copy context from this HasRunContext to a Span."""
        for prop in IsRun.__declared_properties__.values():
            if type(prop.ptr_prop) is Property:
                prop = prop.ptr_prop
            prop_value = getattr(self, prop.name)
            if prop_value is not None:
                setattr(span, prop.name, prop_value)
