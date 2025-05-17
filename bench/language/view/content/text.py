from typing import TYPE_CHECKING, Optional

from bench.language.core import MaybeVariable, NodeType, node_, p_regular
from bench.pb2 import TextViewData

from .content import ContentViewBase

if TYPE_CHECKING:
    from bench.language import Fill, Font


@node_(NodeType.TEXT_VIEW)
class TextView(ContentViewBase[TextViewData]):
    """A (rich) text view."""

    # appearance
    user_select: Optional[bool] = p_regular(65)
    font: Optional["Font"] = p_regular(66)
    color: MaybeVariable["Fill"] = p_regular(67)

    # text
    # nocheckin: TextView.text - what type?
    text: MaybeVariable[str] = p_regular(100)
