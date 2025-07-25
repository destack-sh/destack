from typing import Optional

from destack.language.core import Float64, NodeType, builtin_node, builtin_property

from .input import InputView

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(
    NodeType.SLIDER_INPUT_VIEW,
)
class SliderInputView(InputView):
    """A slider input View."""

    value: Optional[Float64] = builtin_property(250)
