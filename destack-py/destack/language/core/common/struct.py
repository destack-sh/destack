from typing import TYPE_CHECKING

from destack.proto import CustomStructDefinitionData

from ..builtin import (
    Entity,
    HasIcon,
    HasName,
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


@builtin_node(NodeType.CUSTOM_STRUCT_DEFINITION)
class CustomStructDefinition(
    Spatial,
    Entity,
    HasName,
    HasIcon,
    IsTaggable,
    IsDeletable,
    IsSourceable,
    IsExtensible,
    Node[CustomStructDefinitionData],
):
    """A CustomStructDefinition describes a custom Type with Fields."""

    pass
