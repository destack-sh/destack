from typing import TYPE_CHECKING

from destack.core import (
    Float32,
    NodeType,
    declare_event,
    declare_property,
)

from .input import InputEvent

if TYPE_CHECKING:
    from destack import Vector2


@declare_event(NodeType.POINTER_EVENT, is_abstract=True)
class PointerEvent(InputEvent):
    """A PointerEvent is an InputEvent that corresponds to some direct user input with a pointer."""

    position: "Vector2" = declare_property(110, is_repr=True)
    pressure: Float32 | None = declare_property(111, is_repr=True)

    shift_key: bool = declare_property(120)
    alt_key: bool = declare_property(121)
    ctrl_key: bool = declare_property(122)
    meta_key: bool = declare_property(123)


@declare_event(NodeType.POINTER_DOWN_EVENT)
class PointerDownEvent(PointerEvent):
    """A PointerDownEvent is a PointerEvent when a pointer is pressed down."""

    pass


@declare_event(NodeType.POINTER_UP_EVENT)
class PointerUpEvent(PointerEvent):
    """A PointerUpEvent is a PointerEvent when a pointer is released."""

    pass


@declare_event(NodeType.POINTER_MOVE_EVENT)
class PointerMoveEvent(PointerEvent):
    """A PointerMoveEvent is a PointerEvent when a pointer is moved."""

    pass


@declare_event(NodeType.POINTER_ENTER_EVENT)
class PointerEnterEvent(PointerEvent):
    """A PointerEnterEvent is a PointerEvent when a pointer enters an element."""

    pass


@declare_event(NodeType.POINTER_OVER_EVENT)
class PointerOverEvent(PointerEvent):
    """A PointerOverEvent is a PointerEvent when a pointer is over an element."""

    pass


@declare_event(NodeType.POINTER_LEAVE_EVENT)
class PointerLeaveEvent(PointerEvent):
    """A PointerLeaveEvent is a PointerEvent when a pointer leaves an element."""

    pass


@declare_event(NodeType.POINTER_LONG_PRESS_EVENT)
class PointerLongPressEvent(PointerEvent):
    """A PointerLongPressEvent is a PointerEvent when a pointer is pressed down and held for a long time."""

    pass
