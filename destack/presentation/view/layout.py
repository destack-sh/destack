from typing import TYPE_CHECKING, Optional

from destack.core import Float32, NodeType, TagDeclaration, declare_entity, declare_property

from .view import View2D

if TYPE_CHECKING:
    from destack import (
        Align,
        Axis2,
        Direction,
        Distribute,
        Grid2,
        GridSpan2,
        Inset2,
        Layout,
    )


@declare_entity(
    NodeType.LAYOUT_VIEW2D,
    is_abstract=True,
    expected_descendant_types=(NodeType.VIEW2D,),
    tags=(TagDeclaration(id=180, name="layout", description="The layout of the View."),),
)
class LayoutView2D(View2D):
    """
    A 2D layout View defines how its children are laid out.
    """

    # layout
    layout: Optional["Layout"] = declare_property(
        180,
        description="How to layout the children of this View.",
        tag="layout",
    )
    direction: Optional["Direction"] = declare_property(
        181,
        description="The direction in which the children of this View are laid out.",
        tag="layout",
    )
    distribute: Optional["Distribute"] = declare_property(
        182,
        description="How to distribute the children of this View.",
        tag="layout",
    )
    align: Optional["Align"] = declare_property(
        183,
        description="How to align the children of this View.",
        tag="layout",
    )
    gap: Optional["Axis2"] = declare_property(
        184,
        description="The gap between the children of this View.",
        tag="layout",
    )
    padding: Optional["Inset2"] = declare_property(
        185,
        description="The padding around the children of this View.",
        tag="layout",
    )
    grid: Optional["Grid2"] = declare_property(
        186,
        description="The grid in which the children of this View are laid out.",
        tag="layout",
    )
    grid_span: Optional["GridSpan2"] = declare_property(
        187,
        description="The span of this View in the grid.",
        tag="layout",
    )
    aspect_ratio: Optional[Float32] = declare_property(
        188,
        description="The aspect ratio of this View.",
        tag="layout",
    )
    is_wrap: Optional[bool] = declare_property(
        189,
        description="Whether the children of this View are wrapped in the grid.",
        tag="layout",
    )
