from typing import TYPE_CHECKING, Optional, Union, overload

from bench.language.core import (
    InlineNode,
    IsClaimable,
    IsModal,
    IsOwnable,
    IsTemplatable,
    IsTitled,
    LocalNodeList,
    Node,
    NodeType,
    TextIn,
    TextLineIn,
    node_,
    p_node_children,
    p_node_parent,
    p_regular,
    text_line,
    to_text,
)
from bench.language.source.block import Block
from bench.pb2 import BlockData

if TYPE_CHECKING:
    from bench.language import Block, Channel, Package, Thread

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.PAGE)
class Page(
    IsTemplatable,
    IsModal,
    IsTitled,
    IsOwnable,
    IsClaimable,
    InlineNode[BlockData],
):
    """A Page of Blocks."""

    # meta
    parent: Union["Package", "Page", None] = p_node_parent(4, NodeType.PACKAGE, NodeType.PAGE)
    main_thread: Optional["Thread"] = p_regular(
        38, default=None, require=False, array=False, references=NodeType.THREAD
    )

    # identity, roles, ...?

    pages: LocalNodeList["Page"] = p_node_children(NodeType.PAGE)
    blocks: LocalNodeList["Block"] = p_node_children(NodeType.BLOCK)
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

    def add_text(
        self, text: TextIn, after: Optional["Block"] = None, before: Optional["Block"] = None
    ) -> list[Block]:
        """Add text to the Page."""
        text = to_text(text)
        blocks = Block.from_text(text)
        self.blocks.extend(*blocks, after=after, before=before)
        return blocks

    def remove_text(self, after: Optional["Block"] = None, before: Optional["Block"] = None):
        """Remove text from the Page (between two blocks, exclusive)."""
        self.blocks.remove_between(after, before)

    @staticmethod
    def new(title: "TextLineIn", *nodes: "Block | InlineNode", **kwargs) -> "Page":
        page = Page(title=text_line(title), **kwargs)
        for node in nodes:
            page.append(node)
        return page
