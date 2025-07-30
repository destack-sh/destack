from typing import TYPE_CHECKING

from destack.language.core import (
    Entity,
    NodeType,
    TraitType,
    builtin_entity,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_entity(
    NodeType.STAGE,
    traits=(TraitType.OWNABLE, TraitType.ORDERED, TraitType.JOINABLE),
)
class Stage(Entity):
    """
    A Stage for someone to interact with a Space.
    """

    pass
