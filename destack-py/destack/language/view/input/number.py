from typing import Optional

from destack.language.core import NodeType, builtin_node, property_

from .input import InputView

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.NUMBER_INPUT_VIEW)
class NumberInputView(InputView):
    """A general number input View."""

    value: Optional[str] = property_(100)
    placeholder: Optional[str] = property_(101)
