from typing import TYPE_CHECKING

from destack.core import NodeType, declare_entity

from .view import View2D

if TYPE_CHECKING:
    pass


@declare_entity(
    NodeType.INPUT_VIEW2D,
    is_abstract=True,
)
class InputView2D(View2D):
    """An input View."""

    pass
