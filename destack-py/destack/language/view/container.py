from typing import TYPE_CHECKING, Optional

from destack.language.core import NodeType, builtin_node, builtin_property

from .view import View

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
        Vector2f,
    )

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(
    NodeType.CONTAINER_VIEW,
    is_abstract=True,
    expected_descendant_types=(NodeType.VIEW,),
)
class ContainerView(View):
    """
    A container View contains other Views.
    Containers can be laid out as stacks or grids.
    """

    # layout
    layout: Optional["Layout"] = builtin_property(120)
    direction: Optional["Direction"] = builtin_property(121)
    distribute: Optional["Distribute"] = builtin_property(122)
    align: Optional["Align"] = builtin_property(123)
    gap: Optional["Axis2"] = builtin_property(124)
    padding: Optional["Insets"] = builtin_property(125)
    grid: Optional["Grid"] = builtin_property(126)
    grid_span: Optional["GridSpan"] = builtin_property(127)
    aspect_ratio: Optional[float] = builtin_property(128)
    is_wrap: Optional[bool] = builtin_property(129)

    # appearance
    is_visible: Optional[bool] = builtin_property(140)
    opacity: Optional[float] = builtin_property(141)
    fill: Optional["Fill"] = builtin_property(142)
    rotation: Optional["Axis3"] = builtin_property(143)
    skew: Optional["Vector2f"] = builtin_property(144)
    scale: Optional[float] = builtin_property(145)
    shadow: Optional["Shadow"] = builtin_property(146)
    border: Optional["Border"] = builtin_property(147)
    radius: Optional["Corners"] = builtin_property(148)
