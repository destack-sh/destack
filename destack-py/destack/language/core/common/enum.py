from typing import TYPE_CHECKING

from destack.pb2 import CustomEnumDefinitionData

from ..builtin import (
    Entity,
    HasIcon,
    HasName,
    IsDeletable,
    IsExtensible,
    IsSourceable,
    IsTaggable,
    IsTemplatable,
    Node,
    NodeType,
    Spatial,
    builtin_node,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.CUSTOM_ENUM_DEFINITION)
class CustomEnumDefinition(
    Spatial,
    Entity,
    HasName,
    HasIcon,
    IsTaggable,
    IsTemplatable,
    IsDeletable,
    IsSourceable,
    IsExtensible,
    Node[CustomEnumDefinitionData],
):
    """A CustomEnumDefinition describes a custom Enum with Options."""

    pass
