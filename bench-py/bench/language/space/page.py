from typing import TYPE_CHECKING, Optional, Union

from bench.language.core import (
    HasEnvironment,
    HasIcon,
    HasSlug,
    HasTitle,
    IsArchivable,
    IsBlockable,
    IsDeletable,
    IsInPackage,
    IsOwnable,
    IsTemplatable,
    IsTracked,
    Node,
    NodeType,
    TextIn,
    node_,
    property_parent_,
    to_text,
)
from bench.pb2 import BlockData

if TYPE_CHECKING:
    from bench.language import Block, Package

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.PAGE)
class Page(
    HasEnvironment,
    HasTitle,
    HasIcon,
    HasSlug,
    IsTemplatable,
    IsOwnable,
    IsBlockable,
    IsDeletable,
    IsArchivable,
    IsInPackage,
    IsTracked,
    Node[BlockData],
):
    """A Page of Blocks laying out rich Text, data, logic, resources -- anything software needs."""

    # meta
    parent: Union["Package", "Page", None] = property_parent_()
    # NOTE: :Architecture: maybe some IsBlockables should have their own Page? or is that confusing?
    # app? scene? plugin? Page/Record/View/... tying? :NodeTying

    def add_text(
        self, text: TextIn, after: Optional["Block"] = None, before: Optional["Block"] = None
    ) -> list["Block"]:
        """Add text to the Page."""
        text = to_text(text)
        blocks = Block.from_text(text)
        raise NotImplementedError("TODO: :Incomplete: add_children(..., before=..., after=...)")
        # self.add_children(*blocks, after=after, before=before)
        return blocks
