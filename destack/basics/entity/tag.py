from typing import (
    TYPE_CHECKING,
    final,
)

from destack.core import UNSET, Entity, NodeType, TraitType, declare_entity, declare_property

if TYPE_CHECKING:
    from destack import Icon, NodeReference

# pyright: reportIncompatibleVariableOverride=false


@declare_entity(
    NodeType.TAG,
    traits=(TraitType.ORDERED,),
)
class Tag(Entity):
    """A Tag to tag an Entity with (in a Tagging)."""

    icon: "Icon | None" = declare_property(102)


@declare_entity(
    NodeType.TAGGING,
    traits=(TraitType.ORDERED,),
    is_final=True,
)
@final
class Tagging(Entity):
    """A Tagging of a Node by a Tag."""

    tag: Tag = declare_property(110)
    if TYPE_CHECKING:
        tag_ptr: NodeReference = UNSET
