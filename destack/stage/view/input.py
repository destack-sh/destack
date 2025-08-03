from typing import TYPE_CHECKING

from destack.core import NodeType, builtin_entity

from .view import View

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_entity(
    NodeType.INPUT_VIEW,
    is_abstract=True,
)
class InputView(View):
    """An input View."""

    pass
