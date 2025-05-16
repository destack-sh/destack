from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    IsInstantiable,
    IsModal,
    IsNamed,
    IsOwnable,
    NodeType,
    PageNode,
    node_,
    p_regular,
)
from bench.pb2 import RouteData

if TYPE_CHECKING:
    from bench.language import Scene

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.ROUTE)
class Route(IsOwnable, IsInstantiable, IsNamed, IsModal, PageNode[RouteData]):
    """A Route is a path to a Scene."""

    scene: Optional["Scene"] = p_regular(
        40,
        description="The Scene to route to.",
    )
