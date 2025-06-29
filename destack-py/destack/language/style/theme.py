from typing import TYPE_CHECKING

from destack.language.core import (
    Entity,
    HasIcon,
    HasName,
    IsDeletable,
    IsOrdered,
    IsTaggable,
    Node,
    NodeType,
    Spatial,
    builtin_node,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.THEME)
class Theme(
    Spatial,
    Entity,
    HasName,
    HasIcon,
    IsOrdered,
    IsTaggable,
    IsDeletable,
    Node,
):
    """A Theme with common Styles."""
