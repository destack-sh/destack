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
    from destack.language import Icon

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.PALETTE)
class Palette(
    IsOrdered,
    IsTaggable,
    IsDeletable,
    Entity,
):
    """A Palette of Colors."""

    icon: "Icon | None" = builtin_property(102)
