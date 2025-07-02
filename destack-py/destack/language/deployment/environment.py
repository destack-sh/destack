from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    IsDeletable,
    IsSpatial,
    NodeType,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import Icon, Space

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.ENVIRONMENT)
class Environment(IsSpatial, IsDeletable, Entity):
    """An Environment is a deployment of a Space."""

    parent: Optional["Space"] = builtin_property_parent(node_is_extensible=False)

    name: str = builtin_property(101, is_repr=True)
    icon: "Icon | None" = builtin_property(102)
