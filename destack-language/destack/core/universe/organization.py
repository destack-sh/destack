from typing import TYPE_CHECKING, final

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
    NodeType.ORGANIZATION,
    is_final=True,
    traits=(TraitType.ACTOR, TraitType.JOINABLE),
)
@final
class Organization(Entity):
    """
    An Organization with Users and Teams.
    """

    slug: str = declare_property(101, is_repr=True, tag=None)
