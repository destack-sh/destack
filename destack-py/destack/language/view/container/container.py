from typing import TYPE_CHECKING, Optional

from destack.language.core import NodeType, builtin_node, builtin_property

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
class ContainerView(View):
    """A container View contains other Views."""

    # layout
    layout: Optional["Layout"] = builtin_property(50)
    direction: Optional["Direction"] = builtin_property(51)
    distribute: Optional["Distribute"] = builtin_property(52)
    align: Optional["Align"] = builtin_property(53)
    gap: Optional["Axis2"] = builtin_property(54)
    padding: Optional["Insets"] = builtin_property(55)
    grid: Optional["Grid"] = builtin_property(56)
    grid_span: Optional["GridSpan"] = builtin_property(57)
    aspect_ratio: Optional[float] = builtin_property(58)
    is_wrap: Optional[bool] = builtin_property(59)

    # appearance
    is_visible: Optional[bool] = builtin_property(60)
    opacity: Optional[float] = builtin_property(61)
    fill: Optional["Fill"] = builtin_property(62)
    rotation: Optional["Axis3"] = builtin_property(63)
    skew: Optional["Vector2"] = builtin_property(64)
    scale: Optional[float] = builtin_property(65)
    shadow: Optional["Shadow"] = builtin_property(66)
    border: Optional["Border"] = builtin_property(67)
    radius: Optional["Corners"] = builtin_property(68)
