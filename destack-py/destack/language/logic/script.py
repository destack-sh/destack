from typing import TYPE_CHECKING

from destack.language.core import (
    Entity,
    IsOrdered,
    NodeType,
    builtin_node,
    builtin_property,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.SCRIPT)
class Script(
    IsOrdered,
    Entity,
):
    """A Script."""

    code: str = builtin_property(110)
    # type, language, code, ...
