from typing import TYPE_CHECKING, Union

from ..builtin import (
    Entity,
    IsCustomizable,
    IsSourceable,
    NodeType,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import Icon
# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.CUSTOM_ENUM)
class CustomEnum(
    IsSourceable,
    IsCustomizable,
    Entity,
):
    """A CustomEnum describes a custom Enum with custom Options."""

    icon: "Icon | None" = builtin_property(102)


@builtin_node(NodeType.CUSTOM_OPTION)
class CustomOption(
    IsSourceable,
    Entity,
):
    parent: Union["CustomEnum", None] = builtin_property_parent()

    icon: "Icon | None" = builtin_property(102)
