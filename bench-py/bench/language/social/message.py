from datetime import datetime
from typing import TYPE_CHECKING, Optional, Union

import structlog
from fastuuid import UUID

from bench.language.core import (
    UNSET,
    BuiltinEnum,
    EnumType,
    HasTitle,
    IsDeletable,
    IsEnvironmental,
    IsInPackage,
    IsOwnable,
    Node,
    NodeType,
    Text,
    TextIn,
    enum_,
    node_,
    property_,
    property_parent_,
    to_text,
)
from bench.pb2 import MessageData

if TYPE_CHECKING:
    from bench.language import NodeReference, Thread

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
    IsOwnable,
    IsDeletable,
    HasTitle,
    IsEnvironmental,
    IsInPackage,
    Node[MessageData],
):
    """
    A Message about something (usually in a Thread or a Channel).
    """

    # meta
    parent: Union["Thread", None] = property_parent_(node_is_customizable=True)
    type: MessageType = property_(30, default=MessageType.DEFAULT, is_repr=True)
    # platform? source?
    thread: Optional["Thread"] = property_(35, node_space_from="self")
    if TYPE_CHECKING:
        thread_id: Optional[UUID] = None
        thread_ptr: Optional[NodeReference] = None

    # status
    edited_at: Optional[datetime] = property_(40)

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

    def edit(self, text: TextIn, nodes: list["Node"] = UNSET):
        """Edit the Message with new Text."""
        self.text = to_text(text)
        if nodes is not UNSET:
            self.nodes = nodes or []
        self.edited_at = self._session.oracle.utc()
