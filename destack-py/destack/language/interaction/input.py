from typing import TYPE_CHECKING

from destack.language.core import (
    Enum,
    EnumType,
    Event,
    Node,
    NodeType,
    TraitType,
    Vector2,
    builtin_enum,
    builtin_node,
    builtin_trait,
    property_,
)
from destack.proto import (
    CopyEventProto,
    CutEventProto,
    DoubleClickEventProto,
    DragEndEventProto,
    DragEnterEventProto,
    DragLeaveEventProto,
    DragOverEventProto,
    DragStartEventProto,
    DropEventProto,
    FocusInEventProto,
    FocusOutEventProto,
    KeyDownEventProto,
    KeyPressEventProto,
    KeyUpEventProto,
    LeftClickEventProto,
    LongPressEventProto,
    MiddleClickEventProto,
    PasteEventProto,
    PointerDownEventProto,
    PointerEnterEventProto,
    PointerLeaveEventProto,
    PointerMoveEventProto,
    PointerOverEventProto,
    PointerUpEventProto,
    RightClickEventProto,
    WheelEventProto,
)

if TYPE_CHECKING:
    from destack.language import View

# pyright: reportIncompatibleVariableOverride=false


@builtin_trait(TraitType.INPUT_EVENT)
class InputEvent[NodeT: View = View](Event[NodeT]):
    """An InputEvent is an Event that corresponds to some direct user input."""

    pass


#
# Pointer Events
#


@builtin_trait(TraitType.POINTER_EVENT)
class PointerEvent(InputEvent):
    """A PointerEvent is an InputEvent that corresponds to some direct user input with a pointer."""

    position: Vector2 = property_(50)
    pressure: float = property_(51)

    shift_key: bool = property_(80)
    alt_key: bool = property_(81)
    ctrl_key: bool = property_(82)
    meta_key: bool = property_(83)
    accel_key: bool = property_(84)


@builtin_node(NodeType.POINTER_DOWN_EVENT)
class PointerDownEvent(PointerEvent, Node[PointerDownEventProto]):
    """A PointerDownEvent is a PointerEvent when a pointer is pressed down."""

    pass


@builtin_node(NodeType.POINTER_UP_EVENT)
class PointerUpEvent(PointerEvent, Node[PointerUpEventProto]):
    """A PointerUpEvent is a PointerEvent when a pointer is released."""

    pass


@builtin_node(NodeType.POINTER_MOVE_EVENT)
class PointerMoveEvent(PointerEvent, Node[PointerMoveEventProto]):
    """A PointerMoveEvent is a PointerEvent when a pointer is moved."""

    pass


@builtin_node(NodeType.POINTER_ENTER_EVENT)
class PointerEnterEvent(PointerEvent, Node[PointerEnterEventProto]):
    """A PointerEnterEvent is a PointerEvent when a pointer enters an element."""

    pass


@builtin_node(NodeType.POINTER_OVER_EVENT)
class PointerOverEvent(PointerEvent, Node[PointerOverEventProto]):
    """A PointerOverEvent is a PointerEvent when a pointer is over an element."""

    pass


@builtin_node(NodeType.POINTER_LEAVE_EVENT)
class PointerLeaveEvent(PointerEvent, Node[PointerLeaveEventProto]):
    """A PointerLeaveEvent is a PointerEvent when a pointer leaves an element."""

    pass


@builtin_node(NodeType.LONG_PRESS_EVENT)
class LongPressEvent(PointerEvent, Node[LongPressEventProto]):
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


@builtin_trait(TraitType.MOUSE_EVENT)
class MouseEvent(PointerEvent):
    """A MouseEvent is a PointerEvent that corresponds to some direct user input with a mouse."""

    button: MouseButton = property_(60)


@builtin_trait(TraitType.CLICK_EVENT)
class ClickEvent(MouseEvent):
    """A ClickEvent is an InputEvent that corresponds to some direct user input with a click (left, right, middle)."""

    pass


@builtin_node(NodeType.LEFT_CLICK_EVENT)
class LeftClickEvent(ClickEvent, Node[LeftClickEventProto]):
    """A LeftClickEvent is a ClickEvent when a pointer is clicked with the left button."""

    pass


@builtin_node(NodeType.RIGHT_CLICK_EVENT)
class RightClickEvent(ClickEvent, Node[RightClickEventProto]):
    """A RightClickEvent is a ClickEvent when a pointer is clicked with the right button."""

    pass


@builtin_node(NodeType.MIDDLE_CLICK_EVENT)
class MiddleClickEvent(ClickEvent, Node[MiddleClickEventProto]):
    """A MiddleClickEvent is a ClickEvent when a pointer is clicked with the middle button."""

    pass


