from typing import TYPE_CHECKING, Any, Optional, Union, cast

import structlog

from bench.language.const import NODE_TYPES, NodeType, StructType
from bench.language.node import (
    BenchNode,
    HasNodeBase,
    HasTimeIdentity,
    RemoteNode,
    timed_node_,
)
from bench.language.property import p_node_parent, p_regular, p_value_packed, p_value_runtime
from bench.language.validation import TITLE_CONSTRAINT
from bench.language.value import HasValues
from bench.proto.wire import AnyNodeData, MessageData, NodeReferenceData

if TYPE_CHECKING:
    from bench.language import Block, NodeReference, Package, Path, Step, Text, ValueObject, View

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


@timed_node_(NodeType.MESSAGE, passthrough="value")
class Message(
    RemoteNode[MessageData],
    HasTimeIdentity,
    HasNodeBase,
    HasValues,
):
    """
    A Message by a User or program (author = created_by).
    If the parent is also a Message, then this is part of a thread. Threads may be nested.
    """

    parent: MessageParent | None = p_node_parent(4, *MESSAGE_PARENT_TYPES)
    origin: BenchNode = p_regular(32, require=True, references=NODE_TYPES.tuple)
    path: Optional["Path"] = p_regular(33, require=False, array=False, struct=StructType.PATH)
    reply_to: Optional["Message"] = p_regular(
        34, require=False, default=None, references=NodeType.MESSAGE, same_bench=True
    )
    if TYPE_CHECKING:
        origin_ptr: Optional[NodeReference] = None
        reply_to_ptr: Optional[NodeReference] = None

    # content
    title: Optional[str] = p_regular(40, require=False, default=None, constraint=TITLE_CONSTRAINT)
    text: Optional["Text"] = p_regular(41, require=False, default=None, struct=StructType.TEXT)
    value_packed: Any = p_value_packed(42)
    value: "ValueObject | None" = p_value_runtime(42, typ=None)  # freely typed

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
        return cast("MessageData", data).origin_ptr
