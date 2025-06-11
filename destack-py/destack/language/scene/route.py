from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    HasIcon,
    HasName,
    IsArchivable,
    IsDeletable,
    IsOwnable,
    IsSpatial,
    IsTaggable,
    IsTracked,
    Node,
    NodeType,
    node_,
    property_,
    property_parent_,
)
from destack.pb2 import RouteData

if TYPE_CHECKING:
    from destack.language import Folder, Scene

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.ROUTE)
class Route(
    HasIcon,
    HasName,
    IsTaggable,
    IsArchivable,
    IsDeletable,
    IsOwnable,
    IsSpatial,
    IsTracked,
    Node[RouteData],
):
    """A Route is a path to a Scene."""

    parent: Optional["Folder"] = property_parent_(node_is_customizable=False)
    scene: Optional["Scene"] = property_(
        40,
        description="The Scene to route to.",
    )
