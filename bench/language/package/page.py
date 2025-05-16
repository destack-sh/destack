from typing import TYPE_CHECKING, Optional, Sequence, Union, cast, overload

from bench.language.core import (
    IsClaimable,
    IsModal,
    IsOwnable,
    IsTemplatable,
    IsTitled,
    Node,
    NodeType,
    PageNode,
    TextIn,
    TextLineIn,
    node_,
    p_node_parent,
    text_line,
    to_text,
)
from bench.language.package.block import Block
from bench.pb2 import BlockData

if TYPE_CHECKING:
    from bench.language import Block, Package

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
    # app? scene? plugin? Page/Record/View/... tying? :NodeTying

    def __content_str__(self):
        return ""

    @overload
    def add_child(self, child: PageNode, move: bool = False) -> "Block": ...
    @overload
    def add_child[T: Node](self, child: T, move: bool = False) -> T: ...
    def add_child[T: Node](self, child: T, move: bool = False) -> "T | Block":
        if not move and isinstance(child, PageNode):
            # wrap PageNodes into Blocks
            child_block = child.wrap_in_block()
            super().add_child(child_block, move)
            super().add_child(child, move)
            return child_block
        else:
            return super().add_child(child, move)

    @overload
    def add_children(self, *children: "PageNode", move: bool = False) -> "Sequence[Block]": ...
    @overload
    def add_children[T: Node](self, *children: T, move: bool = False) -> "Sequence[T | Block]": ...
    def add_children[T: Node](self, *children: T, move: bool = False) -> "Sequence[T | Block]":
        if not move and isinstance(children[0], PageNode):
            # wrap PageNodes into Blocks
            child_blocks = [cast(PageNode, child).wrap_in_block() for child in children]
            super().add_children(*child_blocks, move=move)
            super().add_children(*children, move=move)
            return child_blocks
        else:
            return super().add_children(*children, move=move)

    def add_text(
        self, text: TextIn, after: Optional["Block"] = None, before: Optional["Block"] = None
    ) -> list[Block]:
        """Add text to the Page."""
        text = to_text(text)
        blocks = Block.from_text(text)
        raise NotImplementedError("TODO: :Incomplete: add_children(..., before=..., after=...)")
        # self.add_children(*blocks, after=after, before=before)
        return blocks

    @staticmethod
    def new(title: "TextLineIn", *nodes: "Block | PageNode", **kwargs) -> "Page":
        page = Page(title=text_line(title), **kwargs)
        for node in nodes:
            page.add_child(node)
        return page
