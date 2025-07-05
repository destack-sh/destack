from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    NodeType,
    Text,
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

    text: Optional[Text] = builtin_property(250)
    font: Optional["Font"] = builtin_property(201)
    color: Optional["Fill"] = builtin_property(202)
