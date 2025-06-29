from typing import TYPE_CHECKING

from destack.language.core import NodeType, builtin_node

from ..view import View

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.INTERNAL_VIEW)
class InternalView(View):
    """A content View."""

    pass
