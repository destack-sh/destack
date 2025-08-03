from typing import TYPE_CHECKING

from destack.core import (
    Enum,
    EnumType,
    NodeType,
    builtin_enum,
    builtin_event,
    builtin_property,
)

from .pointer import PointerEvent

if TYPE_CHECKING:
    from destack import Vector2

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.MOUSE_BUTTON)
class MouseButton(Enum):
    """A MouseButton is a button on a mouse."""

    LEFT = 1
    RIGHT = 2
    MIDDLE = 3


@builtin_event(NodeType.MOUSE_EVENT, is_abstract=True)
class MouseEvent(PointerEvent):
    """A MouseEvent is a PointerEvent that corresponds to some direct user input with a mouse."""

    button: MouseButton = builtin_property(130, is_repr=True)


@builtin_event(NodeType.CLICK_EVENT, is_abstract=True)
class ClickEvent(MouseEvent):
    """A ClickEvent is an InputEvent that corresponds to some direct user input with a click (left, right, middle)."""

    pass


@builtin_event(NodeType.SINGLE_CLICK_EVENT)
class SingleClickEvent(ClickEvent):
    """A SingleClickEvent is a ClickEvent when a pointer is clicked once."""

    pass


@builtin_event(NodeType.DOUBLE_CLICK_EVENT)
class DoubleClickEvent(ClickEvent):
    """A DoubleClickEvent is a ClickEvent when a pointer is clicked twice in a short time."""

    pass


@builtin_event(NodeType.TRIPLE_CLICK_EVENT)
class TripleClickEvent(ClickEvent):
    """A TripleClickEvent is a ClickEvent when a pointer is clicked three times in a short time."""

    pass


@builtin_event(NodeType.WHEEL_EVENT)
class WheelEvent(MouseEvent):
    """A WheelEvent is a MouseEvent when a wheel is scrolled."""

    delta: "Vector2" = builtin_property(140)
