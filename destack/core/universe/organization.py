from typing import TYPE_CHECKING, Optional, final

from ..builtin import (
    Entity,
    NodeType,
    TraitType,
    builtin_entity,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.core import Space

# pyright: reportIncompatibleVariableOverride=false


@builtin_entity(
    NodeType.ORGANIZATION,
    is_final=True,
    traits=(TraitType.ACTOR, TraitType.JOINABLE),
)
@final
class Organization(Entity):
    """
    An Organization with Users and Teams.
    """

    parent: Optional["Space"] = builtin_property_parent()
    slug: str = builtin_property(101, is_repr=True)
