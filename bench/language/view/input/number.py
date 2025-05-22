from typing import Optional

from bench.language.core import IsArchivable, IsDeletable, Node, NodeType, node_, p_regular
from bench.pb2 import NumberInputViewData

from .input import IsInputView

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.NUMBER_INPUT_VIEW)
class NumberInputView(
    IsInputView,
    IsDeletable,
    IsArchivable,
    Node[NumberInputViewData],
):
    """A general number input View."""

    value: Optional[str] = p_regular(100)
    placeholder: Optional[str] = p_regular(101)
