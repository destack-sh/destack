from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    HasIcon,
    HasName,
    IsDeletable,
    Node,
    NodeType,
    Spatial,
    builtin_node,
    property_parent_,
)

if TYPE_CHECKING:
    from destack.language import Space

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.ENVIRONMENT)
class Environment(
    Spatial,
    Entity,
    HasName,
    HasIcon,
    IsDeletable,
    Node,
):
    """An Environment is a deployment of a Space."""

    parent: Optional["Space"] = property_parent_(node_is_customizable=False)
