from typing import TYPE_CHECKING, Optional

from destack.core import (
    Entity,
    NodeType,
    declare_entity,
    declare_property,
    declare_property_parent,
)

if TYPE_CHECKING:
    from destack import Icon, Space


@declare_entity(NodeType.ENVIRONMENT)
class Environment(Entity):
    """An Environment is a deployment scenario of a Space."""

    parent: Optional["Space"] = declare_property_parent()

    icon: "Icon | None" = declare_property(102)
