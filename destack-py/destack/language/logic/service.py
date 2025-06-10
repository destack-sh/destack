from typing import TYPE_CHECKING

from destack.language.core import (
    HasName,
    IsDeletable,
    IsEntity,
    IsEnvironmental,
    IsOwnable,
    IsRunnable,
    IsScriptable,
    IsTemplatable,
    Node,
    NodeType,
    node_,
)
from destack.pb2 import ServiceData

if TYPE_CHECKING:
    pass


# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


@node_(NodeType.SERVICE)
class Service(
    HasName,
    IsEnvironmental,
    IsTemplatable,
    IsOwnable,
    IsDeletable,
    IsRunnable,
    IsScriptable,
    IsEntity,
    Node[ServiceData],
):
    """
    A set of Actions for a Node.
    """
