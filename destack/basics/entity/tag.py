from typing import TYPE_CHECKING, Optional

from destack.core import Entity, NodeType, TraitType, Type, declare_entity, declare_property

if TYPE_CHECKING:
    pass


@declare_entity(
    NodeType.TAG,
    traits=(TraitType.ORDERED,),
)
class Tag(Entity):
    """A Tag to tag an Entity with."""

    type: Optional["Type"] = declare_property(
        100,
        description="The designated Type for Values associated with this Tag. Any if unset.",
        tag=None,
    )
