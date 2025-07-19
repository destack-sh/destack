from typing import TYPE_CHECKING

from destack.language.core import (
    NodeType,
    builtin_node,
    builtin_property,
)

from .input import InputEvent

if TYPE_CHECKING:
    from destack.language import Vector2

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.DRAG_EVENT, frozen=True, is_abstract=True)
class DragEvent(InputEvent):
    """A DragEvent is an InputEvent that corresponds to some direct user input with a drag."""

    position: "Vector2" = builtin_property(110, is_repr=True)


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
