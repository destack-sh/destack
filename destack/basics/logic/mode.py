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

# pyright: reportIncompatibleVariableOverride=false


@declare_entity(NodeType.MODE)
class Mode(Entity):
    """An Mode is a deployment scenario of a Space."""

    parent: Optional["Space"] = declare_property_parent()

    icon: "Icon | None" = declare_property(102)
