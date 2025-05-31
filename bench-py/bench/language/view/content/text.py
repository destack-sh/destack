from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    IsArchivable,
    IsDeletable,
    Node,
    NodeType,
    VariableProperty,
    node_,
    property_,
)
from bench.pb2 import TextViewData

from .content import IsContentView

if TYPE_CHECKING:
    from bench.language import Fill, Font

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.TEXT_VIEW)
class TextView(
    IsContentView,
    IsDeletable,
    IsArchivable,
    Node[TextViewData],
):
    """A (rich) text view."""

    # appearance
    user_select: Optional[bool] = property_(65)
    font: Optional["Font"] = property_(66)
    color: VariableProperty["Fill"] = property_(67)

    # text
    text: VariableProperty[str] = property_(100)
