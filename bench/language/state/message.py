from datetime import datetime
from typing import TYPE_CHECKING, Any, Optional, cast

import structlog

from bench.language.core import (
    NAME_CONSTRAINT,
    TITLE_CONSTRAINT,
    BenchNode,
    BlockType,
    EnumType,
    FieldType,
    HasNodeBase,
    HasTimeIdentity,
    NodeType,
    StateNode,
    StructType,
    enum_,
    p_internal,
    p_regular,
    p_value_packed,
    p_value_runtime,
    timed_node_,
)
from bench.language.source import TypeBase, constraint
from bench.pb2 import AnyNodeData, MessageData, NodeReferenceData
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    from bench.language import Block, NodeReference, Text

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)


@enum_(EnumType.MESSAGE_TYPE)
class MessageType(IdEnum):
    LOCAL = 1  # within Bench
    FEDERATED = 2  # from/to another Bench
    WEBHOOK = 10
    EMAIL = 20
    SMS = 21
    # WHATSAPP, TELEGRAM, SLACK, ...?


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


@timed_node_(NodeType.MESSAGE, passthrough_get="value", passthrough_set="value", has_subtypes=True)
class Message(HasTimeIdentity, StateNode[MessageData], HasNodeBase):
    """
    A Message by a User or program (author = created_by).
    If the parent is also a Message, then this Message is part of a Message thread.
    """

    type: MessageType = p_regular(30, require=True, default=MessageType.LOCAL)
    status: MessageStatus = p_internal(31, default=MessageStatus.SENT)
    name: str | None = p_regular(32, require=False, constraint=NAME_CONSTRAINT)
    origin: BenchNode | None = p_regular(35, require=False, references="any")
    block: "Block | None" = p_internal(
        36,
        require=False,
        array=False,
        references=NodeType.BLOCK,
        constraint=constraint(node_subtypes=[BlockType.MESSAGE]),
    )
    if TYPE_CHECKING:
        origin_ptr: Optional[NodeReference] = None

    # content
    title: Optional[str] = p_regular(40, require=False, default=None, constraint=TITLE_CONSTRAINT)
    text: Optional["Text"] = p_regular(41, require=False, default=None, struct=StructType.TEXT)
    value_packed: Any = p_value_packed(42)
    value: Any = p_value_runtime(
        42, type=FieldType.MEMBER, typ=lambda self: cast("Message", self).value_type
    )
    expires_at: Optional[datetime] = p_internal(43, default=None)
    read_at: Optional[datetime] = p_internal(44, default=None)

    # routing
    reply_to: Optional["Message"] = p_regular(
        50, require=False, array=False, references=NodeType.MESSAGE
    )
    # to: roles, identities, users, teams, ...

    # flags
    is_pinned: bool = p_regular(70, default=False)

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
