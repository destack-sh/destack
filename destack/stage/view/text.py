from typing import TYPE_CHECKING, Optional

from destack.core import (
    NodeType,
    Text,
    declare_entity,
    declare_property,
)

from .content import ContentView

if TYPE_CHECKING:
    from destack import Fill, Font

# pyright: reportIncompatibleVariableOverride=false


@declare_entity(
    NodeType.TEXT_VIEW,
)
class TextView(ContentView):
    """A (rich) text view."""

    text: Optional[Text] = declare_property(250)
    font: Optional["Font"] = declare_property(201)
    color: Optional["Fill"] = declare_property(202)
