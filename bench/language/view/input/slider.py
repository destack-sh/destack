from typing import Optional

from bench.language.core import IsArchivable, IsDeletable, Node, NodeType, node_, property_
from bench.pb2 import SliderInputViewData

from .input import IsInputView

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.SLIDER_INPUT_VIEW)
class SliderInputView(
    IsInputView,
    IsDeletable,
    IsArchivable,
    Node[SliderInputViewData],
):
    """A slider input View."""

    value: Optional[float] = property_(100)
    min_value: Optional[float] = property_(101)
    max_value: Optional[float] = property_(102)
    step: Optional[float] = property_(103)
