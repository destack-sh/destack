from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity2D,
    Event,
    Float32,
    NodeType,
    builtin_entity,
    builtin_event,
    builtin_property,
)

if TYPE_CHECKING:
    from destack.language import (
        Border,
        Corner2,
        Fill,
        Length,
        Shadow,
        View,
    )

# pyright: reportIncompatibleVariableOverride=false


@builtin_event(NodeType.VIEW_EVENT, is_abstract=True)
class ViewEvent(Event["View"]):
    """A Event regarding a View."""

    node: "View" = builtin_property(101)


@builtin_entity(
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
    width: Optional["Length"] = builtin_property(120, tags=("size",))
    height: Optional["Length"] = builtin_property(121, tags=("size",))
    min_width: Optional["Length"] = builtin_property(122, tags=("size",))
    min_height: Optional["Length"] = builtin_property(123, tags=("size",))
    max_width: Optional["Length"] = builtin_property(124, tags=("size",))
    max_height: Optional["Length"] = builtin_property(125, tags=("size",))

    # visibility
    is_visible: Optional[bool] = builtin_property(130, tags=("visibility",))
    opacity: Optional[Float32] = builtin_property(131, tags=("visibility",))

    # style
    fill: Optional["Fill"] = builtin_property(
        140,
        tags=("style",),
    )
    shadow: Optional["Shadow"] = builtin_property(
        141,
        tags=("style",),
    )
    border: Optional["Border"] = builtin_property(
        142,
        tags=("style",),
    )
    radius: Optional["Corner2"] = builtin_property(
        143,
        tags=("style",),
    )
