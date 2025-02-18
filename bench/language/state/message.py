from datetime import datetime
from typing import TYPE_CHECKING, Any, Optional, Union, cast
from uuid import UUID

import structlog

from bench.language.core import (
    TITLE_CONSTRAINT,
    BuiltinEnum,
    EnumType,
    FieldType,
    HasNodeBase,
    HasTimeIdentity,
    InlineSourceNode,
    Node,
    NodeType,
    StateNode,
    StructType,
    Text,
    TypeBase,
    enum_,
    p_internal,
    p_node_parent,
    p_regular,
    p_system,
    p_value_packed,
    p_value_runtime,
    timed_node_,
)
from bench.pb2 import AnyNodeData, MessageData, NodeReferenceData

if TYPE_CHECKING:
    from bench.language import (
        Bench,
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
    REGULAR = 1, "Standard Message"
    REPLY = 2, "Reply to another Message"
    FORWARDED = 3, "Forwarded Message"
    THREAD = 10, "Begin a Thread"
    RUN = 20, "Begin a Run"
    INTERRUPTION = 21, "Interrupt a Run"
    # for inspiration also see https://discord.com/developers/docs/resources/message


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
class Message(HasTimeIdentity, StateNode[MessageData], HasNodeBase):
    """
    A Message about something.
    """

    # meta
    parent: Union["Bench", None] = p_node_parent(4, NodeType.BENCH)
    type: MessageType = p_regular(30, require=True, default=MessageType.REGULAR)
    platform: MessagePlatform = p_regular(31, require=True, default=MessagePlatform.BENCH)
    channel: Optional["Channel"] = p_system(
        32,
        require=False,
        array=False,
        same_bench=True,
        references=NodeType.CHANNEL,
    )
    thread: Optional["Thread"] = p_regular(
        33,
        require=False,
        array=False,
        same_bench=True,
        references=NodeType.THREAD,
    )
    scope: Union["InlineSourceNode", "Package"] = p_regular(
        35, require=False, references=(NodeType.PAGE, NodeType.PACKAGE)
    )
    run: Optional["Run"] = p_regular(
        36,
        require=False,
        array=False,
        references=NodeType.RUN,
        description="The Run this Message is scoped to.",
    )
    clazz: Optional["Class"] = p_internal(
        37,
        require=False,
        array=False,
        references=NodeType.CLASS,
        description="The Message class.",
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
        clazz_id: Optional[UUID] = None
        clazz_ptr: Optional[NodeReference] = None

    # status
    status: MessageStatus = p_internal(40, default=MessageStatus.SENT)
    failed_at: Optional[datetime] = p_system(42, default=None)
    sent_at: Optional[datetime] = p_system(43, default=None)
    received_at: Optional[datetime] = p_system(44, default=None)
    read_at: Optional[datetime] = p_system(45, default=None)

    # content
    title: Optional[str] = p_regular(50, require=False, default=None, constraint=TITLE_CONSTRAINT)
    text: Optional["Text"] = p_regular(51, require=False, default=None, struct=StructType.TEXT)
    value_packed: Any = p_value_packed(52)
    value: Any = p_value_runtime(
        52, type=FieldType.MEMBER, typ=lambda self: cast("Message", self).value_type
    )
    nodes: list["Node"] = p_regular(53, array=True, require=False, references="any")
    interruption: Optional["Interruption"] = p_regular(
        55, require=False, array=False, references=NodeType.INTERRUPTION
    )

    # routing
    reply_to: Optional["Message"] = p_regular(
        60, require=False, array=False, references=NodeType.MESSAGE
    )
    forwarded_from: Optional["Message"] = p_regular(
        61, require=False, array=False, references=NodeType.MESSAGE
    )
    spawned_thread: Optional["Message"] = p_regular(
        62,
        require=False,
        array=False,
        references=NodeType.MESSAGE,
        same_bench=True,
        description="The Thread that was created from this Message.",
    )
    if TYPE_CHECKING:
        reply_to_id: Optional[UUID] = None
        reply_to_ptr: Optional[NodeReference] = None
        forwarded_from_id: Optional[UUID] = None
        forwarded_from_ptr: Optional[NodeReference] = None
        spawned_thread_id: Optional[UUID] = None
        spawned_thread_ptr: Optional[NodeReference] = None
    # roles, identities, users, teams, ...

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

    @property
    def value_type(self) -> "TypeBase | None":
        class_ = self.clazz
        return class_.to_type() if class_ is not None else None

    @staticmethod
    def new(
        title: str | None = None,
        text: Text | None = None,
        *,
        platform: MessagePlatform = MessagePlatform.BENCH,
        bench: Optional["Bench"] = None,
        channel: Optional["Channel"] = None,
        thread: Optional["Thread"] = None,
    ) -> "Message":
        message = Message(
            parent=bench,
            type=MessageType.REGULAR,
            platform=platform,
            title=title,
            text=text,
            channel=channel,
            thread=thread,
        )
        return message
