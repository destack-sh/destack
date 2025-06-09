from typing import TYPE_CHECKING

from destack.language.core import (
    HasName,
    IsArchivable,
    IsBlockable,
    IsDeletable,
    IsEnvironmental,
    IsOwnable,
    IsRunnable,
    IsScriptable,
    IsTemplatable,
    IsTracked,
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
    IsEnvironmental,
    HasName,
    IsTemplatable,
    IsOwnable,
    IsDeletable,
    IsArchivable,
    IsRunnable,
    IsBlockable,
    IsScriptable,
    IsTracked,
    Node[ServiceData],
):
    """
    A set of Actions for a Node.
    """
