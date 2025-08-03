from typing import Optional

from destack.core import Float64, NodeType, builtin_entity, builtin_property

from .input import InputView

# pyright: reportIncompatibleVariableOverride=false


@builtin_entity(
    NodeType.SLIDER_INPUT_VIEW,
)
class SliderInputView(InputView):
    """A slider input View."""

    value: Optional[Float64] = builtin_property(250)
