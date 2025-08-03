from typing import Optional

from destack.core import Float64, NodeType, builtin_entity, builtin_property

from .input import InputView

# pyright: reportIncompatibleVariableOverride=false


@builtin_entity(
    NodeType.NUMBER_INPUT_VIEW,
)
class NumberInputView(InputView):
    """A general number input View."""

    value: Optional[Float64] = builtin_property(250)
    placeholder: Optional[str] = builtin_property(251)
