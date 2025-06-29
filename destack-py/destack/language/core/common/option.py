from typing import TYPE_CHECKING, Union

from ..builtin import (
    Entity,
    HasIcon,
    HasName,
    IsDeletable,
    IsSourceable,
    IsTaggable,
    Node,
    NodeType,
    Spatial,
    builtin_node,
    property_parent_,
)

if TYPE_CHECKING:
    from destack.language import CustomProperty, CustomStructDefinition

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.CUSTOM_OPTION)
class CustomOption(
    Spatial,
    Entity,
    HasName,
    HasIcon,
    IsTaggable,
    IsDeletable,
    IsSourceable,
    Node,
):
    parent: Union["CustomStructDefinition", "CustomProperty", None] = property_parent_(
        node_is_customizable=True
    )
