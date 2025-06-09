from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    HasIcon,
    HasName,
    HasSlug,
    IsArchivable,
    IsDeletable,
    IsEnvironmental,
    IsInFolder,
    IsOwnable,
    IsTracked,
    Node,
    NodeType,
    node_,
    property_,
)
from destack.pb2 import RouteData

if TYPE_CHECKING:
    from destack.language import Scene

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.ROUTE)
class Route(
    HasIcon,
    HasSlug,
    HasName,
    IsEnvironmental,
    IsArchivable,
    IsDeletable,
    IsOwnable,
    IsInFolder,
    IsTracked,
    Node[RouteData],
):
    """A Route is a path to a Scene."""

    scene: Optional["Scene"] = property_(
        40,
        description="The Scene to route to.",
    )
