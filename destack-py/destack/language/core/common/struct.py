from typing import TYPE_CHECKING

from destack.proto import CustomStructDefinitionProto

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
    Node[CustomStructDefinitionProto],
):
    """A CustomStructDefinition describes a custom Type with Fields."""

    pass
