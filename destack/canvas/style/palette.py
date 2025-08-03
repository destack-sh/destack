from typing import TYPE_CHECKING

from destack.core import (
    Entity,
    NodeType,
    declare_entity,
    declare_property,
)

if TYPE_CHECKING:
    from destack import Icon

# pyright: reportIncompatibleVariableOverride=false


@declare_entity(NodeType.PALETTE)
class Palette(
    Entity,
):
    """A Palette of Colors."""

    icon: "Icon | None" = declare_property(102)
