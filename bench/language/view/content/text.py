from typing import TYPE_CHECKING, Optional

from bench.language.core import NodeType, StructType, node_, p_regular
from bench.pb2 import TextViewData

from .content import ContentViewBase

if TYPE_CHECKING:
    from bench.language import Fill, Font


@node_(NodeType.TEXT_VIEW)
class TextView(ContentViewBase[TextViewData]):
    """A text view."""

    # appearance
    user_select: Optional[bool] = p_regular(65, require=False)
    font: Optional["Font"] = p_regular(66, require=False, struct=StructType.FONT)
    color: Optional["Fill"] = p_regular(67, require=False, struct=StructType.FILL)

    # text
    # nocheckin: TextView.text - what type?
    text: str = p_regular(100)
