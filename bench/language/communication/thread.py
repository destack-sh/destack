from datetime import datetime
from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    InlineNode,
    IsInstantiable,
    IsModal,
    IsOwnable,
    IsRuntime,
    IsTimed,
    IsTitled,
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
    from bench.language import Channel, Message, Package, Page, Run

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
class Thread(
    IsTimed,
    IsOwnable,
    IsTitled,
    IsModal,
    IsRuntime,
    IsInstantiable,
    InlineNode[ThreadData],
):
    """
    A Thread for communicating with Messages on something.
    """

    # meta
    parent: Union["Package", "Page", "Channel", "Thread", "Run", None] = p_node_parent(
        4, NodeType.PACKAGE, NodeType.PAGE, NodeType.CHANNEL, NodeType.THREAD, NodeType.RUN
    )
    type: ThreadType = p_regular(30, require=True, default=ThreadType.SOURCE)

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
    run_root: Optional["Run"] = p_regular(
        42,
        require=False,
        array=False,
        same_bench=True,
        references=NodeType.RUN,
        description="The root Run this Thread is scoped to.",
    )
    run: Optional["Run"] = p_regular(
        43,
        require=False,
        array=False,
        same_bench=True,
        references=NodeType.RUN,
        description="The Run this Thread is scoped to.",
    )
    created_from: Optional["Message"] = p_regular(
        45, require=False, array=False, baseless=True, references=NodeType.MESSAGE, same_bench=True
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
        created_from_ptr: Optional[NodeReference] = None
        created_from_id: Optional[UUID] = None

    # status
    status: ThreadStatus = p_internal(50, default=ThreadStatus.OPEN)
    closed_at: Optional[datetime] = p_internal(55, default=None)

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
