from typing import TYPE_CHECKING, Optional, Union

from bench.language.core import (
    NAME_CONSTRAINT,
    LocalNodeList,
    NodeType,
    SourceNode,
    StructType,
    node_,
    p_internal,
    p_node_children,
    p_node_parent,
    p_regular,
)
from bench.pb2 import BlockData
from bench.utils.fractional import INTEGER_ZERO

if TYPE_CHECKING:
    from bench.language import (
        Block,
        Channel,
        Choice,
        Class,
        Database,
        Field,
        Flow,
        Icon,
        Package,
        Text,
    )

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.PAGE, passthrough_get=("fields",))
class Page(SourceNode[BlockData]):
    """A Page of Blocks."""

    parent: Union["Page", "Package", None] = p_node_parent(4, NodeType.PAGE, NodeType.PACKAGE)

    # content
    name: str = p_regular(32, constraint=NAME_CONSTRAINT)
    order_key: str = p_internal(33, default=INTEGER_ZERO)
    icon: Optional["Icon"] = p_regular(
        34, default=None, require=False, array=False, struct=StructType.ICON
    )
    text: Optional["Text"] = p_regular(
        35, default=None, require=False, array=False, struct=StructType.TEXT
    )

    blocks: LocalNodeList["Block"] = p_node_children(NodeType.BLOCK)
    fields: LocalNodeList["Field"] = p_node_children(NodeType.FIELD)
    choices: LocalNodeList["Choice"] = p_node_children(NodeType.CHOICE)
    flows: LocalNodeList["Flow"] = p_node_children(NodeType.FLOW)
    databases: LocalNodeList["Database"] = p_node_children(NodeType.DATABASE)
    classes: LocalNodeList["Class"] = p_node_children(NodeType.CLASS)
    channels: LocalNodeList["Channel"] = p_node_children(NodeType.CHANNEL)

    def __content_str__(self):
        return ""
