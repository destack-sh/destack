from typing import TYPE_CHECKING

from ..builtin import Entity, NodeType, builtin_entity

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_entity(NodeType.PERMISSION)
class Permission(
    Entity,
):
    """A Permission for something."""
