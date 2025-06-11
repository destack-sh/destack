from typing import Optional

from destack.language.core import Node, NodeType, node_, property_
from destack.pb2 import NumberInputViewData

from .input import InputView

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.NUMBER_INPUT_VIEW)
class NumberInputView(
    InputView,
    Node[NumberInputViewData],
):
    """A general number input View."""

    value: Optional[str] = property_(100)
    placeholder: Optional[str] = property_(101)
