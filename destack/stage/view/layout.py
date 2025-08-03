from typing import TYPE_CHECKING, Optional

from destack.core import Float32, NodeType, declare_entity, declare_property

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

# pyright: reportIncompatibleVariableOverride=false


@declare_entity(
    NodeType.LAYOUT_VIEW,
    is_abstract=True,
    expected_descendant_types=(NodeType.VIEW,),
)
class LayoutView(View):
    """
    A Layout View defines how its children  are laid out.
    """

    # layout
    layout: Optional["Layout"] = declare_property(150, tags=("layout",))
    direction: Optional["Direction"] = declare_property(151, tags=("layout",))
    distribute: Optional["Distribute"] = declare_property(152, tags=("layout",))
    align: Optional["Align"] = declare_property(153, tags=("layout",))
    gap: Optional["Axis2"] = declare_property(154, tags=("layout",))
    padding: Optional["Inset2"] = declare_property(155, tags=("layout",))
    grid: Optional["Grid2"] = declare_property(156, tags=("layout",))
    grid_span: Optional["GridSpan2"] = declare_property(157, tags=("layout",))
    aspect_ratio: Optional[Float32] = declare_property(158, tags=("layout",))
    is_wrap: Optional[bool] = declare_property(159, tags=("layout",))
