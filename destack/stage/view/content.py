from typing import TYPE_CHECKING

from destack.core import NodeType, declare_entity

from .view import View

if TYPE_CHECKING:
    pass


@declare_entity(
    NodeType.CONTENT_VIEW,
    is_abstract=True,
)
class ContentView(View):
    """A content View."""

    pass
