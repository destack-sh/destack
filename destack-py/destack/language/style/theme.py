from typing import TYPE_CHECKING

from destack.language.core import (
    Entity,
    IsExtensible,
    IsOrdered,
    IsTaggable,
    NodeType,
    builtin_node,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.THEME)
class Theme(
    IsExtensible,
    IsTaggable,
    IsOrdered,
    Entity,
):
    """A Theme with common Styles."""
