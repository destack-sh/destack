from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    HasIcon,
    HasName,
    IsDeletable,
    IsSpatial,
    NodeType,
    builtin_node,
    property_parent_,
)

if TYPE_CHECKING:
    from destack.language import Space

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.ENVIRONMENT)
class Environment(IsSpatial, HasName, HasIcon, IsDeletable, Entity):
    """An Environment is a deployment of a Space."""

    parent: Optional["Space"] = property_parent_(node_is_customizable=False)