@builtin_node(NodeType.DOUBLE_CLICK_EVENT)
class DoubleClickEvent(ClickEvent, Node[DoubleClickEventProto]):
    """A DoubleClickEvent is a ClickEvent when a pointer is clicked twice in a short time."""

    pass


@builtin_node(NodeType.WHEEL_EVENT)
class WheelEvent(MouseEvent, Node[WheelEventProto]):
    """A WheelEvent is a MouseEvent when a wheel is scrolled."""

    delta: Vector2 = property_(70)


#
# Keyboard Events
#


@builtin_trait(TraitType.KEYBOARD_EVENT)
class KeyboardEvent(InputEvent):
    """A KeyboardEvent is an InputEvent that corresponds to some direct user input with a keyboard."""

    key: str = property_(50)
    code: str = property_(51)
    repeat: bool = property_(52)

    shift_key: bool = property_(80)
    alt_key: bool = property_(81)
    ctrl_key: bool = property_(82)
    meta_key: bool = property_(83)


@builtin_node(NodeType.KEY_DOWN_EVENT)
class KeyDownEvent(KeyboardEvent, Node[KeyDownEventProto]):
    """A KeyDownEvent is a KeyboardEvent when a key is pressed down."""

    pass


@builtin_node(NodeType.KEY_UP_EVENT)
class KeyUpEvent(KeyboardEvent, Node[KeyUpEventProto]):
    """A KeyUpEvent is a KeyboardEvent when a key is released."""

    pass


@builtin_node(NodeType.KEY_PRESS_EVENT)
class KeyPressEvent(KeyboardEvent, Node[KeyPressEventProto]):
    """A KeyPressEvent is a KeyboardEvent when a key is pressed."""

    pass


#
# Drag Events
#


@builtin_trait(TraitType.DRAG_EVENT)
class DragEvent(InputEvent):
    """A DragEvent is an InputEvent that corresponds to some direct user input with a drag."""

    position: Vector2 = property_(50)


@builtin_node(NodeType.DRAG_START_EVENT)
class DragStartEvent(DragEvent, Node[DragStartEventProto]):
    """A DragStartEvent is a DragEvent when a drag starts."""

    pass


@builtin_node(NodeType.DRAG_END_EVENT)
class DragEndEvent(DragEvent, Node[DragEndEventProto]):
    """A DragEndEvent is a DragEvent when a drag ends."""

    pass


@builtin_node(NodeType.DRAG_OVER_EVENT)
class DragOverEvent(DragEvent, Node[DragOverEventProto]):
    """A DragOverEvent is a DragEvent when a drag is over an element."""

    pass


@builtin_node(NodeType.DRAG_ENTER_EVENT)
class DragEnterEvent(DragEvent, Node[DragEnterEventProto]):
    """A DragEnterEvent is a DragEvent when a drag enters an element."""

    pass


@builtin_node(NodeType.DRAG_LEAVE_EVENT)
class DragLeaveEvent(DragEvent, Node[DragLeaveEventProto]):
    """A DragLeaveEvent is a DragEvent when a drag leaves an element."""

    pass


@builtin_node(NodeType.DROP_EVENT)
class DropEvent(DragEvent, Node[DropEventProto]):
    """A DropEvent is a DragEvent when a drag is dropped on an element."""

    pass


#
# Clipboard Events
#


@builtin_trait(TraitType.CLIPBOARD_EVENT)
class ClipboardEvent(InputEvent):
    """A ClipboardEvent is an InputEvent that corresponds to some direct user input with a clipboard."""

    pass


@builtin_node(NodeType.COPY_EVENT)
class CopyEvent(ClipboardEvent, Node[CopyEventProto]):
    """A CopyEvent is a ClipboardEvent when a copy is performed."""

    pass


@builtin_node(NodeType.CUT_EVENT)
class CutEvent(ClipboardEvent, Node[CutEventProto]):
    """A CutEvent is a ClipboardEvent when a cut is performed."""

    pass


@builtin_node(NodeType.PASTE_EVENT)
class PasteEvent(ClipboardEvent, Node[PasteEventProto]):
    """A PasteEvent is a ClipboardEvent when a paste is performed."""

    pass


#
# Focus Events
#


@builtin_trait(TraitType.FOCUS_EVENT)
class FocusEvent(InputEvent):
    """A FocusEvent is an InputEvent that corresponds to some direct user input with a focus."""

    pass


@builtin_node(NodeType.FOCUS_IN_EVENT)
class FocusInEvent(FocusEvent, Node[FocusInEventProto]):
    """A FocusInEvent is a FocusEvent when a focus is gained."""


@builtin_node(NodeType.FOCUS_OUT_EVENT)
class FocusOutEvent(FocusEvent, Node[FocusOutEventProto]):
    """A FocusOutEvent is a FocusEvent when a focus is lost."""

    pass
