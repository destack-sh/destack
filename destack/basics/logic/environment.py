from typing import TYPE_CHECKING, Optional

from destack.core import (
    Entity,
    NodeType,
    builtin_entity,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack import Icon, Space

# pyright: reportIncompatibleVariableOverride=false


@builtin_entity(NodeType.ENVIRONMENT)
class Environment(Entity):
    """An Environment is a deployment scenario of a Space."""

    parent: Optional["Space"] = builtin_property_parent()

    icon: "Icon | None" = builtin_property(102)
