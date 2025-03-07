from datetime import datetime
from typing import TYPE_CHECKING, Any, Optional, Union, cast
from uuid import UUID

import structlog

from bench.language.core import (
    INLINE_SOURCE_NODE_TYPES,
    TITLE_CONSTRAINT,
    BenchNode,
    BuiltinEnum,
    EnumType,
    FieldType,
    InlineSourceNode,
    IsBased,
    IsTimed,
    Node,
    NodeType,
    StructType,
    Text,
    TypeBase,
    enum_,
    p_internal,
    p_node_ancestor,
    p_node_parent,
    p_regular,
    p_system,
    p_value_packed,
    p_value_runtime,
    timed_node_,
)
from bench.pb2 import AnyNodeData, MessageData
from bench.pb2.lang_pb2 import NodeReferenceData

if TYPE_CHECKING:
    from bench.language import (
        Channel,
        Class,
        Interruption,
        NodeReference,
        Package,
        Run,
        Selection,
        Thread,
    )

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)


@enum_(EnumType.MESSAGE_TYPE)
class MessageType(BuiltinEnum):
    REGULAR = 1, "Regular", "Standard Message", "fas fa-envelope"
    REPLY = 2, "Reply", "Reply to another Message", "fas fa-reply"
    FORWARDED = 3, "Forwarded", "Forwarded Message", "fas fa-forward"
    THREAD = 100, "Thread", "Begin a Thread", "fas fa-thread"
    RUN = 110, "Run", "Begin a Run", "fas fa-play"
    INTERRUPTION = 120, "Interruption", "Interrupt a Run", "fas fa-hand"
    POLL = 130, "Poll", "Poll for a response", "fas fa-ballot"
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
class Message(IsTimed, IsBased, BenchNode[MessageData]):
    """
    A Message about something.
    """

    # meta
    parent: Union["Channel", "Thread", None] = p_node_parent(4, NodeType.CHANNEL, NodeType.THREAD)
    type: MessageType = p_regular(30, require=True, default=MessageType.REGULAR)
    platform: MessagePlatform = p_regular(31, require=True, default=MessagePlatform.BENCH)
    channel: "Channel" = p_node_ancestor(
        32,
        NodeType.CHANNEL,
        require=True,
        store=True,
        wire=True,
        is_bench_implicit=True,
    )
    thread: Optional["Thread"] = p_node_ancestor(
        33,
        NodeType.THREAD,
        require=False,
        store=True,
        wire=True,
        is_bench_implicit=True,
    )
    scope: Union["InlineSourceNode", "Package"] = p_regular(
        35, require=False, references=(*INLINE_SOURCE_NODE_TYPES, NodeType.PACKAGE)
    )
    run_root: Optional["Run"] = p_regular(
        36,
        require=False,
        array=False,
        references=NodeType.RUN,
        same_bench=True,
        description="The root Run this Message/Thread is scoped to.",
    )
    run: Optional["Run"] = p_regular(
        37,
        require=False,
        array=False,
        references=NodeType.RUN,
        description="The Run this Message/Thread is scoped to.",
    )
    if TYPE_CHECKING:
        channel_id: Optional[UUID] = None
        channel_ptr: Optional[NodeReference] = None
        thread_id: Optional[UUID] = None
        thread_ptr: Optional[NodeReference] = None
        scope_id: Optional[UUID] = None
        scope_ptr: Optional[NodeReference] = None
        run_id: Optional[UUID] = None
        run_ptr: Optional[NodeReference] = None

    # status
    status: MessageStatus = p_internal(40, default=MessageStatus.SENT)
    failed_at: Optional[datetime] = p_system(42, default=None)
    sent_at: Optional[datetime] = p_system(43, default=None)
    received_at: Optional[datetime] = p_system(44, default=None)
    read_at: Optional[datetime] = p_system(45, default=None)

    # routing
    reply_to: Optional["Message"] = p_regular(
        50, require=False, array=False, references=NodeType.MESSAGE
    )
    forwarded_from: Optional["Message"] = p_regular(
        51, require=False, array=False, references=NodeType.MESSAGE
    )
    if TYPE_CHECKING:
        reply_to_ptr: Optional[NodeReference] = None
        reply_to_id: Optional[UUID] = None
        forwarded_from_ptr: Optional[NodeReference] = None
        forwarded_from_id: Optional[UUID] = None
    # roles, identities, users, teams, ...?

    # content
    title: Optional[str] = p_regular(60, require=False, default=None, constraint=TITLE_CONSTRAINT)
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
    nodes: list["Node"] = p_regular(64, array=True, require=False, references="any")
    selection: Optional["Selection"] = p_regular(
        65, require=False, array=False, struct=StructType.SELECTION
    )
    created_interruption: Optional["Interruption"] = p_regular(
        70,
        require=False,
        array=False,
        references=NodeType.INTERRUPTION,
        description="The Interruption this Message was for.",
    )
    created_run: Optional["Run"] = p_regular(
        71,
        require=False,
        array=False,
        references=NodeType.RUN,
        description="The Run this Message was created for.",
    )
    created_thread: Optional["Thread"] = p_regular(
        72,
        require=False,
        array=False,
        references=NodeType.THREAD,
        same_bench=True,
        description="The Thread that was created from this Message.",
    )
    if TYPE_CHECKING:
        clazz_ptr: Optional[NodeReference] = None
        clazz_id: Optional[UUID] = None
        interruption_ptr: Optional[NodeReference] = None
        interruption_id: Optional[UUID] = None
        run_ptr: Optional[NodeReference] = None
        run_id: Optional[UUID] = None
        spawned_thread_ptr: Optional[NodeReference] = None
        spawned_thread_id: Optional[UUID] = None

    def __content_str__(self) -> str:
        if self.title:
            return self.title
        elif self.text:
            return self.text.to_markdown()
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
        return self.clazz

    @staticmethod
    def get_base_from_data(data: AnyNodeData) -> Optional[NodeReferenceData]:
        return cast("MessageData", data).clazz_ptr

    @staticmethod
    def get_base_from_partial(data: dict[str, Any]) -> Optional["Class"]:
        if "clazz" in data:
            return data["clazz"]
        else:
            return None

    @staticmethod
    def new(
        title: str | None = None,
        text: Text | None = None,
        *,
        platform: MessagePlatform = MessagePlatform.BENCH,
        scope: Optional["InlineSourceNode"] = None,
        reply_to: Optional["Message"] = None,
    ) -> "Message":
        message = Message(
            type=MessageType.REGULAR,
            platform=platform,
            title=title,
            text=text,
            reply_to=reply_to,
        )
        if scope is not None:
            message.scope = scope
        return message
