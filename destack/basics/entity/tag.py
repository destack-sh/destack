from typing import (
    TYPE_CHECKING,
    final,
)

from ..core.builtin.builtin import NodeType, TraitType
from ..core.builtin.const import UNSET
from ..core.builtin.entity import builtin_entity
from ..core.builtin.property import (
    builtin_property,
)

if TYPE_CHECKING:
    from destack import Icon, NodeReference

from ..core.builtin.entity import Entity

# pyright: reportIncompatibleVariableOverride=false


@builtin_entity(
    NodeType.TAG,
    traits=(TraitType.ORDERED,),
)
class Tag(Entity):
    """A Tag to tag an Entity with (in a Tagging)."""

    icon: "Icon | None" = builtin_property(102)


@builtin_entity(
    NodeType.TAGGING,
    traits=(TraitType.ORDERED,),
    is_final=True,
)
@final
class Tagging(Entity):
    """A Tagging of a Node by a Tag."""

    tag: Tag = builtin_property(110)
    if TYPE_CHECKING:
        tag_ptr: NodeReference = UNSET
