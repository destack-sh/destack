from datetime import datetime
from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    InlineNode,
    IsOwnable,
    IsTimed,
    IsTitled,
    Node,
    NodeReference,
    NodeType,
    PackageNode,
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
from bench.pb2 import MessageData, ThreadData

if TYPE_CHECKING:
    from bench.language import Channel, Message, Package, Run

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.THREAD_TYPE)
class ThreadType(BuiltinEnum):
    SOURCE = 1
    RUN = 2


@enum_(EnumType.THREAD_STATUS)
class ThreadStatus(BuiltinEnum):
    OPEN = 10
    CLOSED = 30


@timed_node_(NodeType.THREAD, has_subtypes=True)
class Thread(IsTimed, IsOwnable, IsTitled, PackageNode[ThreadData]):
    """
    A Thread for communicating with Messages on something.
    """

    # meta
    parent: Union["Channel", "Thread", None] = p_node_parent(4, NodeType.CHANNEL, NodeType.THREAD)
    type: ThreadType = p_regular(30, require=True, default=ThreadType.SOURCE)
    channel: "Channel" = p_system(
        33,
        require=True,
        array=False,
        same_bench=True,
        references=NodeType.CHANNEL,
    )
    scope: Union["InlineNode", "Package"] = p_regular(
        35, require=False, references=(NodeType.PAGE, NodeType.PACKAGE)
    )
    run_root: Optional["Run"] = p_regular(
        36,
        require=False,
        array=False,
        same_bench=True,
        references=NodeType.RUN,
        description="The root Run this Thread is scoped to.",
    )
    run: Optional["Run"] = p_regular(
        37,
        require=False,
        array=False,
        same_bench=True,
        references=NodeType.RUN,
        description="The Run this Thread is scoped to.",
    )
    if TYPE_CHECKING:
        channel_ptr: Optional[NodeReference] = None
        channel_id: Optional[UUID] = None
        scope_ptr: Optional[NodeReference] = None
        scope_id: Optional[UUID] = None
        run_root_ptr: Optional[NodeReference] = None
        run_root_id: Optional[UUID] = None
        run_ptr: Optional[NodeReference] = None
        run_id: Optional[UUID] = None

    # status
    status: ThreadStatus = p_internal(40, default=ThreadStatus.OPEN)
    closed_at: Optional[datetime] = p_internal(45, default=None)

    # routing
    created_from: Optional["Message"] = p_regular(
        50, require=False, array=False, baseless=True, references=NodeType.MESSAGE, same_bench=True
    )

    # content
    text: Optional["Text"] = p_regular(61, require=False, default=None, struct=StructType.TEXT)

    messages: RemoteNodeList["Message", MessageData] = p_node_children(
        NodeType.MESSAGE, list=RemoteNodeList
    )

    @property
    def container(self) -> "Node | None":
        parent = self.parent
        if isinstance(parent, Thread):
            return parent
        else:
            return self.channel
