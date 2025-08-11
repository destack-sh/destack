from typing import TYPE_CHECKING, Optional

from destack.core import (
    Event,
    Float32,
    NodeType,
    ReferenceType,
    TagDeclaration,
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
        tag=None,
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
    tags=(
        TagDeclaration(id=120, name="size", description="The sizing of the View."),
        TagDeclaration(id=130, name="visibility", description="The visibility of the View."),
        TagDeclaration(id=140, name="style", description="The style of the View."),
    ),
)
class View(Entity2D):
    """A View is a 2D user interface element."""

    # transform
    origin: Optional["Vector2"] = declare_property(
        115,
        description="The origin of the Entity in 2D space.",
        tag="transform",
    )
    anchor: Optional["Anchor"] = declare_property(
        116,
        description="The anchor of the Entity in 2D space.",
        tag="transform",
    )
    offset: Optional["Offset2"] = declare_property(
        117,
        description="The offset of the Entity in 2D space.",
        tag="transform",
    )

    # size
    width: Optional["Length"] = declare_property(
        120,
        description="The width of the View.",
        tag="size",
    )
    height: Optional["Length"] = declare_property(
        121,
        description="The height of the View.",
        tag="size",
    )
    min_width: Optional["Length"] = declare_property(
        122,
        description="The minimum width of the View.",
        tag="size",
    )
    min_height: Optional["Length"] = declare_property(
        123,
        description="The minimum height of the View.",
        tag="size",
    )
    max_width: Optional["Length"] = declare_property(
        124,
        description="The maximum width of the View.",
        tag="size",
    )
    max_height: Optional["Length"] = declare_property(
        125,
        description="The maximum height of the View.",
        tag="size",
    )

    # visibility
    is_visible: Optional[bool] = declare_property(
        130,
        description="Whether the View is visible.",
        tag="visibility",
    )
    opacity: Optional[Float32] = declare_property(
        131,
        description="The opacity of the View.",
        tag="visibility",
    )

    # style
    fill: Optional["Fill"] = declare_property(
        140,
        description="The fill of the View.",
        tag="style",
    )
    shadow: Optional["Shadow"] = declare_property(
        141,
        description="The shadow of the View.",
        tag="style",
    )
    border: Optional["Border"] = declare_property(
        142,
        description="The border of the View.",
        tag="style",
    )
    radius: Optional["Corner2"] = declare_property(
        143,
        description="The radius of the View.",
        tag="style",
    )
