from typing import TYPE_CHECKING, Optional, Union, overload

from bench.language.core import (
    NAME_CONSTRAINT,
    InlineSourceNode,
    LocalNodeList,
    Node,
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
    from bench.language import Block, Channel, Field, Icon, Package, Text

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

    pages: LocalNodeList["Page"] = p_node_children(NodeType.PAGE)
    blocks: LocalNodeList["Block"] = p_node_children(NodeType.BLOCK)
    fields: LocalNodeList["Field"] = p_node_children(NodeType.FIELD)
    channels: LocalNodeList["Channel"] = p_node_children(NodeType.CHANNEL)

    def __content_str__(self):
        return ""

    @overload
    def append(self, child: InlineSourceNode, move: bool = False) -> "Block": ...
    @overload
    def append[T: Node](self, child: T, move: bool = False) -> T: ...
    def append[T: Node](self, child: T, move: bool = False) -> "T | Block":
        if not move and isinstance(child, InlineSourceNode):
            # wrap InlineSourceNodes into Blocks
            child_block = child.to_block()
            self.blocks.append(child_block)
            super().append(child, move)
            return child_block
        else:
            return super().append(child, move)

    @staticmethod
    def new(name: str, **kwargs) -> "Page":
        return Page(name=name, **kwargs)
