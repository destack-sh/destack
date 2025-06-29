from typing import Optional

from destack.language.core import Node, NodeType, builtin_node, property_

from .input import InputView

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.NUMBER_INPUT_VIEW)
class NumberInputView(
    InputView,
    Node,
):
    """A general number input View."""

    value: Optional[str] = property_(100)
    placeholder: Optional[str] = property_(101)
