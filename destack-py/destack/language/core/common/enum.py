from typing import TYPE_CHECKING, Union

from ..builtin import (
    Entity,
    IsCustomizable,
    IsDeletable,
    IsSourceable,
    IsSpatial,
    IsTaggable,
    NodeType,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import CustomProperty, CustomStructDefinition, Icon
# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.CUSTOM_ENUM_DEFINITION)
class CustomEnumDefinition(
    IsSpatial,
    IsTaggable,
    IsDeletable,
    IsSourceable,
    IsCustomizable,
    Entity,
):
    """A CustomEnumDefinition describes a custom Enum with Options."""

    name: str = builtin_property(101, is_repr=True)
    icon: "Icon | None" = builtin_property(102)


@builtin_node(NodeType.CUSTOM_OPTION)
class CustomOption(
    IsSpatial,
    IsTaggable,
    IsDeletable,
    IsSourceable,
    Entity,
):
    parent: Union["CustomStructDefinition", "CustomProperty", None] = builtin_property_parent()

    name: str = builtin_property(101, is_repr=True)
    icon: "Icon | None" = builtin_property(102)
