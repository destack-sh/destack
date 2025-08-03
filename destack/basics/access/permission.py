from typing import TYPE_CHECKING

from ..core.builtin import Entity, NodeType, declare_entity

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@declare_entity(NodeType.PERMISSION)
class Permission(
    Entity,
):
    """A Permission for something."""
