from typing import Optional

from destack.language.core import Node, NodeType, node_, property_
from destack.pb2 import SliderInputViewData

from .input import InputView

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.SLIDER_INPUT_VIEW)
class SliderInputView(
    InputView,
    Node[SliderInputViewData],
):
    """A slider input View."""

    value: Optional[float] = property_(100)
    min_value: Optional[float] = property_(101)
    max_value: Optional[float] = property_(102)
    step: Optional[float] = property_(103)
