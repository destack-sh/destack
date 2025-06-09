from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Node,
    NodeType,
    node_,
    property_,
)
from destack.pb2 import TextViewData

from .content import IsContentView

if TYPE_CHECKING:
    from destack.language import Fill, Font

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.TEXT_VIEW)
class TextView(IsContentView, Node[TextViewData]):
    """A (rich) text view."""

    # appearance
    user_select: Optional[bool] = property_(65)
    font: Optional["Font"] = property_(66)
    color: Optional["Fill"] = property_(67)

    # text
    text: Optional[str] = property_(100)
