from typing import TYPE_CHECKING

from destack.language.core import (
    Entity,
    HasName,
    IsActionable,
    IsDeletable,
    IsExtensible,
    IsOwnable,
    IsRunnable,
    IsScriptable,
    IsSourceable,
    IsTaggable,
    IsTemplatable,
    Node,
    NodeType,
    Spatial,
    builtin_node,
)
from destack.pb2 import ServiceData

if TYPE_CHECKING:
    pass


# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


@builtin_node(NodeType.SERVICE)
class Service(
    Spatial,
    Entity,
    HasName,
    IsActionable,
    IsDeletable,
    IsOwnable,
    IsTaggable,
    IsTemplatable,
    IsRunnable,
    IsScriptable,
    IsSourceable,
    IsExtensible,
    Node[ServiceData],
):
    """
    A set of Actions for a Node.
    """
