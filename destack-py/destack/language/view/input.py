from typing import TYPE_CHECKING

from destack.language.core import NodeType, builtin_node

from .view import View

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.INPUT_VIEW, is_abstract=True)
class InputView(View):
    """An input View."""

    pass
