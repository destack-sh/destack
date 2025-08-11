from typing import TYPE_CHECKING, Optional

from destack.core import (
    Event,
    Float32,
    NodeType,
    ReferenceType,
    declare_entity,
    declare_event,
    declare_property,
)
from destack.simulation.geometry import Entity2D

if TYPE_CHECKING:
    from destack import (
        Anchor,
        Border,
        Corner2,
        Fill,
        Length,
        Offset2,
        Shadow,
        Vector2,
        View,
    )


@declare_event(NodeType.VIEW_EVENT, is_abstract=True)
class ViewEvent(Event):
    """A Event regarding a View."""

    view: "View" = declare_property(
        101,
        reference_type=ReferenceType.LOCATION,
    )


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

    # transform
    origin: Optional["Vector2"] = declare_property(
        115,
        tags=("transform",),
        description="The origin of the Entity in 2D space.",
    )
    anchor: Optional["Anchor"] = declare_property(
        116,
        tags=("transform",),
        description="The anchor of the Entity in 2D space.",
    )
    offset: Optional["Offset2"] = declare_property(
        117,
        tags=("transform",),
        description="The offset of the Entity in 2D space.",
    )

    # dimensions
    width: Optional["Length"] = declare_property(
        120,
        tags=("dimensions",),
        description="The width of the View.",
    )
    height: Optional["Length"] = declare_property(
        121,
        tags=("dimensions",),
        description="The height of the View.",
    )
    min_width: Optional["Length"] = declare_property(
        122,
        tags=("dimensions",),
        description="The minimum width of the View.",
    )
    min_height: Optional["Length"] = declare_property(
        123,
        tags=("dimensions",),
        description="The minimum height of the View.",
    )
    max_width: Optional["Length"] = declare_property(
        124,
        tags=("dimensions",),
        description="The maximum width of the View.",
    )
    max_height: Optional["Length"] = declare_property(
        125,
        tags=("dimensions",),
        description="The maximum height of the View.",
    )

    # visibility
    is_visible: Optional[bool] = declare_property(
        130,
        tags=("visibility",),
        description="Whether the View is visible.",
    )
    opacity: Optional[Float32] = declare_property(
        131,
        tags=("visibility",),
        description="The opacity of the View.",
    )

    # style
    fill: Optional["Fill"] = declare_property(
        140,
        tags=("style",),
        description="The fill of the View.",
    )
    shadow: Optional["Shadow"] = declare_property(
        141,
        tags=("style",),
        description="The shadow of the View.",
    )
    border: Optional["Border"] = declare_property(
        142,
        tags=("style",),
        description="The border of the View.",
    )
    radius: Optional["Corner2"] = declare_property(
        143,
        tags=("style",),
        description="The radius of the View.",
    )
