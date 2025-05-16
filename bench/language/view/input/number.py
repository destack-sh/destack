from typing import Optional

from bench.language.core import NodeType, node_, p_regular
from bench.pb2 import NumberInputViewData

from .input import InputViewBase


@node_(NodeType.NUMBER_INPUT_VIEW)
class NumberInputView(InputViewBase[NumberInputViewData]):
    """A general number input View."""

    value: Optional[str] = p_regular(100)
    placeholder: Optional[str] = p_regular(101)
