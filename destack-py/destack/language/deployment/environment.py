from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    HasIcon,
    HasName,
    IsDeletable,
    IsSpatial,
    NodeType,
    builtin_node,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import Space

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.ENVIRONMENT)
class Environment(IsSpatial, HasName, HasIcon, IsDeletable, Entity):
    """An Environment is a deployment of a Space."""

    parent: Optional["Space"] = builtin_property_parent(node_is_extensible=False)
