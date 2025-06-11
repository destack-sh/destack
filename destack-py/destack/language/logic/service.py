from typing import TYPE_CHECKING

from destack.language.core import (
    Entity,
    HasName,
    IsActionable,
    IsDeletable,
    IsOwnable,
    IsRunnable,
    IsScriptable,
    IsTaggable,
    IsTemplatable,
    Node,
    NodeType,
    Spatial,
    node_,
)
from destack.pb2 import ServiceData

if TYPE_CHECKING:
    pass


# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


@node_(NodeType.SERVICE)
class Service(
    Spatial,
    Entity,
    HasName,
    IsTaggable,
    IsTemplatable,
    IsActionable,
    IsOwnable,
    IsDeletable,
    IsRunnable,
    IsScriptable,
    Node[ServiceData],
):
    """
    A set of Actions for a Node.
    """
