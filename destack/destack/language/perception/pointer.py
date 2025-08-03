from typing import TYPE_CHECKING

from destack.language.core import (
    Float32,
    NodeType,
    builtin_event,
    builtin_property,
)

from .input import InputEvent

if TYPE_CHECKING:
    from destack.language import Vector2

# pyright: reportIncompatibleVariableOverride=false


@builtin_event(NodeType.POINTER_EVENT, is_abstract=True)
class PointerEvent(InputEvent):
    """A PointerEvent is an InputEvent that corresponds to some direct user input with a pointer."""

    position: "Vector2" = builtin_property(110, is_repr=True)
    pressure: Float32 | None = builtin_property(111, is_repr=True)

    shift_key: bool = builtin_property(120)
    alt_key: bool = builtin_property(121)
    ctrl_key: bool = builtin_property(122)
    meta_key: bool = builtin_property(123)


@builtin_event(NodeType.POINTER_DOWN_EVENT)
class PointerDownEvent(PointerEvent):
    """A PointerDownEvent is a PointerEvent when a pointer is pressed down."""

    pass


@builtin_event(NodeType.POINTER_UP_EVENT)
class PointerUpEvent(PointerEvent):
    """A PointerUpEvent is a PointerEvent when a pointer is released."""

    pass


@builtin_event(NodeType.POINTER_MOVE_EVENT)
class PointerMoveEvent(PointerEvent):
    """A PointerMoveEvent is a PointerEvent when a pointer is moved."""

    pass


@builtin_event(NodeType.POINTER_ENTER_EVENT)
class PointerEnterEvent(PointerEvent):
    """A PointerEnterEvent is a PointerEvent when a pointer enters an element."""

    pass


@builtin_event(NodeType.POINTER_OVER_EVENT)
class PointerOverEvent(PointerEvent):
    """A PointerOverEvent is a PointerEvent when a pointer is over an element."""

    pass


@builtin_event(NodeType.POINTER_LEAVE_EVENT)
class PointerLeaveEvent(PointerEvent):
    """A PointerLeaveEvent is a PointerEvent when a pointer leaves an element."""

    pass


@builtin_event(NodeType.POINTER_LONG_PRESS_EVENT)
class PointerLongPressEvent(PointerEvent):
    """A PointerLongPressEvent is a PointerEvent when a pointer is pressed down and held for a long time."""

    pass
