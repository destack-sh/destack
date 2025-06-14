from typing import TYPE_CHECKING

from destack.pb2 import CustomEnumDefinitionData

from ..builtin import (
    HasIcon,
    HasName,
    Instance,
    IsDeletable,
    IsExtensible,
    IsSourceable,
    IsTaggable,
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
    Instance,
    HasName,
    HasIcon,
    IsTaggable,
    IsDeletable,
    IsSourceable,
    IsExtensible,
    Node[CustomEnumDefinitionData],
):
    """A CustomEnumDefinition describes a custom Enum with Options."""

    pass
