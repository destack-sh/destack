from typing import TYPE_CHECKING

from destack.language.core import (
    Entity,
    HasName,
    IsCustomizable,
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
    IsExtensible,
    IsSourceable,
    IsCustomizable,
    Entity,
):
    """
    A set of Actions for a Node.
    """
