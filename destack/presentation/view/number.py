from typing import Optional

from destack.core import Float64, NodeType, declare_entity, declare_property

from .input import InputView


@declare_entity(
    NodeType.NUMBER_INPUT_VIEW,
)
class NumberInputView(InputView):
    """A general number input View."""

    value: Optional[Float64] = declare_property(
        250,
        tag=None,
    )
    placeholder: Optional[str] = declare_property(
        251,
        tag=None,
    )
