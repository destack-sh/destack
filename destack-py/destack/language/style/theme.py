from typing import TYPE_CHECKING

from destack.language.core import (
    Entity,
    IsDeletable,
    IsOrdered,
    IsTaggable,
    NodeType,
    builtin_node,
    builtin_property,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.THEME)
class Theme(
    IsOrdered,
    IsTaggable,
    IsDeletable,
    Entity,
):
    """A Theme with common Styles."""

    name: str = builtin_property(101, is_repr=True)
