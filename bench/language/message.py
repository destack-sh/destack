from typing import TYPE_CHECKING, Any, Optional, Union, cast

from bench.language.const import NodeType, StructType
from bench.language.node import LINK_TARGET_NODE_TYPES, BasedNode, Node, node
from bench.language.property import p_node_parent, p_regular, p_value_packed, p_value_runtime
from bench.language.value import HasValues
from bench.proto.wire import AnyNodeData, MessageData, NodeReferenceData
from bench.utils.uuidt import UUIDT

if TYPE_CHECKING:
    from bench.language import Block, Package, Path, Text

# pyright: reportIncompatibleVariableOverride=false

MessageParent = Union["Package", "Block", "Message"]
MESSAGE_PARENT_TYPES: tuple[NodeType, ...] = (NodeType.PACKAGE, NodeType.BLOCK, NodeType.MESSAGE)


@node(NodeType.MESSAGE, id_factory=UUIDT, local=True)
class Message(BasedNode[MessageData], HasValues):  # noqa: F821
    """
    A Message by a User or program (author = created_by).
    If the parent is also a Message, then this is part of a thread. Threads may be nested.
    Messages are ordered by created_at.
    """

    parent: MessageParent = p_node_parent(4, *MESSAGE_PARENT_TYPES)
    origin: Node = p_regular(32, require=True, references=LINK_TARGET_NODE_TYPES)
    path: Optional["Path"] = p_regular(33, require=False, array=False, struct=StructType.PATH)
    reply_to: Optional["Message"] = p_regular(
        34, require=False, default=None, references=NodeType.MESSAGE, is_bench_implicit=True
    )
    if TYPE_CHECKING:
        origin_ptr: Optional[NodeReferenceData] = None
        reply_to_ptr: Optional[NodeReferenceData] = None

    # content
    title: Optional[str] = p_regular(40, require=False, default=None)
    text: Optional["Text"] = p_regular(41, require=False, default=None, struct=StructType.TEXT)
    value_packed: Any = p_value_packed(42)
    secret_value_packed: Any = p_value_packed(43)
    value: Any = p_value_runtime(42, 43)

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
    def base(self) -> "Node":
        return self.origin

    @staticmethod
    def get_base_from_data(data: AnyNodeData) -> Optional[NodeReferenceData]:
        return cast("MessageData", data).origin_ptr
