from typing import TYPE_CHECKING

from ..builtin import (
    Entity,
    NodeType,
    TraitType,
    declare_entity,
    declare_property,
)

if TYPE_CHECKING:
    pass


@declare_entity(
    NodeType.TEAM,
    traits=(TraitType.ACTOR, TraitType.JOINABLE),
)
class Team(Entity):
    """
    An Team with Users and Teams.
    """

    slug: str = declare_property(102, is_repr=True, tag=None)
