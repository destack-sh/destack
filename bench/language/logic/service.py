from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    IsClaimable,
    IsModal,
    IsNamed,
    IsOwnable,
    IsTemplatable,
    NodeType,
    PageNode,
    node_,
    p_regular,
)
from bench.pb2 import ServiceData

if TYPE_CHECKING:
    pass


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

    target: Optional["PageNode"] = p_regular(40)

    @staticmethod
    def new(name: str, **kwargs) -> "Service":
        return Service(name=name, **kwargs)
