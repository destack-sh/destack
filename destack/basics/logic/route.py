from typing import TYPE_CHECKING

from destack.core import (
    Entity,
    NodeType,
    TraitType,
    declare_entity,
)

if TYPE_CHECKING:
    pass


@declare_entity(
    NodeType.ROUTE,
    is_abstract=True,
    traits=(TraitType.ORDERED, TraitType.OWNABLE),
)
class Route(Entity):
    """A Route is a path to something (a Scene, a View in a Scene, an Action, etc.)."""

    pass
