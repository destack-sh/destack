from typing import TYPE_CHECKING, Optional, Sequence, Union, cast, overload

from bench.language.core import (
    IsClaimable,
    IsModal,
    IsOwnable,
    IsTemplatable,
    IsTitled,
    LocalNodeList,
    Node,
    NodeType,
    PageNode,
    TextIn,
    TextLineIn,
    node_,
    p_node_children,
    p_node_parent,
    p_regular,
    text_line,
    to_text,
)
from bench.language.package.block import Block
from bench.pb2 import BlockData

if TYPE_CHECKING:
    from bench.language import Block, Package, Thread

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.PAGE)
class Page(
    IsTemplatable,
    IsModal,
    IsTitled,
    IsOwnable,
    IsClaimable,
    PageNode[BlockData],
):
    """A Page of Blocks laying out rich Text, data, logic, resources -- anything software needs."""

    # NOTE: :Architecture: maybe some PageNodes should have their own Page? or is that confusing?

    # meta
    parent: Union["Package", "Page", None] = p_node_parent(4, NodeType.PACKAGE, NodeType.PAGE)
    thread: Optional["Thread"] = p_regular(
        38, default=None, require=False, array=False, references=NodeType.THREAD
    )
    # app? Page/Record/... tying?

    pages: LocalNodeList["Page"] = p_node_children(NodeType.PAGE)
    blocks: LocalNodeList["Block"] = p_node_children(NodeType.BLOCK)

    def __content_str__(self):
        return ""

    @overload
    def append(self, child: PageNode, move: bool = False) -> "Block": ...
    @overload
    def append[T: Node](self, child: T, move: bool = False) -> T: ...
    def append[T: Node](self, child: T, move: bool = False) -> "T | Block":
        if not move and isinstance(child, PageNode):
            # wrap PageNodes into Blocks
            child_block = child.wrap_in_block()
            self.blocks.append(child_block)
            super().append(child, move)
            return child_block
        else:
            return super().append(child, move)

    @overload
    def extend(self, *children: "PageNode", move: bool = False) -> "Sequence[Block]": ...
    @overload
    def extend[T: Node](self, *children: T, move: bool = False) -> "Sequence[T | Block]": ...
    def extend[T: Node](self, *children: T, move: bool = False) -> "Sequence[T | Block]":
        if not move and isinstance(children[0], PageNode):
            # wrap PageNodes into Blocks
            child_blocks = [cast(PageNode, child).wrap_in_block() for child in children]
            self.blocks.extend(*child_blocks)
            for child in children:
                super().append(child, move)
            return child_blocks
        else:
            return super().extend(*children, move=move)

    def add_text(
        self, text: TextIn, after: Optional["Block"] = None, before: Optional["Block"] = None
    ) -> list[Block]:
        """Add text to the Page."""
        text = to_text(text)
        blocks = Block.from_text(text)
        self.blocks.extend(*blocks, after=after, before=before)
        return blocks

    def remove_text(self, after: Optional["Block"] = None, before: Optional["Block"] = None):
        """Remove text from the Page (between Blocks, exclusive)."""
        blocks = self.blocks.between(after, before)
        if any(not b.type.is_text for b in blocks):
            bad_blocks = [b for b in blocks if not b.type.is_text]
            raise ValueError(f"cannot remove non-text blocks with Page.remove_text: {bad_blocks}")
        for block in blocks:
            self.blocks.remove(block)

    def remove_range(self, start: Optional["Block"] = None, end: Optional["Block"] = None):
        """Remove a range of Blocks."""
        blocks = self.blocks.between(start, end)
        self.blocks.remove(*blocks)

    @staticmethod
    def new(title: "TextLineIn", *nodes: "Block | PageNode", **kwargs) -> "Page":
        page = Page(title=text_line(title), **kwargs)
        for node in nodes:
            page.append(node)
        return page
