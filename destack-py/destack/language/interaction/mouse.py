from destack.language.core import (
    Enum,
    EnumType,
    NodeType,
    Vector2f,
    builtin_enum,
    builtin_node,
    builtin_property,
)

from .pointer import PointerEvent

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.MOUSE_BUTTON)
class MouseButton(Enum):
    """A MouseButton is a button on a mouse."""

    LEFT = 1
    RIGHT = 2
    MIDDLE = 3


@builtin_node(NodeType.MOUSE_EVENT, frozen=True, is_abstract=True)
class MouseEvent(PointerEvent):
    """A MouseEvent is a PointerEvent that corresponds to some direct user input with a mouse."""

    button: MouseButton = builtin_property(130)


@builtin_node(NodeType.CLICK_EVENT, frozen=True, is_abstract=True)
class ClickEvent(MouseEvent):
    """A ClickEvent is an InputEvent that corresponds to some direct user input with a click (left, right, middle)."""

    pass


@builtin_node(NodeType.LEFT_CLICK_EVENT, frozen=True)
class LeftClickEvent(ClickEvent):
    """A LeftClickEvent is a ClickEvent when a pointer is clicked with the left button."""

    pass


@builtin_node(NodeType.RIGHT_CLICK_EVENT, frozen=True)
class RightClickEvent(ClickEvent):
    """A RightClickEvent is a ClickEvent when a pointer is clicked with the right button."""

    pass


@builtin_node(NodeType.MIDDLE_CLICK_EVENT, frozen=True)
class MiddleClickEvent(ClickEvent):
    """A MiddleClickEvent is a ClickEvent when a pointer is clicked with the middle button."""

    pass


@builtin_node(NodeType.DOUBLE_CLICK_EVENT, frozen=True)
class DoubleClickEvent(ClickEvent):
    """A DoubleClickEvent is a ClickEvent when a pointer is clicked twice in a short time."""

    pass


@builtin_node(NodeType.WHEEL_EVENT, frozen=True)
class WheelEvent(MouseEvent):
    """A WheelEvent is a MouseEvent when a wheel is scrolled."""

    delta: Vector2f = builtin_property(140)
