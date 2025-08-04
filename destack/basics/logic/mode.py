from typing import TYPE_CHECKING

from destack.core import (
    Entity,
    NodeType,
    declare_entity,
    declare_property,
)

if TYPE_CHECKING:
    from destack import Icon


@declare_entity(NodeType.MODE)
class Mode(Entity):
    """A Mode is a usage scenario."""

    icon: "Icon | None" = declare_property(102)
