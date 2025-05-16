from typing import TYPE_CHECKING, Optional

from bench.language.core import StructType, node_component_, p_regular
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
    layout: Optional["Layout"] = p_regular(50, require=False)
    direction: Optional["Direction"] = p_regular(51, require=False)
    distribute: Optional["Distribute"] = p_regular(52, require=False)
    align: Optional["Align"] = p_regular(53, require=False)
    gap: Optional["Axis2"] = p_regular(54, require=False, struct=StructType.AXIS2)
    padding: Optional["Insets"] = p_regular(55, require=False, struct=StructType.INSETS)
    grid: Optional["Grid"] = p_regular(56, require=False, struct=StructType.GRID)
    grid_span: Optional["GridSpan"] = p_regular(57, require=False, struct=StructType.GRID_SPAN)
    aspect_ratio: Optional[float] = p_regular(58, require=False)
    wrap: Optional[bool] = p_regular(59, require=False)

    # appearance
    visible: Optional[bool] = p_regular(60, require=False)
    opacity: Optional[float] = p_regular(61, require=False)
    fill: Optional["Fill"] = p_regular(62, require=False, struct=StructType.FILL)
    rotation: Optional["Axis3"] = p_regular(63, require=False, struct=StructType.AXIS3)
    skew: Optional["Vector2"] = p_regular(64, require=False, struct=StructType.VECTOR2)
    scale: Optional[float] = p_regular(65, require=False)
    shadow: Optional["Shadow"] = p_regular(66, require=False, struct=StructType.SHADOW)
    border: Optional["Border"] = p_regular(67, require=False, struct=StructType.BORDER)
    radius: Optional["Corners"] = p_regular(68, require=False, struct=StructType.CORNERS)
