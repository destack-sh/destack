from typing import TYPE_CHECKING, Optional
from uuid import UUID

from bench.language.core import (
    BuiltinObject,
    IsRuntime,
    NodeType,
    Property,
    Struct,
    StructType,
    node_component_,
    p_internal,
    p_regular,
    struct_,
)

if TYPE_CHECKING:
    from bench.language import (
        Action,
        Agent,
        Flow,
        Kit,
        Link,
        Message,
        NodeReference,
        Page,
        Plan,
        Run,
        Session,
        Span,
        Task,
        Trigger,
    )

# pyright: reportIncompatibleVariableOverride=false


@node_component_()
class IsRun(BuiltinObject):
    """Context for a Node that's in a Run."""

    # context
    flow: Optional["Flow"] = p_internal(
        70,
        require=False,
        array=False,
        references=NodeType.FLOW,
        same_bench=True,
        description="The Flow the Action is in.",
    )
    kit: Optional["Kit"] = p_internal(
        71,
        require=False,
        array=False,
        references=NodeType.KIT,
        same_bench=True,
        description="The Kit the Action is in.",
    )
    action: Optional["Action"] = p_internal(
        72,
        require=False,
        array=False,
        references=NodeType.ACTION,
        same_bench=True,
        description="The Action this Run is executing.",
    )
    link: Optional["Link"] = p_internal(
        73,
        require=False,
        array=False,
        references=NodeType.LINK,
        same_bench=True,
        description="The Link this Run is executing.",
    )
    plan: Optional["Plan"] = p_internal(
        74,
        require=False,
        array=False,
        references=NodeType.PLAN,
        same_bench=True,
        description="The Plan this Run is following (leaf).",
    )
    task: Optional["Task"] = p_internal(
        75,
        require=False,
        array=False,
        references=NodeType.TASK,
        same_bench=True,
        description="The Task this Run is implementing.",
    )
    trigger: Optional["Trigger"] = p_regular(
        76,
        require=False,
        array=False,
        references=NodeType.TRIGGER,
        same_bench=True,
        description="The Trigger this Run is triggered by.",
    )
    trigger_key: Optional[str] = p_internal(
        77,
        require=False,
        default=None,
        description="A unique key for this invocation of the Trigger.",
    )
    message: Optional["Message"] = p_internal(
        78,
        require=False,
        array=False,
        references=NodeType.MESSAGE,
        same_bench=True,
    )
    if TYPE_CHECKING:
        flow_ptr: Optional[NodeReference] = None
        flow_id: Optional[UUID] = None
        kit_ptr: Optional[NodeReference] = None
        kit_id: Optional[UUID] = None
        action_ptr: Optional[NodeReference] = None
        action_id: Optional[UUID] = None
        action_ck: Optional[UUID] = None
        link_ptr: Optional[NodeReference] = None
        link_id: Optional[UUID] = None
        incoming_ptr: tuple["NodeReference", ...] = ()
        trigger_ptr: Optional[NodeReference] = None
        trigger_id: Optional[UUID] = None
        message_ptr: Optional[NodeReference] = None
        message_id: Optional[UUID] = None

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
