from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    HasIcon,
    HasName,
    HasSlug,
    IsArchivable,
    IsBlockable,
    IsDeletable,
    IsInPackage,
    IsModal,
    IsOwnable,
    Node,
    NodeType,
    node_,
    property_,
)
from bench.pb2 import RouteData

if TYPE_CHECKING:
    from bench.language import Scene

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.ROUTE)
class Route(
    IsArchivable,
    HasIcon,
    HasSlug,
    IsDeletable,
    IsOwnable,
    HasName,
    IsModal,
    IsBlockable,
    IsInPackage,
    Node[RouteData],
):
    """A Route is a path to a Scene."""

    scene: Optional["Scene"] = property_(
        40,
        description="The Scene to route to.",
    )
