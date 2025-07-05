from typing import TYPE_CHECKING, Optional, Union

from destack.language.core import (
    Entity,
    Event,
    IsDeletable,
    IsExtensible,
    IsOrdered,
    IsSpatial,
    IsTaggable,
    NodeType,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import (
        ContainerView,
        Dimension,
        Folder,
        Layer,
        Position,
        Scene,
        View,
        Window,
    )

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.VIEW_EVENT, frozen=True, is_abstract=True)
class ViewEvent(Event["View"]):
    """A Event regarding a View."""

    node: "View" = builtin_property(101)


@builtin_node(NodeType.VIEW_ENTERED_EVENT, frozen=True)
class ViewEnteredEvent(ViewEvent):
    """A View was entered."""

    pass


@builtin_node(NodeType.VIEW_EXITED_EVENT, frozen=True)
class ViewExitedEvent(ViewEvent):
    """A View was exited."""

    pass


@builtin_node(
    NodeType.VIEW,
    is_abstract=True,
    event_types=(
        NodeType.VIEW_EVENT,
        NodeType.POINTER_EVENT,
        NodeType.MOUSE_EVENT,
        NodeType.KEYBOARD_EVENT,
        NodeType.DRAG_EVENT,
        NodeType.CLIPBOARD_EVENT,
        NodeType.FOCUS_EVENT,
    ),
)
class View(
    IsSpatial,
    Entity,
    IsOrdered,
    IsTaggable,
    IsExtensible,
    IsDeletable,
):
    """A View is a graphical interface."""

    parent: Union["Window", "Scene", "Layer", "ContainerView", "Folder", None] = (
        builtin_property_parent()
    )
    name: str = builtin_property(101, is_repr=True)

    # sizing
    position: Optional["Position"] = builtin_property(110)
    width: Optional["Dimension"] = builtin_property(111)
    height: Optional["Dimension"] = builtin_property(112)
    min_width: Optional["Dimension"] = builtin_property(113)
    min_height: Optional["Dimension"] = builtin_property(114)
    max_width: Optional["Dimension"] = builtin_property(115)
    max_height: Optional["Dimension"] = builtin_property(116)
