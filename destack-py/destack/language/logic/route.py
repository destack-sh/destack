from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    HasName,
    IsDeletable,
    IsOrdered,
    IsOwnable,
    IsTaggable,
    Node,
    NodeType,
    Spatial,
    builtin_node,
    property_,
    property_parent_,
)
from destack.pb2 import RouteData

if TYPE_CHECKING:
    from destack.language import Folder, Scene

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.ROUTE)
class Route(
    Spatial,
    Entity,
    HasName,
    IsDeletable,
    IsOrdered,
    IsOwnable,
    IsTaggable,
    Node[RouteData],
):
    """A Route is a path to a Scene."""

    parent: Optional["Folder"] = property_parent_(node_is_customizable=False)
    scene: Optional["Scene"] = property_(
        40,
        description="The Scene to route to.",
    )
