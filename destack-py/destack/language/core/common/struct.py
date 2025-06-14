from typing import TYPE_CHECKING

from destack.pb2 import CustomStructDefinitionData

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


@builtin_node(NodeType.CUSTOM_STRUCT_DEFINITION)
class CustomStructDefinition(
    Spatial,
    Instance,
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
