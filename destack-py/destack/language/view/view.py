from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    Event,
    IsExtensible,
    NodeType,
    builtin_node,
    builtin_property,
)

if TYPE_CHECKING:
    from destack.language import (
        Axis3,
        Border,
        Corners,
        Dimension,
        Fill,
        Position,
        Shadow,
        Vector2,
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
class View(
    IsExtensible,
    Entity,
):
    """A View is a graphical interface."""

    # transform
    position: Optional["Position"] = builtin_property(110)
    scale: Optional[float] = builtin_property(111)
    rotation: Optional["Axis3"] = builtin_property(112)
    skew: Optional["Vector2"] = builtin_property(113)

    # size
    width: Optional["Dimension"] = builtin_property(120)
    height: Optional["Dimension"] = builtin_property(121)
    min_width: Optional["Dimension"] = builtin_property(122)
    min_height: Optional["Dimension"] = builtin_property(123)
    max_width: Optional["Dimension"] = builtin_property(124)
    max_height: Optional["Dimension"] = builtin_property(125)

    # appearance
    is_visible: Optional[bool] = builtin_property(130)
    opacity: Optional[float] = builtin_property(131)
    fill: Optional["Fill"] = builtin_property(132)
    shadow: Optional["Shadow"] = builtin_property(136)
    border: Optional["Border"] = builtin_property(137)
    radius: Optional["Corners"] = builtin_property(138)
