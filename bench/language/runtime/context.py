from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

from bench.language.core import (
    BuiltinObject,
    IsRuntime,
    NodeType,
    Property,
    Struct,
    StructType,
    bittuple,
    node_component_,
    p_internal,
    struct_,
)

if TYPE_CHECKING:
    from bench.language import (
        Action,
        Agent,
        Flow,
        FlowEdge,
        Message,
        NodeReference,
        Page,
        Run,
        Service,
        Session,
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
        require=False,
        array=False,
        references=NodeType.AGENT,
        same_bench=True,
        description="The Agent we're running as.",
    )
    flow: Optional["Flow"] = p_internal(
        71,
        require=False,
        array=False,
        references=NodeType.FLOW,
        same_bench=True,
        description="The Flow the Action is in.",
    )
    service: Optional["Service"] = p_internal(
        72,
        require=False,
        array=False,
        references=NodeType.SERVICE,
        same_bench=True,
        description="The Service the Action is in.",
    )
    action: Optional["Action"] = p_internal(
        73,
        require=False,
        array=False,
        references=NodeType.ACTION,
        same_bench=True,
        description="The Action this Run is executing.",
    )
    transition: Optional["FlowEdge"] = p_internal(
        74,
        require=False,
        array=False,
        references=NodeType.FLOW_EDGE,
        same_bench=True,
        description="The Transition this Run is executing.",
    )
    task: Optional["Task"] = p_internal(
        78,
        require=False,
        array=False,
        references=NodeType.TASK,
        same_bench=True,
        description="The manual Task this Run is implementing (leaf).",
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
        transition_ptr: Optional[NodeReference] = None
        transition_id: Optional[UUID] = None
        task_ptr: Optional[NodeReference] = None
        task_id: Optional[UUID] = None

    def _copy_context_to(self, span: "Span"):
        """Copy context from this HasRunContext to a Span."""
        for prop in IsRun.__declared_properties__.values():
            if type(prop.reference_wired_ptr) is Property:
                prop = prop.reference_wired_ptr
            prop_value = getattr(self, prop.name)
            if prop_value is not None:
                setattr(span, prop.name, prop_value)


@struct_(StructType.EDIT_CONTEXT)
class EditContext(Struct):
    """Additional context for a specific edit (per-edit variable subset of Session context)."""

    page: Optional["Page"] = p_internal(70, require=False, array=False, references=NodeType.PAGE)
    flow: Optional["Flow"] = p_internal(71, require=False, array=False, references=NodeType.FLOW)
    action: Optional["Action"] = p_internal(
        72, require=False, array=False, references=NodeType.ACTION
    )
    session: Optional["Session"] = p_internal(
        73, require=False, array=False, references=NodeType.SESSION, same_bench=True
    )
    run: Optional["Run"] = p_internal(
        74, require=False, array=False, references=NodeType.RUN, same_bench=True
    )
    run_root: Optional["Run"] = p_internal(
        75, require=False, array=False, references=NodeType.RUN, same_bench=True
    )
    agent: Optional["Agent"] = p_internal(76, require=False, array=False, references=NodeType.AGENT)
    if TYPE_CHECKING:
        block_ptr: Optional[NodeReference] = None
        action_ptr: Optional[NodeReference] = None
        session_ptr: Optional[NodeReference] = None
        run_ptr: Optional[NodeReference] = None
        run_root_ptr: Optional[NodeReference] = None
        agent_ptr: Optional[NodeReference] = None


@struct_(StructType.CONTEXT)
class Context(Struct, IsRuntime):
    """Context information for runtime nodes created in a session."""

    pass
