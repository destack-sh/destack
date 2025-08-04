from typing import TYPE_CHECKING, Optional, final

from ..builtin import (
    Entity,
    NodeType,
    TraitType,
    declare_entity,
    declare_property,
    declare_property_parent,
)

if TYPE_CHECKING:
    from destack.core import Space


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

    parent: Optional["Space"] = declare_property_parent()
    slug: str = declare_property(101, is_repr=True)
