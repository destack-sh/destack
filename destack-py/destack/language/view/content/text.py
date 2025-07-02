from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    NodeType,
    builtin_node,
    builtin_property,
)

from .content import ContentView

if TYPE_CHECKING:
    from destack.language import Fill, Font

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.TEXT_VIEW)
class TextView(ContentView):
    """A (rich) text view."""

    # appearance
    user_select: Optional[bool] = builtin_property(65)
    font: Optional["Font"] = builtin_property(66)
    color: Optional["Fill"] = builtin_property(67)

    # text
    text: Optional[str] = builtin_property(100)
