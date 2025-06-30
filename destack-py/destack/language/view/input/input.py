from typing import TYPE_CHECKING, Optional

from destack.language.core import NodeType, builtin_node, property_

from ..view import View

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.INPUT_VIEW, is_abstract=True)
class InputView(View):
    """An input View."""

    # appearance
    is_visible: Optional[bool] = property_(60)
    opacity: Optional[float] = property_(61)
