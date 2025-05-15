from typing import Optional

from bench.language.core import NodeType, node_, p_regular
from bench.pb2 import NumberViewData

from .input import InputViewBase


@node_(NodeType.NUMBER_INPUT_VIEW)
class NumberInputView(InputViewBase[NumberViewData]):
    """A general number input View."""

    value: Optional[str] = p_regular(100, default=None)
    placeholder: Optional[str] = p_regular(101, default=None)
