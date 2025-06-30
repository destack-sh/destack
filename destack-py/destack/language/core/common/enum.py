from typing import TYPE_CHECKING

from ..builtin import (
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
from .entity import Entity

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.CUSTOM_ENUM_DEFINITION)
class CustomEnumDefinition(
    IsSpatial,
    HasName,
    HasIcon,
    IsTaggable,
    IsDeletable,
    IsSourceable,
    IsExtensible,
    Entity,
):
    """A CustomEnumDefinition describes a custom Enum with Options."""

    pass
