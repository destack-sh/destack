from typing import TYPE_CHECKING

from destack.language.core import (
    Entity,
    Enum,
    EnumType,
    HasName,
    IsDeletable,
    IsOrdered,
    IsOwnable,
    IsVisual,
    Node,
    NodeType,
    Spatial,
    builtin_enum,
    builtin_node,
    property_,
)
from destack.proto import WindowProto

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
    Spatial,
    Entity,
    HasName,
    IsVisual,
    IsOwnable,
    IsOrdered,
    IsDeletable,
    Node[WindowProto],
):
    """
    A Window for someone to interact with a Space via Scenes.
    """

    type: WindowType = property_(30)
