from typing import TYPE_CHECKING, Optional

from destack.language.core import NodeType, builtin_node, builtin_property

from .view import View

if TYPE_CHECKING:
    from destack.language import (
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


@builtin_node(
    NodeType.LAYOUT_VIEW,
    is_extensible=True,
    is_abstract=True,
    expected_descendant_types=(NodeType.VIEW,),
)
class LayoutView(View):
    """
    A Layout View defines how its children  are laid out.
    """

    # layout
    layout: Optional["Layout"] = builtin_property(150, tags=("layout",))
    direction: Optional["Direction"] = builtin_property(151, tags=("layout",))
    distribute: Optional["Distribute"] = builtin_property(152, tags=("layout",))
    align: Optional["Align"] = builtin_property(153, tags=("layout",))
    gap: Optional["Axis2"] = builtin_property(154, tags=("layout",))
    padding: Optional["Inset2"] = builtin_property(155, tags=("layout",))
    grid: Optional["Grid2"] = builtin_property(156, tags=("layout",))
    grid_span: Optional["GridSpan2"] = builtin_property(157, tags=("layout",))
    aspect_ratio: Optional[float] = builtin_property(158, tags=("layout",))
    is_wrap: Optional[bool] = builtin_property(159, tags=("layout",))
