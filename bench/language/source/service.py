from typing import TYPE_CHECKING, Optional

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
from bench.pb2 import ServiceData

if TYPE_CHECKING:
    from bench.language import Action, Claim


# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


@node_(NodeType.SERVICE)
class Service(
    IsTemplatable,
    IsOwnable,
    IsClaimable,
    IsModal,
    IsNamed,
    PageNode[ServiceData],
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
    def new(name: str, **kwargs) -> "Service":
        return Service(name=name, **kwargs)
