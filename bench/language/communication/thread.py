from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    IsComputable,
    IsInstantiable,
    IsJoinable,
    IsModal,
    IsOwnable,
    IsProcessable,
    IsRuntime,
    IsTimed,
    IsTitled,
    LocalNodeList,
    NodeReference,
    NodeType,
    PageNode,
    RemoteNodeList,
    StructType,
    Text,
    TextIn,
    TextLineIn,
    enum_,
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
    from bench.language import (
        Agent,
        Channel,
        Claim,
        Cursor,
        Field,
        File,
        Link,
        Membership,
        Message,
        Package,
        Page,
        Plan,
    )

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.THREAD_STATUS)
class ThreadStatus(BuiltinEnum):
    OPEN = 10
    CLOSED = 30


@timed_node_(NodeType.THREAD)
class Thread(
    IsComputable,
    IsTimed,
    IsOwnable,
    IsProcessable,
    IsJoinable,
    IsTitled,
    IsModal,
    IsRuntime,
    IsInstantiable,
    PageNode[ThreadData],
):
    """
    A Thread for communicating with Messages.
    Threads may be nested to organize conversations and work.
    TODO :Incomplete: nested Threads, ThreadType & "monologue" threads
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
    if TYPE_CHECKING:
        channel_ptr: Optional[NodeReference] = None
        channel_id: Optional[UUID] = None

    # content
    page: Optional["Page"] = p_regular(
        60,
        require=False,
        array=False,
        references=NodeType.PAGE,
        same_bench=True,
        description="The main or root Page used by this Thread (may be shared).",
    )
    text: Optional["Text"] = p_regular(65, require=False, default=None, struct=StructType.TEXT)
    if TYPE_CHECKING:
        page_ptr: Optional[NodeReference] = None
        page_id: Optional[UUID] = None
        plan_ptr: Optional[NodeReference] = None
        plan_id: Optional[UUID] = None
        plan_ck: Optional[UUID] = None

    # ...IsProcessable[80-]

    messages: RemoteNodeList["Message", MessageData] = p_node_children(
        NodeType.MESSAGE, list=RemoteNodeList
    )
    agents: LocalNodeList["Agent"] = p_node_children(NodeType.AGENT)
    memberships: LocalNodeList["Membership"] = p_node_children(NodeType.MEMBERSHIP)
    fields: LocalNodeList["Field"] = p_node_children(NodeType.FIELD)
    plans: LocalNodeList["Plan"] = p_node_children(NodeType.PLAN)
    claims: LocalNodeList["Claim"] = p_node_children(NodeType.CLAIM)
    cursors: LocalNodeList["Cursor"] = p_node_children(NodeType.CURSOR)
    files: LocalNodeList["File"] = p_node_children(NodeType.FILE)
    links: LocalNodeList["Link"] = p_node_children(NodeType.LINK)

    @staticmethod
    def new(
        title: TextLineIn | None = None,
        text: TextIn | None = None,
        *,
        channel: "Channel | None" = None,
        page: "Page | None" = None,
        **kwargs,
    ) -> "Thread":
        thread = Thread(
            title=to_text_line(title) if title is not None else None,
            text=to_text(text) if text is not None else None,
            channel=channel,
            page=page,
            **kwargs,
        )
        return thread
