from typing import TYPE_CHECKING

from destack.language.core import (
    Entity,
    IsExtensible,
    NodeType,
    builtin_node,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.STYLE, is_abstract=True)
class Style(
    IsExtensible,
    Entity,
):
    """A Style defines a base visual appearance in some context."""

    pass
