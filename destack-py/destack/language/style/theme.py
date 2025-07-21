from typing import TYPE_CHECKING

from destack.language.core import (
    Entity,
    NodeType,
    builtin_node,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(
    NodeType.THEME,
    is_extensible=True,
)
class Theme(
    Entity,
):
    """A Theme with common Styles."""
