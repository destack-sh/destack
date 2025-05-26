from typing import TYPE_CHECKING, Optional, Sequence, Union, cast, overload

from bench.language.core import (
    HasIcon,
    HasSlug,
    HasTitle,
    IsArchivable,
    IsBlockable,
    IsDeletable,
    IsModal,
    IsOwnable,
    IsTemplatable,
    Node,
    NodeType,
    TextIn,
    TextLineIn,
    node_,
    property_parent_,
    text_line,
    to_text,
)
from bench.pb2 import BlockData

if TYPE_CHECKING:
    from bench.language import Block, Package

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.PAGE)
class Page(
    IsTemplatable,
    IsModal,
    HasTitle,
    HasIcon,
    IsOwnable,
    HasSlug,
    IsBlockable,
    IsDeletable,
    IsArchivable,
    Node[BlockData],
):
    """A Page of Blocks laying out rich Text, data, logic, resources -- anything software needs."""

    # meta
    parent: Union["Package", "Page", None] = property_parent_()
    # NOTE: :Architecture: maybe some IsBlockables should have their own Page? or is that confusing?
    # app? scene? plugin? Page/Record/View/... tying? :NodeTying

    @overload
    def add_child(self, child: IsBlockable, move: bool = False) -> "Block": ...
    @overload
    def add_child[T: Node](self, child: T, move: bool = False) -> T: ...
    def add_child[T: Node](self, child: T, move: bool = False) -> "T | Block":
        if not move and isinstance(child, IsBlockable):
            # wrap IsBlockables into Blocks
            child_block = child.wrap_in_block()
            super().add_child(child_block, move)
            super().add_child(child, move)
            return child_block
        else:
            return super().add_child(child, move)

    @overload
    def add_children(self, *children: "IsBlockable", move: bool = False) -> "Sequence[Block]": ...
    @overload
    def add_children[T: Node](self, *children: T, move: bool = False) -> "Sequence[T | Block]": ...
    def add_children[T: Node](self, *children: T, move: bool = False) -> "Sequence[T | Block]":
        if not move and isinstance(children[0], IsBlockable):
            # wrap IsBlockables into Blocks
            child_blocks = [cast(IsBlockable, child).wrap_in_block() for child in children]
            super().add_children(*child_blocks, move=move)
            super().add_children(*children, move=move)
            return child_blocks
        else:
            return super().add_children(*children, move=move)

    def add_text(
        self, text: TextIn, after: Optional["Block"] = None, before: Optional["Block"] = None
    ) -> list["Block"]:
        """Add text to the Page."""
        text = to_text(text)
        blocks = Block.from_text(text)
        raise NotImplementedError("TODO: :Incomplete: add_children(..., before=..., after=...)")
        # self.add_children(*blocks, after=after, before=before)
        return blocks

    @staticmethod
    def new(title: "TextLineIn", *nodes: "Block | IsBlockable", **kwargs) -> "Page":
        page = Page(title=text_line(title), **kwargs)
        for node in nodes:
            page.add_child(node)
        return page
