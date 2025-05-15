from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    StructType,
    node_component_,
    p_regular,
)
from bench.pb2 import AnyNodeData

from ..view import ViewBase

if TYPE_CHECKING:
    from bench.language import (
        Align,
        Border,
        Direction,
        Distribute,
        Fill,
        Gap,
        Grid,
        GridSpan,
        Layout,
        Padding,
        Rotation,
        Shadow,
    )


@node_component_()
class ContainerViewBase[NodeDataT: AnyNodeData](ViewBase[NodeDataT]):
    """A container View contains other Views."""

    # border, padding, margin, fill, ...

    # layout
    layout: Optional["Layout"] = p_regular(50, require=False)
    direction: Optional["Direction"] = p_regular(51, require=False)
    distribute: Optional["Distribute"] = p_regular(52, require=False)
    align: Optional["Align"] = p_regular(53, require=False)
    gap: Optional["Gap"] = p_regular(54, require=False, struct=StructType.GAP)
    padding: Optional["Padding"] = p_regular(55, require=False, struct=StructType.PADDING)
    grid: Optional["Grid"] = p_regular(56, require=False, struct=StructType.GRID)
    grid_span: Optional["GridSpan"] = p_regular(57, require=False, struct=StructType.GRID_SPAN)
    aspect_ratio: Optional[float] = p_regular(58, require=False)

    # style
    fill: Optional["Fill"] = p_regular(60, require=False, struct=StructType.FILL)
    opacity: Optional[float] = p_regular(61, require=False)
    is_visible: Optional[bool] = p_regular(62, require=False)
    rotation: Optional["Rotation"] = p_regular(63, require=False, struct=StructType.ROTATION)
    shadow: Optional["Shadow"] = p_regular(64, require=False, struct=StructType.SHADOW)
    border: Optional["Border"] = p_regular(65, require=False, struct=StructType.BORDER)

    # behavior
    # focus: Optional[Node] = p_regular(
    #     70, default=None, require=False, array=False, references="any"
    # )
    # selection: Optional[Selection] = p_regular(
    #     71, default=None, require=False, struct=StructType.SELECTION
    # )
    ...  # actions/effects/...
