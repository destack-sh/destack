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


@declare_entity(
    NodeType.TEXT_VIEW,
)
class TextView(ContentView):
    """A (rich) text view."""

    text: Optional[Text] = declare_property(
        250,
        tag=None,
    )
    font: Optional["Font"] = declare_property(
        201,
        tag=None,
    )
    color: Optional["Fill"] = declare_property(
        202,
        tag=None,
    )
