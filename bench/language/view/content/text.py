from typing import TYPE_CHECKING, Optional

from bench.language.core import NodeType, VariableProperty, node_, p_regular
from bench.pb2 import TextViewData

from .content import IsContentView

if TYPE_CHECKING:
    from bench.language import Fill, Font


@node_(NodeType.TEXT_VIEW)
class TextView(IsContentView[TextViewData]):
    """A (rich) text view."""

    # appearance
    user_select: Optional[bool] = p_regular(65)
    font: Optional["Font"] = p_regular(66)
    color: VariableProperty["Fill"] = p_regular(67)

    # text
    text: VariableProperty[str] = p_regular(100)
