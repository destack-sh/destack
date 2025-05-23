from datetime import datetime
from typing import TYPE_CHECKING, Optional, Union

import structlog
from fastuuid import UUID

from bench.language.core import (
    UNSET,
    BuiltinEnum,
    EnumType,
    IsBased,
    IsComputable,
    IsDeletable,
    IsInPackage,
    IsModal,
    IsOwnable,
    IsSubject,
    IsTitled,
    Node,
    NodeType,
    ResourceStatus,
    Text,
    TextIn,
    TextLine,
    enum_,
    node_,
    p_internal,
    p_node_parent,
    property_,
    to_text,
)
from bench.pb2 import MessageData

if TYPE_CHECKING:
    from bench.language import Channel, NodeReference, Thread

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


@node_(NodeType.MESSAGE)
class Message(
    IsComputable,
    IsBased,
    IsOwnable,
    IsDeletable,
    IsTitled,
    IsModal,
    IsInPackage,
    Node[MessageData],
):
    """
    A Message about something (usually in a Thread or a Channel).
    """

    # meta
    parent: Union["Channel", "Thread", None] = p_node_parent()
    type: MessageType = property_(30, default=MessageType.DEFAULT)
    # platform? source?
    channel: Optional["Channel"] = property_(34, node_bench_from="self")
    thread: Optional["Thread"] = property_(35, node_bench_from="self")
    if TYPE_CHECKING:
        channel_id: Optional[UUID] = None
        channel_ptr: Optional[NodeReference] = None
        thread_id: Optional[UUID] = None
        thread_ptr: Optional[NodeReference] = None

    # status
    edited_at: Optional[datetime] = p_internal(40)

    # routing
    reply_to: Optional["Message"] = property_(50)
    forwarded_from: Optional["Message"] = property_(51)
    if TYPE_CHECKING:
        reply_to_ptr: Optional[NodeReference] = None
        reply_to_id: Optional[UUID] = None
        forwarded_from_ptr: Optional[NodeReference] = None
        forwarded_from_id: Optional[UUID] = None

    # content
    text: Optional["Text"] = property_(61)
    node: Optional["Node"] = property_(62)
    if TYPE_CHECKING:
        node_ptr: Optional[NodeReference] = None
        node_id: Optional[UUID] = None
    resource_status: Optional[ResourceStatus] = property_(64)

    def __content_str__(self) -> str:
        if self.title:
            return self.title.to_plain(max_characters=100)
        elif self.text:
            return self.text.to_plain(max_characters=100)
        else:
            return "<empty>"

    def edit(self, text: TextIn, nodes: list["Node"] = UNSET):
        """Edit the Message with new Text."""
        self.text = to_text(text)
        if nodes is not UNSET:
            self.nodes = nodes or []
        self.edited_at = self.active_session.oracle.utc()

    @staticmethod
    def new(
        text: TextIn | None = None,
        *,
        type: MessageType = MessageType.DEFAULT,
        title: TextLine | None = None,
        owned_by: Optional[IsSubject] = None,
        reply_to: Optional["Message"] = None,
        node: Optional["Node"] = None,
        **kwargs,
    ) -> "Message":
        raise NotImplementedError
