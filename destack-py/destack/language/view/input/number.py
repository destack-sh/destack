from typing import Optional

from destack.language.core import NodeType, builtin_node, builtin_property

from .input import InputView

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.NUMBER_INPUT_VIEW)
class NumberInputView(InputView):
    """A general number input View."""

    value: Optional[str] = builtin_property(100)
    placeholder: Optional[str] = builtin_property(101)
