from typing import TYPE_CHECKING

from destack.core import (
    NodeType,
    declare_event,
    declare_property,
)

from .input import InputEvent

if TYPE_CHECKING:
    from destack import Vector2


@declare_event(NodeType.DRAG_EVENT, is_abstract=True)
class DragEvent(InputEvent):
    """A DragEvent is an InputEvent that corresponds to some direct user input with a drag."""

    position: "Vector2" = declare_property(110, is_repr=True, tag=None)


@declare_event(NodeType.DRAG_START_EVENT)
class DragStartEvent(DragEvent):
    """A DragStartEvent is a DragEvent when a drag starts."""

    pass


@declare_event(NodeType.DRAG_END_EVENT)
class DragEndEvent(DragEvent):
    """A DragEndEvent is a DragEvent when a drag ends."""

    pass


@declare_event(NodeType.DRAG_OVER_EVENT)
class DragOverEvent(DragEvent):
    """A DragOverEvent is a DragEvent when a drag is over an element."""

    pass


@declare_event(NodeType.DRAG_ENTER_EVENT)
class DragEnterEvent(DragEvent):
    """A DragEnterEvent is a DragEvent when a drag enters an element."""

    pass


@declare_event(NodeType.DRAG_LEAVE_EVENT)
class DragLeaveEvent(DragEvent):
    """A DragLeaveEvent is a DragEvent when a drag leaves an element."""

    pass


@declare_event(NodeType.DROP_EVENT)
class DropEvent(DragEvent):
    """A DropEvent is a DragEvent when a drag is dropped on an element."""

    pass
