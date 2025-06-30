from typing import Optional

from destack.language.core import NodeType, builtin_node, property_

from .input import InputView

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.SLIDER_INPUT_VIEW)
class SliderInputView(InputView):
    """A slider input View."""

    value: Optional[float] = property_(100)
    min_value: Optional[float] = property_(101)
    max_value: Optional[float] = property_(102)
    step: Optional[float] = property_(103)
