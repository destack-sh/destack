from typing import TYPE_CHECKING, Optional

from bench.language.core import Trait, VariableProperty, property_, trait_

from ..view import IsView

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


@trait_(Trait.CONTAINER_VIEW)
class IsContainerView(IsView):
    """A container View contains other Views."""

    # layout
    layout: Optional["Layout"] = property_(50)
    direction: VariableProperty["Direction"] = property_(51)
    distribute: VariableProperty["Distribute"] = property_(52)
    align: VariableProperty["Align"] = property_(53)
    gap: VariableProperty["Axis2"] = property_(54)
    padding: VariableProperty["Insets"] = property_(55)
    grid: Optional["Grid"] = property_(56)
    grid_span: Optional["GridSpan"] = property_(57)
    aspect_ratio: Optional[float] = property_(58)
    is_wrap: Optional[bool] = property_(59)

    # appearance
    is_visible: VariableProperty[bool] = property_(60)
    opacity: VariableProperty[float] = property_(61)
    fill: VariableProperty["Fill"] = property_(62)
    rotation: VariableProperty["Axis3"] = property_(63)
    skew: VariableProperty["Vector2"] = property_(64)
    scale: VariableProperty[float] = property_(65)
    shadow: VariableProperty["Shadow"] = property_(66)
    border: VariableProperty["Border"] = property_(67)
    radius: VariableProperty["Corners"] = property_(68)
