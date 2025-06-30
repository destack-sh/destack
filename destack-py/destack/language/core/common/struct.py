from typing import TYPE_CHECKING

from ..builtin import (
    Entity,
    HasIcon,
    HasName,
    IsDeletable,
    IsExtensible,
    IsSourceable,
    IsSpatial,
    IsTaggable,
    NodeType,
    builtin_node,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.CUSTOM_STRUCT_DEFINITION)
class CustomStructDefinition(
    IsSpatial,
    HasName,
    HasIcon,
    IsTaggable,
    IsDeletable,
    IsSourceable,
    IsExtensible,
    Entity,
):
    """A CustomStructDefinition describes a custom Type with Fields."""

    pass
