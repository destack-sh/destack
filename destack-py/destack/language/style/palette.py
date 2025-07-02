from typing import TYPE_CHECKING

from destack.language.core import (
    Entity,
    IsDeletable,
    IsOrdered,
    IsSpatial,
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
    IsSpatial,
    IsOrdered,
    IsTaggable,
    IsDeletable,
    Entity,
):
    """A Palette of Colors."""

    name: str = builtin_property(101, is_repr=True)
    icon: "Icon | None" = builtin_property(102)
