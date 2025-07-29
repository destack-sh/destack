from typing import TYPE_CHECKING

from destack.language.core import (
    Entity,
    NodeType,
    TraitType,
    builtin_node,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(
    NodeType.ROUTE,
    is_abstract=True,
    traits=(TraitType.ORDERED, TraitType.OWNABLE),
)
class Route(Entity):
    """A Route is a path to something (a Scene, a View in a Scene, an Action, etc.)."""

    pass
