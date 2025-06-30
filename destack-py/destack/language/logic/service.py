from typing import TYPE_CHECKING

from destack.language.core import (
    Entity,
    HasName,
    IsDeletable,
    IsExtensible,
    IsOwnable,
    IsRunnable,
    IsScriptable,
    IsSourceable,
    IsSpatial,
    IsTaggable,
    NodeType,
    builtin_node,
)

if TYPE_CHECKING:
    pass


# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


@builtin_node(NodeType.SERVICE)
class Service(
    IsSpatial,
    HasName,
    IsDeletable,
    IsOwnable,
    IsTaggable,
    IsRunnable,
    IsScriptable,
    IsSourceable,
    IsExtensible,
    Entity,
):
    """
    A set of Actions for a Node.
    """
