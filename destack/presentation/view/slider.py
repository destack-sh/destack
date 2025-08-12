from typing import Optional

from destack.core import Float64, NodeType, declare_entity, declare_property

from .input import InputView2D


@declare_entity(
    NodeType.SLIDER_INPUT_VIEW2D,
)
class SliderInputView2D(InputView2D):
    """A 2D slider input View."""

    value: Optional[Float64] = declare_property(
        250,
        tag=None,
    )
