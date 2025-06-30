from typing import TYPE_CHECKING

from destack.language.core import (
    Entity,
    HasIcon,
    HasName,
    IsDeletable,
    IsOrdered,
    IsSpatial,
    IsTaggable,
    NodeType,
    builtin_node,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.THEME)
class Theme(
    IsSpatial,
    HasName,
    HasIcon,
    IsOrdered,
    IsTaggable,
    IsDeletable,
    Entity,
):
    """A Theme with common Styles."""
