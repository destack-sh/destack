from typing import TYPE_CHECKING, Optional

from destack.language.core import NodeType, builtin_node, property_

from ..view import View

if TYPE_CHECKING:
    from destack.language import Align

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.CONTENT_VIEW, is_abstract=True)
class ContentView(View):
    """A content View."""

    # layout
    align: Optional["Align"] = property_(53)

    # appearance
    is_visible: Optional[bool] = property_(60)
    opacity: Optional[float] = property_(61)
