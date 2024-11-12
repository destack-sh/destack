from datetime import datetime
from typing import TYPE_CHECKING, Any, Optional, Union, cast

import structlog

from bench.language.const import (
    NODE_TYPES,
    BlockType,
    EnumType,
    NodeType,
    ObjectKind,
    StructType,
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
    NATIVE = 1
    EMAIL = 10
    SMS = 11
    # WHATSAPP, TELEGRAM, ...?


@timed_node_(NodeType.MESSAGE, passthrough_get="value", passthrough_set="value")
class Message(HasTimeIdentity, StateNode[MessageData], HasNodeBase):
    """
    A Message by a User or program (author = created_by).
    If the parent is also a Message, then this is part of a thread. Threads may be nested.
    """

    parent: MessageParent | None = p_node_parent(4, *MESSAGE_PARENT_TYPES)
    type: MessageType = p_regular(30, require=True, default=MessageType.NATIVE)
    origin: BenchNode = p_regular(32, require=True, references=NODE_TYPES.tuple)
    block: "Block" = p_internal(
        33,
        require=True,
        array=False,
        references=NodeType.BLOCK,
        constraint=constraint(block_types=[BlockType.MESSAGE]),
    )
    reply_to: Optional["Message"] = p_regular(
        34, require=False, array=False, references=NodeType.MESSAGE
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

    # context
    # ...HasSessionContext[70-79]

    def __content_str__(self) -> str:
        if self.title:
            return self.title
        elif self.text:
            return self.text.to_markdown()
        else:
            return "<empty>"

    @property
    def base(self):
        return self.origin

    @staticmethod
    def get_base_from_data(data: AnyNodeData) -> Optional[NodeReferenceData]:
        return cast("MessageData", data).block_ptr

    @property
    def value_type(self) -> "TypeBase | None":
        return self.block.to_type(as_object=True)
