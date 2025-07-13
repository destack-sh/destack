from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    IsDeletable,
    IsOrdered,
    IsOwnable,
    IsTaggable,
    NodeType,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import Folder

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.ROUTE, is_abstract=True)
class Route(
    IsDeletable,
    IsOrdered,
    IsOwnable,
    IsTaggable,
    Entity,
):
    """A Route is a path to something (a Scene, a View in a Scene, an Action, etc.)."""

    parent: Optional["Folder"] = builtin_property_parent()

    name: str = builtin_property(101, description="The name of the Route.")
