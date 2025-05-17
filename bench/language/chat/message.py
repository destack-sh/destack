from datetime import datetime
from typing import TYPE_CHECKING, Any, Optional, Union, cast
from uuid import UUID

import structlog

from bench.language.core import (
    UNSET,
    BuiltinEnum,
    EnumType,
    IsBased,
    IsComputable,
    IsModal,
    IsOwnable,
    IsTitled,
    Node,
    NodeType,
    PackageNode,
    PageNode,
    ResourceStatus,
    Subject,
    Text,
    TextIn,
    TextLine,
    enum_,
    p_internal,
    p_node_parent,
    p_regular,
    text_line,
    timed_node_,
    to_text,
)
from bench.pb2 import AnyNodeData, MessageData, NodeReferenceData

if TYPE_CHECKING:
    from bench.language import Channel, NodeReference, Package, Thread

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)


@enum_(EnumType.MESSAGE_TYPE)
class MessageType(BuiltinEnum):
    DEFAULT = 1, "Default", "Regular text (and nodes)", "fas fa-envelope"
    # FORWARDED = 3, "Forwarded", "Forwarded Message", "fas fa-forward"
    JOIN = 10, "Join", "Join a chat", "fas fa-arrow-right-to-bracket"
    LEAVE = 11, "Leave", "Leave a chat", "fas fa-arrow-left-from-line"
    RESOURCE = 20, "Resource", "Resource update", "fas fa-plug"
    RUN = 100, "Run", None, "fas fa-play"
    THREAD = 110, "Thread", "Thread inside a chat", "fas fa-thread"
    # EDIT, STREAM, ...
    # also see https://discord.com/developers/docs/resources/message


@timed_node_(NodeType.MESSAGE)
class Message(
    IsComputable,
    IsBased,
    IsOwnable,
    IsTitled,
    IsModal,
    PackageNode[MessageData],
):
    """
    A Message about something (usually in a Thread or a Channel).
    """

    # meta
    parent: Union["Channel", "Thread", None] = p_node_parent(
        4, NodeType.CHANNEL, NodeType.THREAD, ckless=True
    )
    type: MessageType = p_regular(30, default=MessageType.DEFAULT)
    # platform? source?
    channel: Optional["Channel"] = p_regular(34, same_bench=True)
    thread: Optional["Thread"] = p_regular(35, same_bench=True)
    scope: Union["PageNode", "Package"] = p_regular(36)
    if TYPE_CHECKING:
        channel_id: Optional[UUID] = None
        channel_ptr: Optional[NodeReference] = None
        thread_id: Optional[UUID] = None
        thread_ptr: Optional[NodeReference] = None
        scope_id: Optional[UUID] = None
        scope_ptr: Optional[NodeReference] = None

    # status
    edited_at: Optional[datetime] = p_internal(40)

    # routing
    reply_to: Optional["Message"] = p_regular(50, baseless=True)
    forwarded_from: Optional["Message"] = p_regular(51, baseless=True)
    if TYPE_CHECKING:
        reply_to_ptr: Optional[NodeReference] = None
        reply_to_id: Optional[UUID] = None
        forwarded_from_ptr: Optional[NodeReference] = None
        forwarded_from_id: Optional[UUID] = None

    # content
    text: Optional["Text"] = p_regular(61)
    nodes: list["Node"] = p_regular(
        63,
        description="The Nodes this Message is about.",
    )
    if TYPE_CHECKING:
        nodes_ptr: Optional[NodeReference] = None
        nodes_id: Optional[UUID] = None
    resource_status: Optional[ResourceStatus] = p_regular(64)

    def __content_str__(self) -> str:
        if self.title:
            return self.title.to_plain(max_characters=100)
        elif self.text:
            return self.text.to_plain(max_characters=100)
        else:
            return "<empty>"

    @property
    def base(self):
        return self.thread

    @staticmethod
    def get_base_from_data(data: AnyNodeData) -> Optional[NodeReferenceData]:
        return cast("MessageData", data).thread_ptr

    @staticmethod
    def get_base_from_partial(data: dict[str, Any]) -> Optional["Thread"]:
        if "thread" in data:
            return data["thread"]
        else:
            return None

    def edit(self, text: TextIn, nodes: list["Node"] = UNSET):
        """Edit the Message with new Text."""
        self.text = to_text(text)
        if nodes is not UNSET:
            self.nodes = nodes or []
        self.edited_at = self.active_session._oracle.utc()

    @staticmethod
    def new(
        text: TextIn | None = None,
        *,
        type: MessageType = MessageType.DEFAULT,
        title: TextLine | None = None,
        owned_by: Optional[Subject] = None,
        scope: Optional["PageNode"] = None,
        reply_to: Optional["Message"] = None,
        nodes: list["Node"] | None = None,
        **kwargs,
    ) -> "Message":
        message = Message(
            type=type,
            title=text_line(title) if title is not None else None,
            text=to_text(text) if text is not None else None,
            reply_to=reply_to,
            owned_by=owned_by,
            **kwargs,
        )
        if nodes is not None:
            message.nodes = nodes
        if scope is not None:
            message.scope = scope
        return message
