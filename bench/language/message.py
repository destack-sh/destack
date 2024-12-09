from datetime import datetime
from typing import TYPE_CHECKING, Any, Optional, Union, cast

import structlog

from bench.language.const import (
    BlockType,
    EnumType,
    NodeType,
    ObjectKind,
    StructType,
    active_session,
    enum_,
)
from bench.language.field import TypeBase, constraint
from bench.language.node import (
    BenchNode,
    HasNodeBase,
    HasTimeIdentity,
    StateNode,
    timed_node_,
)
from bench.language.property import (
    p_internal,
    p_node_parent,
    p_regular,
    p_value_packed,
    p_value_runtime,
)
from bench.language.validation import TITLE_CONSTRAINT
from bench.proto.wire import AnyNodeData, MessageData, NodeReferenceData
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    from bench.language import Block, CustomObject, NodeReference, Package, Step, Text, View

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)

MessageParent = Union["Package", "Block", "View", "Step", "Message"]
MESSAGE_PARENT_TYPES: tuple[NodeType, ...] = (
    NodeType.PACKAGE,
    NodeType.BLOCK,
    NodeType.VIEW,
    NodeType.STEP,
    NodeType.MESSAGE,
)


@enum_(EnumType.MESSAGE_TYPE)
class MessageType(IdEnum):
    INTERNAL = 1  # within Bench
    FEDERATED = 2  # from/to another Bench
    EMAIL = 10
    SMS = 11
    # WHATSAPP, TELEGRAM, ...?


@enum_(EnumType.MESSAGE_STATUS)
class MessageStatus(IdEnum):
    DRAFT = 1
    PREPARED = 2
    SENDING = 3
    SENT = 4
    FAILED = 5
    RECEIVED = 6
    READ = 7
    EXPIRED = 8


@timed_node_(NodeType.MESSAGE, passthrough_get="value", passthrough_set="value")
class Message(HasTimeIdentity, StateNode[MessageData], HasNodeBase):
    """
    A Message by a User or program (author = created_by).
    If the parent is also a Message, then this is part of a thread (which may also be nested).
    """

    parent: MessageParent | None = p_node_parent(4, *MESSAGE_PARENT_TYPES)
    type: MessageType = p_regular(30, require=True, default=MessageType.INTERNAL)
    status: MessageStatus = p_internal(31, default=MessageStatus.SENT)
    origin: BenchNode | None = p_regular(35, require=False, references="any")
    block: "Block | None" = p_internal(
        36,
        require=False,
        array=False,
        references=NodeType.BLOCK,
        constraint=constraint(node_subtypes=[BlockType.MESSAGE]),
    )
    reply_to: Optional["Message"] = p_regular(
        37, require=False, array=False, references=NodeType.MESSAGE
    )
    if TYPE_CHECKING:
        origin_ptr: Optional[NodeReference] = None
        reply_to_ptr: Optional[NodeReference] = None

    # content
    title: Optional[str] = p_regular(40, require=False, default=None, constraint=TITLE_CONSTRAINT)
    text: Optional["Text"] = p_regular(41, require=False, default=None, struct=StructType.TEXT)
    value_packed: Any = p_value_packed(42)
    value: "CustomObject | None" = p_value_runtime(
        42, kind=ObjectKind.MEMBER, typ=lambda self: cast("Message", self).value_type
    )
    expires_at: Optional[datetime] = p_internal(44, default=None)
    read_at: Optional[datetime] = p_internal(45, default=None)

    # flags
    is_pinned: bool = p_regular(50, default=False)

    def __content_str__(self) -> str:
        if self.title:
            return self.title
        elif self.text:
            return self.text.to_markdown()
        else:
            return "<empty>"

    @property
    def base(self):
        return self.block

    @staticmethod
    def get_base_from_data(data: AnyNodeData) -> Optional[NodeReferenceData]:
        return cast("MessageData", data).block_ptr

    @property
    def value_type(self) -> "TypeBase | None":
        block = self.block
        return block.to_type_maybe(of="value") if block is not None else None

    @staticmethod
    def new(
        block: "Block | None" = None,
        title: str | None = None,
        text: "Text | None" = None,
        value: "CustomObject | None" = None,
        *,
        parent: MessageParent | None = None,
        origin: BenchNode | None = None,
        type: MessageType = MessageType.INTERNAL,
        status: MessageStatus = MessageStatus.SENT,
        reply_to: "Message | None" = None,
    ) -> "Message":
        return Message(
            parent=parent or active_session().package,
            block=block,
            title=title,
            text=text,
            value=value,
            origin=origin,
            reply_to=reply_to,
            type=type,
            status=status,
        )
