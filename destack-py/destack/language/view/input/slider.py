from typing import Optional

from destack.language.core import NodeType, builtin_node, builtin_property

from .input import InputView

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.SLIDER_INPUT_VIEW)
class SliderInputView(InputView):
    """A slider input View."""

    value: Optional[float] = builtin_property(250)
    min_value: Optional[float] = builtin_property(251)
    max_value: Optional[float] = builtin_property(252)
    step: Optional[float] = builtin_property(253)
