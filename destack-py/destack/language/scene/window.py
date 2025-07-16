from typing import TYPE_CHECKING

from destack.language.core import (
    Entity,
    Enum,
    EnumType,
    IsOrdered,
    IsOwnable,
    NodeType,
    builtin_enum,
    builtin_node,
    builtin_property,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.WINDOW_TYPE)
class WindowType(Enum):
    BROWSER = 10
    DESKTOP = 20
    MOBILE = 30


@builtin_node(NodeType.WINDOW)
class Window(
    IsOwnable,
    IsOrdered,
    Entity,
):
    """
    A Window for someone to interact with Destack (in a Space).
    """

    type: WindowType = builtin_property(100, is_repr=True)
