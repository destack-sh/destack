from typing import Optional

from destack.core import Float64, NodeType, declare_entity, declare_property

from .input import InputView

# pyright: reportIncompatibleVariableOverride=false


@declare_entity(
    NodeType.SLIDER_INPUT_VIEW,
)
class SliderInputView(InputView):
    """A slider input View."""

    value: Optional[Float64] = declare_property(250)
