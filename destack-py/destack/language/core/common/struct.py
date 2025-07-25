from typing import TYPE_CHECKING, Optional

from ..builtin import (
    Entity,
    NodeType,
    builtin_node,
    builtin_property,
)

if TYPE_CHECKING:
    from destack.language import Icon, StructDefinitionReference

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.CUSTOM_STRUCT)
class CustomStruct(
    Entity,
):
    """A CustomStruct describes a custom Struct with custom Properties."""

    icon: "Icon | None" = builtin_property(102)

    base_type: Optional["StructDefinitionReference"] = builtin_property(110)
