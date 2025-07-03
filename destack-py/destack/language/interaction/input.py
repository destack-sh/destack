from typing import TYPE_CHECKING

from destack.language.core import (
    Enum,
    EnumType,
    Event,
    NodeType,
    Vector2f,
    builtin_enum,
    builtin_node,
    builtin_property,
)

if TYPE_CHECKING:
    from destack.language import View

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.INPUT_EVENT, frozen=True, is_abstract=True)
class InputEvent[NodeT: View = View](Event[NodeT]):
    """An InputEvent is an Event that corresponds to some direct user input."""

    pass


#
# Pointer Events
#


@builtin_node(NodeType.POINTER_EVENT, frozen=True, is_abstract=True)
class PointerEvent(InputEvent):
    """A PointerEvent is an InputEvent that corresponds to some direct user input with a pointer."""

    position: Vector2f = builtin_property(110)
    pressure: float = builtin_property(111)

    shift_key: bool = builtin_property(120)
    alt_key: bool = builtin_property(121)
    ctrl_key: bool = builtin_property(122)
    meta_key: bool = builtin_property(123)
    accel_key: bool = builtin_property(124)


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


@builtin_node(NodeType.LONG_PRESS_EVENT, frozen=True)
class LongPressEvent(PointerEvent):
    """A LongPressEvent is a PointerEvent when a pointer is pressed down and held for a long time."""

    pass


#
# Mouse Events
#


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


#
# Keyboard Events
#


@builtin_node(NodeType.KEYBOARD_EVENT, frozen=True, is_abstract=True)
class KeyboardEvent(InputEvent):
    """A KeyboardEvent is an InputEvent that corresponds to some direct user input with a keyboard."""

    key: str = builtin_property(110)
    code: str = builtin_property(111)
    repeat: bool = builtin_property(112)

    shift_key: bool = builtin_property(120)
    alt_key: bool = builtin_property(121)
    ctrl_key: bool = builtin_property(122)
    meta_key: bool = builtin_property(123)


@builtin_node(NodeType.KEY_DOWN_EVENT, frozen=True)
class KeyDownEvent(KeyboardEvent):
    """A KeyDownEvent is a KeyboardEvent when a key is pressed down."""

    pass


@builtin_node(NodeType.KEY_UP_EVENT, frozen=True)
class KeyUpEvent(KeyboardEvent):
    """A KeyUpEvent is a KeyboardEvent when a key is released."""

    pass


@builtin_node(NodeType.KEY_PRESS_EVENT, frozen=True)
class KeyPressEvent(KeyboardEvent):
    """A KeyPressEvent is a KeyboardEvent when a key is pressed."""

    pass


#
# Drag Events
#


@builtin_node(NodeType.DRAG_EVENT, frozen=True, is_abstract=True)
class DragEvent(InputEvent):
    """A DragEvent is an InputEvent that corresponds to some direct user input with a drag."""

    position: Vector2f = builtin_property(110)


@builtin_node(NodeType.DRAG_START_EVENT, frozen=True)
class DragStartEvent(DragEvent):
    """A DragStartEvent is a DragEvent when a drag starts."""

    pass


@builtin_node(NodeType.DRAG_END_EVENT, frozen=True)
class DragEndEvent(DragEvent):
    """A DragEndEvent is a DragEvent when a drag ends."""

    pass


@builtin_node(NodeType.DRAG_OVER_EVENT, frozen=True)
class DragOverEvent(DragEvent):
    """A DragOverEvent is a DragEvent when a drag is over an element."""

    pass


@builtin_node(NodeType.DRAG_ENTER_EVENT, frozen=True)
class DragEnterEvent(DragEvent):
    """A DragEnterEvent is a DragEvent when a drag enters an element."""

    pass


@builtin_node(NodeType.DRAG_LEAVE_EVENT, frozen=True)
class DragLeaveEvent(DragEvent):
    """A DragLeaveEvent is a DragEvent when a drag leaves an element."""

    pass


@builtin_node(NodeType.DROP_EVENT, frozen=True)
class DropEvent(DragEvent):
    """A DropEvent is a DragEvent when a drag is dropped on an element."""

    pass


#
# Clipboard Events
#


@builtin_node(NodeType.CLIPBOARD_EVENT, frozen=True, is_abstract=True)
class ClipboardEvent(InputEvent):
    """A ClipboardEvent is an InputEvent that corresponds to some direct user input with a clipboard."""

    pass


@builtin_node(NodeType.COPY_EVENT, frozen=True)
class CopyEvent(ClipboardEvent):
    """A CopyEvent is a ClipboardEvent when a copy is performed."""

    pass


@builtin_node(NodeType.CUT_EVENT, frozen=True)
class CutEvent(ClipboardEvent):
    """A CutEvent is a ClipboardEvent when a cut is performed."""

    pass


@builtin_node(NodeType.PASTE_EVENT, frozen=True)
class PasteEvent(ClipboardEvent):
    """A PasteEvent is a ClipboardEvent when a paste is performed."""

    pass


#
# Focus Events
#


@builtin_node(NodeType.FOCUS_EVENT, frozen=True, is_abstract=True)
class FocusEvent(InputEvent):
    """A FocusEvent is an InputEvent that corresponds to some direct user input with a focus."""

    pass


@builtin_node(NodeType.FOCUS_IN_EVENT, frozen=True)
class FocusInEvent(FocusEvent):
    """A FocusInEvent is a FocusEvent when a focus is gained."""

    pass


@builtin_node(NodeType.FOCUS_OUT_EVENT, frozen=True)
class FocusOutEvent(FocusEvent):
    """A FocusOutEvent is a FocusEvent when a focus is lost."""

    pass
