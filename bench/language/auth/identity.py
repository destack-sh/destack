from typing import TYPE_CHECKING, Any, Optional, Union, cast
from uuid import UUID

from bench.language.core import (
    ColorType,
    CustomObject,
    FieldType,
    InlineNode,
    IsInstantiable,
    IsModal,
    IsNamed,
    IsSubject,
    LocalNodeList,
    NodeReference,
    NodeType,
    StructType,
    node_,
    p_node_children,
    p_node_parent,
    p_regular,
    p_value_packed,
    p_value_runtime,
)
from bench.pb2 import IdentityData

if TYPE_CHECKING:
    from bench.language import Channel, Claim, Field, Flow, Page, Run, Text, Thread

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.IDENTITY)
class Identity(IsInstantiable, IsModal, IsSubject, IsNamed, InlineNode[IdentityData]):
    """The Identity of an 'Agent' (tied to a Flow). May run for another Flow than its own."""

    # meta
    parent: Union["Page", "Channel", "Thread", None] = p_node_parent(
        4, NodeType.PAGE, NodeType.CHANNEL, NodeType.THREAD
    )

    default_flow: Optional["Flow"] = p_regular(
        40,
        require=False,
        references=NodeType.FLOW,
        description="The default Flow backing this Identity.",
    )
    implemented_by: Optional["Run"] = p_regular(
        41,
        require=False,
        references=NodeType.RUN,
        description="The Run implementing this Identity.",
    )
    color: ColorType | None = p_regular(45)
    if TYPE_CHECKING:
        default_flow_ptr: Optional[NodeReference] = None
        default_flow_id: Optional[UUID] = None
        implemented_by_ptr: Optional[NodeReference] = None
        implemented_by_id: Optional[UUID] = None

    # content
    inputs_packed: Any = p_value_packed(61)
    inputs: "CustomObject | None" = p_value_runtime(
        61, type=FieldType.INPUT, typ=lambda self: cast("Run", self).input_type
    )
    outputs_packed: Any = p_value_packed(62)
    outputs: "CustomObject | None" = p_value_runtime(
        62, type=FieldType.OUTPUT, typ=lambda self: cast("Run", self).output_type
    )
    text: Optional["Text"] = p_regular(65, default=None, struct=StructType.TEXT)

    claims: LocalNodeList["Claim"] = p_node_children(NodeType.CLAIM)
    fields: LocalNodeList["Field"] = p_node_children(NodeType.FIELD)

    @staticmethod
    def new(name: str, **kwargs) -> "Identity":
        identity = Identity(name=name, **kwargs)
        return identity
