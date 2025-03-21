from datetime import datetime
from typing import TYPE_CHECKING, Any, Optional, Union, cast
from uuid import UUID

import structlog

from bench.language.core import (
    INLINE_NODE_TYPES,
    BuiltinEnum,
    EnumType,
    FieldType,
    InlineNode,
    IsBased,
    IsModal,
    IsTimed,
    IsTitled,
    Node,
    NodeType,
    PackageNode,
    StructType,
    Text,
    TextLine,
    TypeBase,
    enum_,
    p_internal,
    p_node_ancestor,
    p_node_parent,
    p_regular,
    p_system,
    p_value_packed,
    p_value_runtime,
    text_line,
    timed_node_,
)
from bench.pb2 import AnyNodeData, MessageData, NodeReferenceData

if TYPE_CHECKING:
    from bench.language import (
        Channel,
        Class,
        Interruption,
        NodeReference,
        Package,
        Run,
        Thread,
    )

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)


@enum_(EnumType.MESSAGE_TYPE)
class MessageType(BuiltinEnum):
    REGULAR = 1, "Regular", "Standard Message", "fas fa-envelope"
    REPLY = 2, "Reply", "Reply to another Message", "fas fa-reply"
    # FORWARDED = 3, "Forwarded", "Forwarded Message", "fas fa-forward"
    RUN = 100, "Run", "Begin a Run", "fas fa-play"
    INTERRUPTION = 110, "Interruption", "Interrupt a Run", "fas fa-hand"
    # POLL = 130, "Poll", "Poll for a response", "fas fa-ballot"
    EDIT = 200, "Edited", "Edited the Bench", "fas fa-pencil"
    # also see https://discord.com/developers/docs/resources/message


@enum_(EnumType.MESSAGE_PLATFORM)
class MessagePlatform(BuiltinEnum):
    BENCH = 1
    WEBHOOK = 10
    EMAIL = 20
    SMS = 21
    # WHATSAPP, TELEGRAM, SLACK, ...?


@enum_(EnumType.MESSAGE_STATUS)
class MessageStatus(BuiltinEnum):
    DRAFT = 10
    SENDING = 20
    SENT = 30
    FAILED = 40
    RECEIVED = 50
    READ = 60


@timed_node_(NodeType.MESSAGE, passthrough_get="value", passthrough_set="value", has_subtypes=True)
class Message(IsTimed, IsBased, IsTitled, IsModal, PackageNode[MessageData]):
    """
    A Message about something (usually in a Thread or a Channel).
    """

    # meta
    parent: Union["Channel", "Thread", None] = p_node_parent(
        4, NodeType.CHANNEL, NodeType.THREAD, ckless=True
    )
    type: MessageType = p_regular(30, require=True, default=MessageType.REGULAR)
    platform: MessagePlatform = p_regular(33, require=True, default=MessagePlatform.BENCH)
    channel: Optional["Channel"] = p_node_ancestor(
        34,
        NodeType.CHANNEL,
        require=False,
        store=True,
        wire=True,
        is_bench_implicit=True,
    )
    thread: Optional["Thread"] = p_node_ancestor(
        35,
        NodeType.THREAD,
        require=True,
        store=True,
        wire=True,
        is_bench_implicit=True,
    )
    scope: Union["InlineNode", "Package"] = p_regular(
        36, require=False, references=(*INLINE_NODE_TYPES, NodeType.PACKAGE)
    )
    if TYPE_CHECKING:
        channel_id: Optional[UUID] = None
        channel_ptr: Optional[NodeReference] = None
        thread_id: Optional[UUID] = None
        thread_ptr: Optional[NodeReference] = None
        scope_id: Optional[UUID] = None
        scope_ptr: Optional[NodeReference] = None

    # status
    status: MessageStatus = p_internal(40, default=MessageStatus.SENT, default_sql=None)
    failed_at: Optional[datetime] = p_system(42, default=None)
    sent_at: Optional[datetime] = p_system(43, default=None)
    received_at: Optional[datetime] = p_system(44, default=None)
    read_at: Optional[datetime] = p_system(45, default=None)

    # routing
    reply_to: Optional["Message"] = p_regular(
        50, require=False, array=False, baseless=True, references=NodeType.MESSAGE
    )
    forwarded_from: Optional["Message"] = p_regular(
        51, require=False, array=False, baseless=True, references=NodeType.MESSAGE
    )
    if TYPE_CHECKING:
        reply_to_ptr: Optional[NodeReference] = None
        reply_to_id: Optional[UUID] = None
        forwarded_from_ptr: Optional[NodeReference] = None
        forwarded_from_id: Optional[UUID] = None
    run: Optional["Run"] = p_regular(
        52,
        require=False,
        array=False,
        baseless=True,
        references=NodeType.RUN,
        description="The Run this Message is about.",
    )
    interruption: Optional["Interruption"] = p_regular(
        53,
        require=False,
        array=False,
        baseless=True,
        references=NodeType.INTERRUPTION,
        description="The Interruption this Message is about.",
    )

    # content
    text: Optional["Text"] = p_regular(61, require=False, default=None, struct=StructType.TEXT)
    value_packed: Any = p_value_packed(62)
    value: Any = p_value_runtime(
        62, type=FieldType.MEMBER, typ=lambda self: cast("Message", self).value_type
    )
    clazz: Optional["Class"] = p_internal(
        63,
        require=False,
        array=False,
        references=NodeType.CLASS,
        description="The Message class.",
    )
    nodes: list["Node"] = p_regular(
        64,
        array=True,
        require=False,
        references="any",
        description="The Nodes this Message is about.",
    )
    if TYPE_CHECKING:
        clazz_ptr: Optional[NodeReference] = None
        clazz_id: Optional[UUID] = None
        nodes_ptr: Optional[NodeReference] = None
        nodes_id: Optional[UUID] = None

    def __content_str__(self) -> str:
        if self.title:
            return self.title.to_plain()
        elif self.text:
            return self.text.to_plain()
        else:
            return "<empty>"

    @property
    def container(self) -> "Node | None":
        if (thread := self.thread) is not None:
            return thread
        elif (channel := self.channel) is not None:
            return channel
        else:
            return self.parent

    @property
    def value_type(self) -> "TypeBase | None":
        class_ = self.clazz
        return class_.to_type() if class_ is not None else None

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

    @staticmethod
    def new(
        title: TextLine | None = None,
        text: Text | None = None,
        *,
        platform: MessagePlatform = MessagePlatform.BENCH,
        scope: Optional["InlineNode"] = None,
        reply_to: Optional["Message"] = None,
    ) -> "Message":
        message = Message(
            type=MessageType.REGULAR,
            platform=platform,
            title=text_line(title) if title is not None else None,
            text=text,
            reply_to=reply_to,
        )
        if scope is not None:
            message.scope = scope
        return message
