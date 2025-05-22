from typing import TYPE_CHECKING, Optional

from bench.language.core import NodeTrait, VariableProperty, node_trait_, p_regular

from ..view import ViewBase

if TYPE_CHECKING:
    from bench.language import (
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


@node_trait_(NodeTrait.CONTAINER_VIEW)
class IsContainerView(ViewBase):
    """A container View contains other Views."""

    # layout
    layout: Optional["Layout"] = p_regular(50)
    direction: VariableProperty["Direction"] = p_regular(51)
    distribute: VariableProperty["Distribute"] = p_regular(52)
    align: VariableProperty["Align"] = p_regular(53)
    gap: VariableProperty["Axis2"] = p_regular(54)
    padding: VariableProperty["Insets"] = p_regular(55)
    grid: Optional["Grid"] = p_regular(56)
    grid_span: Optional["GridSpan"] = p_regular(57)
    aspect_ratio: Optional[float] = p_regular(58)
    is_wrap: Optional[bool] = p_regular(59)

    # appearance
    is_visible: VariableProperty[bool] = p_regular(60)
    opacity: VariableProperty[float] = p_regular(61)
    fill: VariableProperty["Fill"] = p_regular(62)
    rotation: VariableProperty["Axis3"] = p_regular(63)
    skew: VariableProperty["Vector2"] = p_regular(64)
    scale: VariableProperty[float] = p_regular(65)
    shadow: VariableProperty["Shadow"] = p_regular(66)
    border: VariableProperty["Border"] = p_regular(67)
    radius: VariableProperty["Corners"] = p_regular(68)
