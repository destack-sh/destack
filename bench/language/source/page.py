from typing import TYPE_CHECKING, Union, overload

from bench.language.core import (
    InlineSourceNode,
    LocalNodeList,
    Node,
    NodeType,
    node_,
    p_internal,
    p_node_children,
    p_node_parent,
)
from bench.pb2 import BlockData
from bench.utils.fractional import INTEGER_ZERO

if TYPE_CHECKING:
    from bench.language import Block, Channel, Field, Package

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.PAGE, passthrough_get=("fields",))
class Page(InlineSourceNode[BlockData]):
    """A Page of Blocks."""

    parent: Union["Page", "Package", None] = p_node_parent(4, NodeType.PAGE, NodeType.PACKAGE)

    order_key: str = p_internal(33, default=INTEGER_ZERO)

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
    def new(name: str, *nodes: "Block | InlineSourceNode", **kwargs) -> "Page":
        page = Page(name=name, **kwargs)
        for node in nodes:
            page.append(node)
        return page
