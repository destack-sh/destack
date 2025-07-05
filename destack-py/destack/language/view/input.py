from typing import TYPE_CHECKING, Optional

from destack.language.core import NodeType, builtin_node, builtin_property

from .view import View

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.INPUT_VIEW, is_abstract=True)
class InputView(View):
    """An input View."""

    # appearance
    is_visible: Optional[bool] = builtin_property(160)
    opacity: Optional[float] = builtin_property(161)
