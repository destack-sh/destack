from typing import TYPE_CHECKING, Optional

from destack.core import (
    Float32,
    NodeType,
    TagDeclaration,
    declare_entity,
    declare_property,
)
from destack.simulation.physics import RigidBody2D

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
        View2D,
    )


@declare_entity(
    NodeType.VIEW2D,
    is_abstract=True,
    event_types=(
        NodeType.POINTER_EVENT,
        NodeType.MOUSE_EVENT,
        NodeType.KEY_EVENT,
        NodeType.DRAG_EVENT,
        NodeType.CLIPBOARD_EVENT,
        NodeType.FOCUS_EVENT,
    ),
    expected_ancestor_types=(NodeType.SCENE,),
    tags=(
        TagDeclaration(id=130, name="sizing", description="The sizing of the View."),
        TagDeclaration(id=140, name="visibility", description="The visibility of the View."),
        TagDeclaration(id=150, name="style", description="The style of the View."),
    ),
)
class View2D(RigidBody2D):
    """
    A View2D is a 2D interface element.
    Views add dynamic positioning, sizing and styling to RigidBody2D.
    """

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
        150,
        description="The width of the View.",
        tag="sizing",
    )
    height: Optional["Length"] = declare_property(
        151,
        description="The height of the View.",
        tag="sizing",
    )
    min_width: Optional["Length"] = declare_property(
        152,
        description="The minimum width of the View.",
        tag="sizing",
    )
    min_height: Optional["Length"] = declare_property(
        153,
        description="The minimum height of the View.",
        tag="sizing",
    )
    max_width: Optional["Length"] = declare_property(
        154,
        description="The maximum width of the View.",
        tag="sizing",
    )
    max_height: Optional["Length"] = declare_property(
        155,
        description="The maximum height of the View.",
        tag="sizing",
    )

    # visibility
    is_visible: Optional[bool] = declare_property(
        160,
        description="Whether the View is visible.",
        tag="visibility",
    )
    opacity: Optional[Float32] = declare_property(
        161,
        description="The opacity of the View.",
        tag="visibility",
    )

    # style
    fill: Optional["Fill"] = declare_property(
        170,
        description="The fill of the View.",
        tag="style",
    )
    shadow: Optional["Shadow"] = declare_property(
        171,
        description="The shadow of the View.",
        tag="style",
    )
    border: Optional["Border"] = declare_property(
        172,
        description="The border of the View.",
        tag="style",
    )
    radius: Optional["Corner2"] = declare_property(
        173,
        description="The radius of the View.",
        tag="style",
    )
