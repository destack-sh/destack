from typing import TYPE_CHECKING, Union

from ..builtin import (
    Entity,
    IsArchivable,
    IsCustomizable,
    IsDeletable,
    IsSourceable,
    IsTaggable,
    NodeType,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import Icon
# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.CUSTOM_ENUM_DEFINITION)
class CustomEnumDefinition(
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
    IsTaggable,
    IsArchivable,
    IsDeletable,
    IsSourceable,
    Entity,
):
    parent: Union["CustomEnumDefinition", None] = builtin_property_parent()

    name: str = builtin_property(101, is_repr=True)
    icon: "Icon | None" = builtin_property(102)
    group: "CustomOptionGroup | None" = builtin_property(105)


@builtin_node(NodeType.CUSTOM_OPTION_GROUP)
class CustomOptionGroup(
    IsArchivable,
    IsDeletable,
    IsSourceable,
    Entity,
):
    """A CustomOptionGroup is a group of CustomOptions."""

    parent: Union["CustomEnumDefinition", None] = builtin_property_parent()

    name: str = builtin_property(101, is_repr=True)
    icon: "Icon | None" = builtin_property(102)
