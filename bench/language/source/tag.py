import typing
from typing import Union

from bench.language.core import (
    IsModal,
    IsNamed,
    IsTemplatable,
    NodeType,
    PageNode,
    node_,
    p_node_parent,
)
from bench.pb2 import TagData

if typing.TYPE_CHECKING:
    from bench.language import Package, Page


# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


@node_(NodeType.TAG)
class Tag(IsTemplatable, IsModal, IsNamed, PageNode[TagData]):
    """
    A Tag to tag other Nodes.
    """

    parent: Union["Page", "Package", None] = p_node_parent(4, NodeType.PAGE, NodeType.PACKAGE)

    @staticmethod
    def new(name: str, **kwargs) -> "Tag":
        return Tag(name=name, **kwargs)
