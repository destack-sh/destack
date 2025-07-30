from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    NodeType,
    TraitType,
    builtin_entity,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import Organization

# pyright: reportIncompatibleVariableOverride=false


@builtin_entity(
    NodeType.TEAM,
    traits=(TraitType.ACTOR, TraitType.JOINABLE),
)
class Team(Entity):
    """
    An Team with Users and Teams.
    """

    parent: Optional["Organization"] = builtin_property_parent()
    slug: str = builtin_property(102, is_repr=True)
