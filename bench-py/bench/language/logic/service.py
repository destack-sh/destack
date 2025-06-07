from typing import TYPE_CHECKING

from bench.language.core import (
    HasEnvironment,
    HasName,
    IsArchivable,
    IsBlockable,
    IsDeletable,
    IsOwnable,
    IsRunnable,
    IsScriptable,
    IsTemplatable,
    IsTracked,
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
    HasEnvironment,
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
