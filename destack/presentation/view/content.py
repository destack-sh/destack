from typing import TYPE_CHECKING

from destack.core import NodeType, declare_entity

from .view import View2D

if TYPE_CHECKING:
    pass


@declare_entity(
    NodeType.CONTENT_VIEW2D,
    is_abstract=True,
)
class ContentView2D(View2D):
    """A 2D content View."""

    pass
