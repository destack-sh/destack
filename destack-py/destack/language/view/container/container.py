from typing import TYPE_CHECKING, Optional

from destack.language.core import IsExtensible, NodeType, builtin_node, property_

from ..view import View

if TYPE_CHECKING:
    from destack.language import (
        Align,
        Axis2,
        Axis3,
        Border,
        Corners,
        Direction,
        Distribute,
        Fill,
        Grid,
        GridSpan,
        Insets,
        Layout,
        Shadow,
        Vector2,
    )

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.CONTAINER_VIEW, is_abstract=True)
class ContainerView(View, IsExtensible):
    """A container View contains other Views."""

    # layout
    layout: Optional["Layout"] = property_(50)
    direction: Optional["Direction"] = property_(51)
    distribute: Optional["Distribute"] = property_(52)
    align: Optional["Align"] = property_(53)
    gap: Optional["Axis2"] = property_(54)
    padding: Optional["Insets"] = property_(55)
    grid: Optional["Grid"] = property_(56)
    grid_span: Optional["GridSpan"] = property_(57)
    aspect_ratio: Optional[float] = property_(58)
    is_wrap: Optional[bool] = property_(59)

    # appearance
    is_visible: Optional[bool] = property_(60)
    opacity: Optional[float] = property_(61)
    fill: Optional["Fill"] = property_(62)
    rotation: Optional["Axis3"] = property_(63)
    skew: Optional["Vector2"] = property_(64)
    scale: Optional[float] = property_(65)
    shadow: Optional["Shadow"] = property_(66)
    border: Optional["Border"] = property_(67)
    radius: Optional["Corners"] = property_(68)
