from typing import TYPE_CHECKING, Optional

from destack.core import Float32, NodeType, TagDeclaration, declare_entity, declare_property

from .view import View

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
    NodeType.LAYOUT_VIEW,
    is_abstract=True,
    expected_descendant_types=(NodeType.VIEW,),
    tags=(TagDeclaration(id=150, name="layout", description="The layout of the View."),),
)
class LayoutView(View):
    """
    A Layout View defines how its children  are laid out.
    """

    # layout
    layout: Optional["Layout"] = declare_property(
        150,
        description="How to layout the children of this View.",
        tag="layout",
    )
    direction: Optional["Direction"] = declare_property(
        151,
        description="The direction in which the children of this View are laid out.",
        tag="layout",
    )
    distribute: Optional["Distribute"] = declare_property(
        152,
        description="How to distribute the children of this View.",
        tag="layout",
    )
    align: Optional["Align"] = declare_property(
        153,
        description="How to align the children of this View.",
        tag="layout",
    )
    gap: Optional["Axis2"] = declare_property(
        154,
        description="The gap between the children of this View.",
        tag="layout",
    )
    padding: Optional["Inset2"] = declare_property(
        155,
        description="The padding around the children of this View.",
        tag="layout",
    )
    grid: Optional["Grid2"] = declare_property(
        156,
        description="The grid in which the children of this View are laid out.",
        tag="layout",
    )
    grid_span: Optional["GridSpan2"] = declare_property(
        157,
        description="The span of this View in the grid.",
        tag="layout",
    )
    aspect_ratio: Optional[Float32] = declare_property(
        158,
        description="The aspect ratio of this View.",
        tag="layout",
    )
    is_wrap: Optional[bool] = declare_property(
        159,
        description="Whether the children of this View are wrapped in the grid.",
        tag="layout",
    )
