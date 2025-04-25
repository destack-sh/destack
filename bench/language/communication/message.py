from datetime import datetime
from typing import TYPE_CHECKING, Any, Optional, Union, cast
from uuid import UUID

import structlog

from bench.language.core import (
    PAGE_NODE_TYPES,
    RUNNABLE_NODE_TYPES,
    UNSET,
    BuiltinEnum,
    EnumType,
    FieldType,
    IsBased,
    IsModal,
    IsOwnable,
    IsTimed,
    IsTitled,
    IsType,
    Node,
    NodeType,
    PackageNode,
    PageNode,
    Runnable,
    StructType,
    Subject,
    Text,
    TextIn,
    TextLine,
    enum_,
    p_internal,
    p_node_ancestor,
    p_node_parent,
    p_regular,
    p_value_packed,
    p_value_runtime,
    text_line,
    timed_node_,
    to_text,
)
from bench.pb2 import AnyNodeData, MessageData, NodeReferenceData

if TYPE_CHECKING:
    from bench.language import (
        Channel,
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
    DEFAULT = 1, "Default", "Regular text (and nodes)", "fas fa-envelope"
    # FORWARDED = 3, "Forwarded", "Forwarded Message", "fas fa-forward"
    JOIN = 10, "Join", "Join a chat", "fas fa-arrow-right-to-bracket"
    LEAVE = 11, "Leave", "Leave a chat", "fas fa-arrow-left-from-line"
    THREAD = 20, "Thread", "Thread inside a chat", "fas fa-thread"
    RUN = 100, "Run", None, "fas fa-play"
    INTERRUPTION = 110, "Interruption", None, "fas fa-times"
    PLAN = 200, "Plan", None, "fas fa-list-check"
    TASK = 210, "Task", None, "fas fa-square-check"
    # EDIT, STREAM, ...
    # also see https://discord.com/developers/docs/resources/message


@enum_(EnumType.MESSAGE_STATUS)
class MessageStatus(BuiltinEnum):
    DRAFT = 10
    SENDING = 20
    SENT = 30
    FAILED = 40
    RECEIVED = 50
    READ = 60


@timed_node_(NodeType.MESSAGE)
class Message(
    IsTimed,
    IsBased,
    IsOwnable,
    IsTitled,
    IsModal,
    PackageNode[MessageData],
):
    """
    A Message about something (usually in a Thread or a Channel).
    """

    # nocheckin: track & show model/budget/... per Message?
    # (maybe in line with Agent settings.. common IsComputable/IsCostable/... trait?)

    # meta
    parent: Union["Channel", "Thread", None] = p_node_parent(
        4, NodeType.CHANNEL, NodeType.THREAD, ckless=True
    )
    type: MessageType = p_regular(30, require=True, default=MessageType.DEFAULT)
    # platform? source?
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
    scope: Union["PageNode", "Package"] = p_regular(
        36, require=False, references=(*PAGE_NODE_TYPES, NodeType.PACKAGE)
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
    failed_at: Optional[datetime] = p_internal(42, default=None)
    sent_at: Optional[datetime] = p_internal(43, default=None)
    received_at: Optional[datetime] = p_internal(44, default=None)
    read_at: Optional[datetime] = p_internal(45, default=None)
    edited_at: Optional[datetime] = p_internal(46, default=None)

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

    # content
    text: Optional["Text"] = p_regular(61, require=False, default=None, struct=StructType.TEXT)
    value_packed: Any = p_value_packed(62)
    value: Any = p_value_runtime(
        62, type=FieldType.MEMBER, typ=lambda self: cast("Message", self).value_type
    )
    nodes: list["Node"] = p_regular(
        63,
        array=True,
        require=False,
        references="any",
        description="The Nodes this Message is about.",
    )
    run: Optional["Run"] = p_regular(
        64,
        require=False,
        array=False,
        baseless=True,
        references=NodeType.RUN,
        description="The Run this Message is about.",
    )
    runnable: Optional[Runnable] = p_regular(
        65,
        require=False,
        array=False,
        baseless=True,
        references=RUNNABLE_NODE_TYPES.tuple,
        description="The Runnable this Message is about.",
    )
    interruption: Optional["Interruption"] = p_regular(
        66,
        require=False,
        array=False,
        baseless=True,
        references=NodeType.INTERRUPTION,
        description="The Interruption this Message is about.",
    )
    if TYPE_CHECKING:
        nodes_ptr: Optional[NodeReference] = None
        nodes_id: Optional[UUID] = None
        run_ptr: Optional[NodeReference] = None
        run_id: Optional[UUID] = None
        runnable_ptr: Optional[NodeReference] = None
        runnable_id: Optional[UUID] = None
        interruption_ptr: Optional[NodeReference] = None
        interruption_id: Optional[UUID] = None

    def __content_str__(self) -> str:
        if self.title:
            return self.title.to_plain()
        elif self.text:
            return self.text.to_plain()
        else:
            return "<empty>"

    @property
    def value_type(self) -> "IsType | None":
        if (runnable := self.runnable) is not None:
            return runnable.to_type_maybe(of="value")
        else:
            return None

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
        run: Optional["Run"] = None,
        runnable: Optional[Runnable] = None,
        interruption: Optional["Interruption"] = None,
        value: Any = None,
    ) -> "Message":
        message = Message(
            type=type,
            title=text_line(title) if title is not None else None,
            text=to_text(text) if text is not None else None,
            reply_to=reply_to,
            owned_by=owned_by,
            run=run,
            runnable=runnable,
            interruption=interruption,
            value=value,
        )
        if nodes is not None:
            message.nodes = nodes
        if scope is not None:
            message.scope = scope
        return message
