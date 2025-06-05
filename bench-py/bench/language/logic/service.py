from typing import TYPE_CHECKING

from bench.language.core import (
    HasEnvironment,
    HasName,
    IsArchivable,
    IsBlockable,
    IsDeletable,
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
    IsDeletable,
    IsArchivable,
    HasEnvironment,
    HasName,
    IsRunnable,
    IsBlockable,
    Node[ServiceData],
):
    """
    A set of Actions for a Node.
    """
