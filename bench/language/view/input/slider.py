from typing import Optional

from bench.language.core import Node, NodeType, node_, p_regular
from bench.pb2 import SliderInputViewData

from .input import IsInputView

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.SLIDER_INPUT_VIEW)
class SliderInputView(IsInputView, Node[SliderInputViewData]):
    """A slider input View."""

    value: Optional[float] = p_regular(100)
    min_value: Optional[float] = p_regular(101)
    max_value: Optional[float] = p_regular(102)
    step: Optional[float] = p_regular(103)
