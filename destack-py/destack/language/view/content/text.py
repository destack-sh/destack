from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Node,
    NodeType,
    builtin_node,
    property_,
)
from destack.proto import TextViewProto

from .content import ContentView

if TYPE_CHECKING:
    from destack.language import Fill, Font

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.TEXT_VIEW)
class TextView(ContentView, Node[TextViewProto]):
    """A (rich) text view."""

    # appearance
    user_select: Optional[bool] = property_(65)
    font: Optional["Font"] = property_(66)
    color: Optional["Fill"] = property_(67)

    # text
    text: Optional[str] = property_(100)
