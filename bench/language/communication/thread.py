from datetime import datetime
from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

from bench.language.core import (
    INLINE_NODE_TYPES,
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
    TextIn,
    TextLineIn,
    enum_,
    p_internal,
    p_node_children,
    p_node_parent,
    p_regular,
    p_system,
    timed_node_,
    to_text,
    to_text_line,
)
from bench.pb2 import MessageData, ThreadData

if TYPE_CHECKING:
    from bench.language import Channel, Claim, Field, File, Membership, Message, Package, Page, Plan

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
    channel: Optional["Channel"] = p_system(
        40,
        require=False,
        array=False,
        same_bench=True,
        references=NodeType.CHANNEL,
    )
    scope: Union["InlineNode", "Package", None] = p_regular(
        41, require=False, references=(*INLINE_NODE_TYPES, NodeType.PACKAGE)
    )
    if TYPE_CHECKING:
        channel_ptr: Optional[NodeReference] = None
        channel_id: Optional[UUID] = None
        scope_ptr: Optional[NodeReference] = None
        scope_id: Optional[UUID] = None

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
    files: LocalNodeList["File"] = p_node_children(NodeType.FILE)

    @property
    def container(self) -> "Node | None":
        parent = self.parent
        if isinstance(parent, Thread):
            return parent
        else:
            return self.channel

    @staticmethod
    def new(
        title: TextLineIn | None = None,
        text: TextIn | None = None,
        *,
        channel: "Channel | None" = None,
        scope: Union["InlineNode", "Package", None] = None,
        main_page: "Page | None" = None,
        main_plan: "Plan | None" = None,
        **kwargs,
    ) -> "Thread":
        thread = Thread(
            title=to_text_line(title) if title is not None else None,
            text=to_text(text) if text is not None else None,
            scope=scope,
            channel=channel,
            main_page=main_page,
            main_plan=main_plan,
            **kwargs,
        )
        return thread
