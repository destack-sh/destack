from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    HasName,
    IsDeletable,
    IsOrdered,
    IsOwnable,
    IsSpatial,
    IsTaggable,
    NodeType,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import Folder, Scene

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.ROUTE)
class Route(
    IsSpatial,
    HasName,
    IsDeletable,
    IsOrdered,
    IsOwnable,
    IsTaggable,
    Entity,
):
    """A Route is a path to a Scene."""

    parent: Optional["Folder"] = builtin_property_parent(node_is_extensible=False)
    scene: Optional["Scene"] = builtin_property(
        40,
        description="The Scene to route to.",
    )
