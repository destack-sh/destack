from typing import TYPE_CHECKING, Optional

from destack.core import (
    Event,
    Float32,
    NodeType,
    declare_entity,
    declare_event,
    declare_property,
)
from destack.simulation.geometry import Entity2D

if TYPE_CHECKING:
    from destack import (
        Border,
        Corner2,
        Fill,
        Length,
        Shadow,
        View,
    )

# pyright: reportIncompatibleVariableOverride=false


@declare_event(NodeType.VIEW_EVENT, is_abstract=True)
class ViewEvent(Event):
    """A Event regarding a View."""

    view: "View" = declare_property(101)


@declare_entity(
    NodeType.VIEW,
    is_abstract=True,
    event_types=(
        NodeType.VIEW_EVENT,
        NodeType.POINTER_EVENT,
        NodeType.MOUSE_EVENT,
        NodeType.KEY_EVENT,
        NodeType.DRAG_EVENT,
        NodeType.CLIPBOARD_EVENT,
        NodeType.FOCUS_EVENT,
    ),
    expected_ancestor_types=(
        NodeType.SCENE,
        NodeType.VIEW,
    ),
)
class View(Entity2D):
    """A View is a 2D user interface element."""

    # size
    width: Optional["Length"] = declare_property(120, tags=("size",))
    height: Optional["Length"] = declare_property(121, tags=("size",))
    min_width: Optional["Length"] = declare_property(122, tags=("size",))
    min_height: Optional["Length"] = declare_property(123, tags=("size",))
    max_width: Optional["Length"] = declare_property(124, tags=("size",))
    max_height: Optional["Length"] = declare_property(125, tags=("size",))

    # visibility
    is_visible: Optional[bool] = declare_property(130, tags=("visibility",))
    opacity: Optional[Float32] = declare_property(131, tags=("visibility",))

    # style
    fill: Optional["Fill"] = declare_property(
        140,
        tags=("style",),
    )
    shadow: Optional["Shadow"] = declare_property(
        141,
        tags=("style",),
    )
    border: Optional["Border"] = declare_property(
        142,
        tags=("style",),
    )
    radius: Optional["Corner2"] = declare_property(
        143,
        tags=("style",),
    )
