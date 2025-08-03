from typing import TYPE_CHECKING

from destack.core import (
    Entity,
    NodeType,
    builtin_entity,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_entity(NodeType.STYLE, is_abstract=True)
class Style(Entity):
    """A Style defines a base visual appearance in some context."""

    pass
