import typing
from typing import Optional

from bench.language.core import (
    PAGE_NODE_TYPES,
    IsClaimable,
    IsModal,
    IsNamed,
    IsOwnable,
    IsTemplatable,
    LocalNodeList,
    NodeType,
    PageNode,
    node_,
    p_node_children,
    p_regular,
)
from bench.pb2 import KitData

if typing.TYPE_CHECKING:
    from bench.language import Action, Claim


# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


@node_(NodeType.KIT)
class Kit(
    IsTemplatable,
    IsOwnable,
    IsClaimable,
    IsModal,
    IsNamed,
    PageNode[KitData],
):
    """
    A Kit of Actions for a Node.
    """

    target: Optional["PageNode"] = p_regular(
        40, require=False, array=False, references=PAGE_NODE_TYPES.tuple
    )

    actions: LocalNodeList["Action"] = p_node_children(NodeType.ACTION)
    claims: LocalNodeList["Claim"] = p_node_children(NodeType.CLAIM)

    @staticmethod
    def new(name: str, **kwargs) -> "Kit":
        return Kit(name=name, **kwargs)
