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
    Node,
    NodeType,
    StateNode,
    StructType,
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
from bench.language.runtime.interruption import Interruption
from bench.pb2 import AnyNodeData, MessageData, NodeReferenceData

if TYPE_CHECKING:
    from bench.language import Bench, Channel, Class, NodeReference, Text, Thread

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)


@enum_(EnumType.MESSAGE_TYPE)
class MessageType(BuiltinEnum):
    DEFAULT = 1
    BLOCK = 2
    # ...


@enum_(EnumType.MESSAGE_PLATFORM)
class MessagePlatform(BuiltinEnum):
    BENCH = 1  # within one Bench
    BENCH_FEDERATED = 2  # from/to another Bench
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
    If the parent is also a Message, then this is a Message thread (may be nested).
    """

    # meta
    parent: Union["Bench", "Thread", None] = p_node_parent(4, NodeType.BENCH, NodeType.THREAD)
    type: MessageType = p_regular(30, require=True, default=MessageType.DEFAULT)
    platform: MessagePlatform = p_regular(31, require=True, default=MessagePlatform.BENCH)
    channel: "Channel | None" = p_system(
        32,
        require=False,
        store=True,
        wire=True,
        same_bench=True,
        references=NodeType.MESSAGE,
    )
    class_: "Class | None" = p_internal(
        35,
        require=False,
        array=False,
        references=NodeType.CLASS,
        description="The Message type.",
    )
    if TYPE_CHECKING:
        scope_id: Optional[UUID] = None
        scope_ptr: Optional[NodeReference] = None

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

    # routing :MessageRouting
    reply_to: Optional["Message"] = p_regular(
        60, require=False, array=False, references=NodeType.MESSAGE
    )

    # to: roles, identities, users, teams, ...

    # flags
    # is_pinned, is_highlighted, ...

    def __content_str__(self) -> str:
        if self.title:
            return self.title
        elif self.text:
            return self.text.to_markdown()
        else:
            return "<empty>"

    @property
    def base(self):
        return self.class_

    @staticmethod
    def get_base_from_data(data: AnyNodeData) -> Optional[NodeReferenceData]:
        return cast("MessageData", data).block_ptr

    @property
    def value_type(self) -> "TypeBase | None":
        class_ = self.class_
        return class_.to_type() if class_ is not None else None
