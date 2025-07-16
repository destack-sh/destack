from typing import TYPE_CHECKING, Union

from destack.language.core import (
    Entity,
    IsOrdered,
    IsOwnable,
    IsScriptable,
    IsTaggable,
    NodeType,
    builtin_node,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import Folder

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.ROUTE, is_abstract=True)
class Route(
    IsOrdered,
    IsOwnable,
    IsTaggable,
    Entity,
):
    """A Route is a path to something (a Scene, a View in a Scene, an Action, etc.)."""

    parent: Union["Folder", "IsScriptable", None] = builtin_property_parent()
