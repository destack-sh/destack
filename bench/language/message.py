from typing import TYPE_CHECKING, Any, Optional, Union

from bench.language.const import NodeType, StructType
from bench.language.node import LINK_TARGET_NODE_TYPES, Node, node
from bench.language.property import p_node_parent, p_regular, p_value_packed, p_value_runtime
from bench.language.value import HasValues

if TYPE_CHECKING:
    from bench.language import Block, Package, Path, Text

# pyright: reportIncompatibleVariableOverride=false

MessageParent = Union["Package", "Block", "Message"]
MESSAGE_PARENT_TYPES: tuple[NodeType, ...] = (NodeType.PACKAGE, NodeType.BLOCK)


@node(NodeType.MESSAGE)
class Message(Node, HasValues):
    """
    A Message by a User or Block (whoever created it).
    If the parent is also a Message, then this is part of a thread. Threads may be nested.
    """

    parent: MessageParent = p_node_parent(4, *MESSAGE_PARENT_TYPES)
    origin: Optional["Node"] = p_regular(32, require=False, references=LINK_TARGET_NODE_TYPES)
    path: Optional["Path"] = p_regular(33, require=False, array=False, struct=StructType.PATH)
    reply_to: Optional["Message"] = p_regular(
        34, require=False, default=None, references=NodeType.MESSAGE
    )

    # content
    title: Optional[str] = p_regular(40, require=False, default=None)
    text: Optional["Text"] = p_regular(41, require=False, default=None, struct=StructType.TEXT)
    value_packed: Any = p_value_packed(42)
    secret_value_packed: Any = p_value_packed(43)
    value: Any = p_value_runtime(42, 43)

    # flags
    is_pinned: bool = p_regular(50, default=False)
