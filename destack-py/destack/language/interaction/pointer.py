from destack.language.core import (
    NodeType,
    Vector2f,
    builtin_node,
    builtin_property,
)

from .input import InputEvent

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.POINTER_EVENT, frozen=True, is_abstract=True)
class PointerEvent(InputEvent):
    """A PointerEvent is an InputEvent that corresponds to some direct user input with a pointer."""

    position: Vector2f = builtin_property(110, is_repr=True)
    pressure: float | None = builtin_property(111, is_repr=True)

    shift_key: bool = builtin_property(120)
    alt_key: bool = builtin_property(121)
    ctrl_key: bool = builtin_property(122)
    meta_key: bool = builtin_property(123)


@builtin_node(NodeType.POINTER_DOWN_EVENT, frozen=True)
class PointerDownEvent(PointerEvent):
    """A PointerDownEvent is a PointerEvent when a pointer is pressed down."""

    pass


@builtin_node(NodeType.POINTER_UP_EVENT, frozen=True)
class PointerUpEvent(PointerEvent):
    """A PointerUpEvent is a PointerEvent when a pointer is released."""

    pass


@builtin_node(NodeType.POINTER_MOVE_EVENT, frozen=True)
class PointerMoveEvent(PointerEvent):
    """A PointerMoveEvent is a PointerEvent when a pointer is moved."""

    pass


@builtin_node(NodeType.POINTER_ENTER_EVENT, frozen=True)
class PointerEnterEvent(PointerEvent):
    """A PointerEnterEvent is a PointerEvent when a pointer enters an element."""

    pass


@builtin_node(NodeType.POINTER_OVER_EVENT, frozen=True)
class PointerOverEvent(PointerEvent):
    """A PointerOverEvent is a PointerEvent when a pointer is over an element."""

    pass


@builtin_node(NodeType.POINTER_LEAVE_EVENT, frozen=True)
class PointerLeaveEvent(PointerEvent):
    """A PointerLeaveEvent is a PointerEvent when a pointer leaves an element."""

    pass


@builtin_node(NodeType.POINTER_LONG_PRESS_EVENT, frozen=True)
class PointerLongPressEvent(PointerEvent):
    """A PointerLongPressEvent is a PointerEvent when a pointer is pressed down and held for a long time."""

    pass
