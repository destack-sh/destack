from typing import TYPE_CHECKING, Optional

from destack.language.core import NodeType, builtin_node, builtin_property

from ..view import View

if TYPE_CHECKING:
    from destack.language import Align

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.CONTENT_VIEW, is_abstract=True)
class ContentView(View):
    """A content View."""

    # layout
    align: Optional["Align"] = builtin_property(53)

    # appearance
    is_visible: Optional[bool] = builtin_property(60)
    opacity: Optional[float] = builtin_property(61)
