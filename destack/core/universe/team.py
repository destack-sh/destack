from typing import TYPE_CHECKING, Optional

from ..builtin import (
    Entity,
    NodeType,
    TraitType,
    declare_entity,
    declare_property,
    declare_property_parent,
)

if TYPE_CHECKING:
    from destack.core import Organization

# pyright: reportIncompatibleVariableOverride=false


@declare_entity(
    NodeType.TEAM,
    traits=(TraitType.ACTOR, TraitType.JOINABLE),
)
class Team(Entity):
    """
    An Team with Users and Teams.
    """

    parent: Optional["Organization"] = declare_property_parent()
    slug: str = declare_property(102, is_repr=True)
