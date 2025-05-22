from typing import TYPE_CHECKING

from bench.language.core import (
    IsBlockable,
    IsClaimable,
    IsModal,
    IsNamed,
    IsOwnable,
    IsRunnable,
    IsTemplatable,
    Node,
    NodeType,
    node_,
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
    IsRunnable,
    IsBlockable,
    Node[ServiceData],
):
    """
    A set of Actions for a Node.
    """

    @staticmethod
    def new(name: str, **kwargs) -> "Service":
        return Service(name=name, **kwargs)
