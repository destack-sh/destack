from typing import TYPE_CHECKING

from destack.language.core import NodeType, builtin_node

from .view import View

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(
    NodeType.CONTENT_VIEW,
    is_extensible=True,
    is_abstract=True,
)
class ContentView(View):
    """A content View."""

    pass
