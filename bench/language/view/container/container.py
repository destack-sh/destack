from typing import TYPE_CHECKING, Optional

from bench.language.core import MaybeVariable, node_component_, p_regular
from bench.pb2 import AnyNodeData

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


@node_component_()
class ContainerViewBase[NodeDataT: AnyNodeData](ViewBase[NodeDataT]):
    """A container View contains other Views."""

    # layout
    layout: Optional["Layout"] = p_regular(50)
    direction: MaybeVariable["Direction"] = p_regular(51)
    distribute: MaybeVariable["Distribute"] = p_regular(52)
    align: MaybeVariable["Align"] = p_regular(53)
    gap: MaybeVariable["Axis2"] = p_regular(54)
    padding: MaybeVariable["Insets"] = p_regular(55)
    grid: Optional["Grid"] = p_regular(56)
    grid_span: Optional["GridSpan"] = p_regular(57)
    aspect_ratio: Optional[float] = p_regular(58)
    is_wrap: Optional[bool] = p_regular(59)

    # appearance
    is_visible: MaybeVariable[bool] = p_regular(60)
    opacity: MaybeVariable[float] = p_regular(61)
    fill: MaybeVariable["Fill"] = p_regular(62)
    rotation: MaybeVariable["Axis3"] = p_regular(63)
    skew: MaybeVariable["Vector2"] = p_regular(64)
    scale: MaybeVariable[float] = p_regular(65)
    shadow: MaybeVariable["Shadow"] = p_regular(66)
    border: MaybeVariable["Border"] = p_regular(67)
    radius: MaybeVariable["Corners"] = p_regular(68)
