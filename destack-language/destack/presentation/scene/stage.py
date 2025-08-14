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
    NodeType.STAGE,
    traits=(TraitType.OWNABLE, TraitType.ORDERED, TraitType.JOINABLE),
)
class Stage(Entity):
    """
    A Stage for someone to interact with a Space.
    """

    pass
