from typing import Optional

from destack.core import Float64, NodeType, declare_entity, declare_property

from .input import InputView2D


@declare_entity(
    NodeType.NUMBER_INPUT_VIEW2D,
)
class NumberInputView2D(InputView2D):
    """A 2D number input View."""

    value: Optional[Float64] = declare_property(
        250,
        tag=None,
    )
    placeholder: Optional[str] = declare_property(
        251,
        tag=None,
    )
