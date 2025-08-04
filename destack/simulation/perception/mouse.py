from typing import TYPE_CHECKING

from destack.core import (
    EnumType,
    NodeType,
    OptionEnum,
    declare_enum,
    declare_event,
    declare_option,
    declare_property,
)

from .pointer import PointerEvent

if TYPE_CHECKING:
    from destack import Vector2


@declare_enum(EnumType.MOUSE_BUTTON)
class MouseButton(OptionEnum):
    """A MouseButton is a button on a mouse."""

    LEFT = declare_option(1, description="Left button")
    RIGHT = declare_option(2, description="Right button")
    MIDDLE = declare_option(3, description="Middle button")


@declare_event(NodeType.MOUSE_EVENT, is_abstract=True)
class MouseEvent(PointerEvent):
    """A MouseEvent is a PointerEvent that corresponds to some direct user input with a mouse."""

    button: MouseButton = declare_property(130, is_repr=True)


@declare_event(NodeType.CLICK_EVENT, is_abstract=True)
class ClickEvent(MouseEvent):
    """A ClickEvent is an InputEvent that corresponds to some direct user input with a click (left, right, middle)."""

    pass


@declare_event(NodeType.SINGLE_CLICK_EVENT)
class SingleClickEvent(ClickEvent):
    """A SingleClickEvent is a ClickEvent when a pointer is clicked once."""

    pass


@declare_event(NodeType.DOUBLE_CLICK_EVENT)
class DoubleClickEvent(ClickEvent):
    """A DoubleClickEvent is a ClickEvent when a pointer is clicked twice in a short time."""

    pass


@declare_event(NodeType.TRIPLE_CLICK_EVENT)
class TripleClickEvent(ClickEvent):
    """A TripleClickEvent is a ClickEvent when a pointer is clicked three times in a short time."""

    pass


@declare_event(NodeType.WHEEL_EVENT)
class WheelEvent(MouseEvent):
    """A WheelEvent is a MouseEvent when a wheel is scrolled."""

    delta: "Vector2" = declare_property(140)
