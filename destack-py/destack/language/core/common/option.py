from typing import TYPE_CHECKING, Union

from ..builtin import (
    HasIcon,
    HasName,
    IsDeletable,
    IsSourceable,
    IsSpatial,
    IsTaggable,
    NodeType,
    builtin_node,
    property_parent_,
)
from .entity import Entity

if TYPE_CHECKING:
    from destack.language import CustomProperty, CustomStructDefinition

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.CUSTOM_OPTION)
class CustomOption(
    IsSpatial,
    HasName,
    HasIcon,
    IsTaggable,
    IsDeletable,
    IsSourceable,
    Entity,
):
    parent: Union["CustomStructDefinition", "CustomProperty", None] = property_parent_(
        node_is_customizable=True
    )
