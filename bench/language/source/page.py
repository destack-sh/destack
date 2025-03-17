from typing import TYPE_CHECKING, Union, overload

from bench.language.core import (
    InlineNode,
    IsModal,
    IsTemplatable,
    IsTitled,
    LocalNodeList,
    Node,
    NodeType,
    TextLineIn,
    node_,
    p_node_children,
    p_node_parent,
    text_line,
)
from bench.pb2 import BlockData

if TYPE_CHECKING:
    from bench.language import Block, Channel, Field, Package, Run

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.PAGE)
class Page(IsTemplatable, IsModal, IsTitled, InlineNode[BlockData]):
    """A Page of Blocks."""

    # meta
    parent: Union["Package", "Page", "Run", None] = p_node_parent(
        4, NodeType.PACKAGE, NodeType.PAGE, NodeType.RUN
    )

    # identity, roles, ...?

    pages: LocalNodeList["Page"] = p_node_children(NodeType.PAGE)
    blocks: LocalNodeList["Block"] = p_node_children(NodeType.BLOCK)
    fields: LocalNodeList["Field"] = p_node_children(NodeType.FIELD)
    channels: LocalNodeList["Channel"] = p_node_children(NodeType.CHANNEL)

    def __content_str__(self):
        return ""

    @overload
    def append(self, child: InlineNode, move: bool = False) -> "Block": ...
    @overload
    def append[T: Node](self, child: T, move: bool = False) -> T: ...
    def append[T: Node](self, child: T, move: bool = False) -> "T | Block":
        if not move and isinstance(child, InlineNode):
            # wrap InlineNodes into Blocks
            child_block = child.to_block()
            self.blocks.append(child_block)
            super().append(child, move)
            return child_block
        else:
            return super().append(child, move)

    @staticmethod
    def new(title: "TextLineIn", *nodes: "Block | InlineNode", **kwargs) -> "Page":
        page = Page(title=text_line(title), **kwargs)
        for node in nodes:
            page.append(node)
        return page
