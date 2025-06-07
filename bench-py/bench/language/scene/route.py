from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    HasEnvironment,
    HasIcon,
    HasName,
    HasSlug,
    IsArchivable,
    IsBlockable,
    IsDeletable,
    IsInPackage,
    IsOwnable,
    IsTracked,
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
    HasIcon,
    HasSlug,
    HasName,
    HasEnvironment,
    IsArchivable,
    IsDeletable,
    IsOwnable,
    IsBlockable,
    IsInPackage,
    IsTracked,
    Node[RouteData],
):
    """A Route is a path to a Scene."""

    scene: Optional["Scene"] = property_(
        40,
        description="The Scene to route to.",
    )
