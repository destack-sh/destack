from typing import TYPE_CHECKING

from destack.language.core import (
    HasName,
    IsActionable,
    IsDeletable,
    IsEntity,
    IsOwnable,
    IsRunnable,
    IsScriptable,
    IsTaggable,
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
    IsTaggable,
    IsTemplatable,
    IsActionable,
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
