from typing import TYPE_CHECKING

from destack.language.core import (
    Entity,
    NodeType,
    builtin_entity,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_entity(NodeType.THEME)
class Theme(
    Entity,
):
    """A Theme with common Styles."""

    pass
