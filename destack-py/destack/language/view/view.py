from typing import TYPE_CHECKING, Optional, Union

from destack.language.core import (
    Entity,
    Event,
    IsDeletable,
    IsExtensible,
    IsSourceable,
    IsTaggable,
    IsViewable,
    NodeType,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import (
        ContainerView,
        Dimension,
        Layer,
        Position,
        View,
    )

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.VIEW_EVENT, frozen=True, is_abstract=True)
class ViewEvent(Event["View"]):
    """A Event regarding a View."""

    node: "View" = builtin_property(101)


@builtin_node(
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
)
class View(
    IsViewable,
    IsTaggable,
    IsExtensible,
    IsDeletable,
    IsSourceable,
    Entity,
):
    """A View is a graphical interface."""

    parent: Union["Layer", "ContainerView", None] = builtin_property_parent()
    name: str = builtin_property(101, is_repr=True)

    # sizing
    position: Optional["Position"] = builtin_property(110)
    width: Optional["Dimension"] = builtin_property(111)
    height: Optional["Dimension"] = builtin_property(112)
    min_width: Optional["Dimension"] = builtin_property(113)
    min_height: Optional["Dimension"] = builtin_property(114)
    max_width: Optional["Dimension"] = builtin_property(115)
    max_height: Optional["Dimension"] = builtin_property(116)
