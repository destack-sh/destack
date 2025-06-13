from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    Enum,
    EnumType,
    IsDeletable,
    IsOrdered,
    IsOwnable,
    IsTemplatable,
    IsVisual,
    Node,
    NodeType,
    Selection,
    Spatial,
    StringFormat,
    builtin_enum,
    builtin_node,
    property_,
)
from destack.pb2 import WindowData

if TYPE_CHECKING:
    from destack.language import Thread

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
    IsVisual,
    IsOwnable,
    IsTemplatable,
    IsOrdered,
    IsDeletable,
    Node[WindowData],
):
    """
    A Window for someone to interact with a Space.
    """

    type: WindowType = property_(30)
    name: str | None = property_(31, format=StringFormat.NAME)

    selection: Optional[Selection] = property_(
        70,
        default=None,
        description="The current selection of the Window.",
    )
    focus: Optional[Node] = property_(71, default=None, description="The current main focus.")
    inspection: Optional[Node] = property_(
        72, default=None, description="The current inspected Node."
    )
    container: Optional[Node] = property_(
        73, default=None, description="The current 'root' container Node."
    )
    thread: Optional["Thread"] = property_(75, default=None, description="The current Thread.")
