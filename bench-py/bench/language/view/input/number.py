from typing import Optional

from bench.language.core import Node, NodeType, node_, property_
from bench.pb2 import NumberInputViewData

from .input import IsInputView

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.NUMBER_INPUT_VIEW)
class NumberInputView(
    IsInputView,
    Node[NumberInputViewData],
):
    """A general number input View."""

    value: Optional[str] = property_(100)
    placeholder: Optional[str] = property_(101)
