from typing import TYPE_CHECKING

from ..builtin import (
    Entity,
    NodeType,
    builtin_node,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.PERMISSION)
class Permission(
    Entity,
):
    """A Permission for something."""
