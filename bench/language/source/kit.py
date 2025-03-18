import typing
from typing import Optional, Union

from bench.language.core import (
    INLINE_NODE_TYPES,
    InlineNode,
    IsClaimable,
    IsModal,
    IsNamed,
    IsTemplatable,
    LocalNodeList,
    NodeType,
    StructType,
    node_,
    p_node_children,
    p_node_parent,
    p_regular,
)
from bench.pb2 import KitData

if typing.TYPE_CHECKING:
    from bench.language import Action, Claim, Package, Page, Text


# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


@node_(NodeType.KIT)
class Kit(IsTemplatable, IsClaimable, IsModal, IsNamed, InlineNode[KitData]):
    """
    A Kit of Actions for a Node.
    """

    parent: Union["Page", "Package", None] = p_node_parent(4, NodeType.PAGE, NodeType.PACKAGE)

    target: Optional["InlineNode"] = p_regular(
        40, require=False, array=False, references=INLINE_NODE_TYPES.tuple
    )
    text: Optional["Text"] = p_regular(
        41, default=None, require=False, array=False, struct=StructType.TEXT
    )

    actions: LocalNodeList["Action"] = p_node_children(NodeType.ACTION)
    claims: LocalNodeList["Claim"] = p_node_children(NodeType.CLAIM)

    @staticmethod
    def new(name: str, **kwargs) -> "Kit":
        return Kit(name=name, **kwargs)
