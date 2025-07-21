from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity2D,
    Event,
    NodeType,
    builtin_node,
    builtin_property,
)

if TYPE_CHECKING:
    from destack.language import (
        Border,
        Corner2,
        Fill,
        Length,
        Shadow,
        View,
    )

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.VIEW_EVENT, frozen=True, is_abstract=True)
class ViewEvent(Event["View"]):
    """A Event regarding a View."""

    node: "View" = builtin_property(101)


@builtin_node(
    NodeType.VIEW,
    is_abstract=True,
    is_extensible=True,
    event_types=(
        NodeType.VIEW_EVENT,
        NodeType.POINTER_EVENT,
        NodeType.MOUSE_EVENT,
        NodeType.KEY_EVENT,
        NodeType.DRAG_EVENT,
        NodeType.CLIPBOARD_EVENT,
        NodeType.FOCUS_EVENT,
    ),
    expected_ancestor_types=(
        NodeType.SCENE,
        NodeType.VIEW,
    ),
)
class View(Entity2D):
    """A View is a 2D user interface element."""

    # size
    width: Optional["Length"] = builtin_property(120)
    height: Optional["Length"] = builtin_property(121)
    min_width: Optional["Length"] = builtin_property(122)
    min_height: Optional["Length"] = builtin_property(123)
    max_width: Optional["Length"] = builtin_property(124)
    max_height: Optional["Length"] = builtin_property(125)

    # appearance
    is_visible: Optional[bool] = builtin_property(130)
    opacity: Optional[float] = builtin_property(131)

    fill: Optional["Fill"] = builtin_property(140)
    shadow: Optional["Shadow"] = builtin_property(141)
    border: Optional["Border"] = builtin_property(142)
    radius: Optional["Corner2"] = builtin_property(143)
