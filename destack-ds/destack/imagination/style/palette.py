from typing import TYPE_CHECKING

from destack.core import (
    Entity,
    NodeType,
    declare_entity,
)

if TYPE_CHECKING:
    pass


@declare_entity(NodeType.PALETTE)
class Palette(Entity):
    """A Palette of Colors."""
