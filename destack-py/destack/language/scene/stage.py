from typing import TYPE_CHECKING

from destack.language.core import (
    Entity,
    IsOrdered,
    IsOwnable,
    IsScriptable,
    NodeType,
    builtin_node,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.STAGE)
class Stage(
    IsOwnable,
    IsOrdered,
    IsScriptable,
    Entity,
):
    """
    A Stage for someone to interact with a Space.
    """

    pass
