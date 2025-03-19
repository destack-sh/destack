from datetime import datetime
from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    InlineNode,
    IsInstantiable,
    IsJoinable,
    IsModal,
    IsOwnable,
    IsRuntime,
    IsTimed,
    IsTitled,
    LocalNodeList,
    Node,
    NodeReference,
    NodeType,
    RemoteNodeList,
    StructType,
    Text,
    enum_,
    p_internal,
    p_node_children,
    p_node_parent,
    p_regular,
    p_system,
    timed_node_,
)
from bench.language.core.const import INLINE_NODE_TYPES
from bench.pb2 import MessageData, ThreadData

if TYPE_CHECKING:
    from bench.language import Channel, Claim, Field, Membership, Message, Package, Page, Plan

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.THREAD_STATUS)
class ThreadStatus(BuiltinEnum):
    OPEN = 10
    CLOSED = 30


@timed_node_(NodeType.THREAD)
class Thread(
    IsTimed,
    IsOwnable,
    IsJoinable,
    IsTitled,
    IsModal,
    IsRuntime,
    IsInstantiable,
    InlineNode[ThreadData],
):
    """
    A Thread for communicating with Messages.
    """

    # meta
    parent: Union["Package", "Page", "Channel", "Thread", None] = p_node_parent(
        4, NodeType.PACKAGE, NodeType.PAGE, NodeType.CHANNEL, NodeType.THREAD
    )
    scope: Union["InlineNode", "Package"] = p_regular(
        40, require=False, references=(*INLINE_NODE_TYPES, NodeType.PACKAGE)
    )
    channel: Optional["Channel"] = p_system(
        42,
        require=False,
        array=False,
        same_bench=True,
        references=NodeType.CHANNEL,
    )
    created_from: Optional["Message"] = p_regular(
        45, require=False, array=False, baseless=True, references=NodeType.MESSAGE, same_bench=True
    )
    if TYPE_CHECKING:
        scope_ptr: Optional[NodeReference] = None
        scope_id: Optional[UUID] = None
        page_ptr: Optional[NodeReference] = None
        page_id: Optional[UUID] = None
        channel_ptr: Optional[NodeReference] = None
        channel_id: Optional[UUID] = None
        created_from_ptr: Optional[NodeReference] = None
        created_from_id: Optional[UUID] = None

    # status
    status: ThreadStatus = p_internal(50, default=ThreadStatus.OPEN)
    closed_at: Optional[datetime] = p_internal(55, default=None)

    # content
    main_page: Optional["Page"] = p_regular(
        60,
        require=False,
        array=False,
        references=NodeType.PAGE,
        same_bench=True,
        description="The main or root Page used by this Thread (may be shared).",
    )
    main_plan: Optional["Plan"] = p_regular(
        61,
        require=False,
        array=False,
        references=NodeType.PLAN,
        same_bench=True,
        description="The main Plan to consider in this Thread (may be on the Page).",
    )
    text: Optional["Text"] = p_regular(65, require=False, default=None, struct=StructType.TEXT)

    messages: RemoteNodeList["Message", MessageData] = p_node_children(
        NodeType.MESSAGE, list=RemoteNodeList
    )
    memberships: LocalNodeList["Membership"] = p_node_children(NodeType.MEMBERSHIP)
    fields: LocalNodeList["Field"] = p_node_children(NodeType.FIELD)
    plans: LocalNodeList["Plan"] = p_node_children(NodeType.PLAN)
    claims: LocalNodeList["Claim"] = p_node_children(NodeType.CLAIM)

    @property
    def container(self) -> "Node | None":
        parent = self.parent
        if isinstance(parent, Thread):
            return parent
        else:
            return self.channel
