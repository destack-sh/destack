from typing import (
    TYPE_CHECKING,
)

from destack.core import Entity, NodeType, TraitType, Type, declare_entity, declare_property

if TYPE_CHECKING:
    pass


@declare_entity(
    NodeType.TAG,
    traits=(TraitType.ORDERED,),
)
class Tag(Entity):
    """A Tag to tag an Entity with."""

    type: "Type | None" = declare_property(
        100,
        description="The designated Type of this Tag. Any if unset.",
    )
