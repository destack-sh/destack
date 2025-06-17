from typing import TYPE_CHECKING

from destack.language.core import (
    Entity,
    HasIcon,
    HasName,
    IsDeletable,
    IsOrdered,
    IsTaggable,
    IsVisual,
    Node,
    NodeType,
    Spatial,
    builtin_node,
)
from destack.proto import ThemeProto

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.THEME)
class Theme(
    Spatial,
    Entity,
    HasName,
    HasIcon,
    IsVisual,
    IsOrdered,
    IsTaggable,
    IsDeletable,
    Node[ThemeProto],
):
    """A Theme with common Styles."""
