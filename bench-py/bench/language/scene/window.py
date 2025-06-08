from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    IsDeletable,
    IsInPackage,
    IsOrdered,
    IsOwnable,
    IsTemplatable,
    IsTracked,
    Node,
    NodeType,
    Selection,
    StringFormat,
    enum_,
    node_,
    property_,
    property_parent_,
)
from bench.pb2 import WindowData

if TYPE_CHECKING:
    from bench.language import Package, Page, Thread

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.WINDOW_TYPE)
class WindowType(BuiltinEnum):
    BROWSER = 10
    DESKTOP = 20
    MOBILE = 30


@node_(NodeType.WINDOW)
class Window(
    IsOwnable,
    IsTemplatable,
    IsOrdered,
    IsInPackage,
    IsDeletable,
    IsTracked,
    Node[WindowData],
):
    """
    A Window for a User to interact with a Space.
    Windows to all Spaces are stored in the owning User's Space.
    """

    parent: Optional["Package"] = property_parent_(node_is_customizable=False)

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
    page: Optional["Page"] = property_(74, default=None, description="The current Page.")
    thread: Optional["Thread"] = property_(75, default=None, description="The current Thread.")
