from typing import Optional

from bench.language.core import NodeType, node_, p_regular
from bench.pb2 import InputViewData

from .view import InputViewBase


@node_(NodeType.INPUT_VIEW)
class InputView(InputViewBase[InputViewData]):
    """A general number or string input View."""

    value: Optional[str] = p_regular(40, default=None)
    placeholder: Optional[str] = p_regular(41, default=None)
