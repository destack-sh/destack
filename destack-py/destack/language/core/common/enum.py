from typing import TYPE_CHECKING, Union

from ..builtin import (
    Entity,
    HasIcon,
    HasName,
    IsCustomizable,
    IsDeletable,
    IsSourceable,
    IsSpatial,
    IsTaggable,
    NodeType,
    builtin_node,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import CustomProperty, CustomStructDefinition

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.CUSTOM_ENUM_DEFINITION)
class CustomEnumDefinition(
    IsSpatial,
    HasName,
    HasIcon,
    IsTaggable,
    IsDeletable,
    IsSourceable,
    IsCustomizable,
    Entity,
):
    """A CustomEnumDefinition describes a custom Enum with Options."""

    pass


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
    parent: Union["CustomStructDefinition", "CustomProperty", None] = builtin_property_parent(
        node_is_extensible=True
    )
