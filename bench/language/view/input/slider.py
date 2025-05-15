from typing import Optional

from bench.language.core import NodeType, node_, p_regular
from bench.pb2 import SliderViewData

from .input import InputViewBase


@node_(NodeType.SLIDER_INPUT_VIEW)
class SliderInputView(InputViewBase[SliderViewData]):
    """A slider input View."""

    value: Optional[float] = p_regular(100, default=None, require=False, array=False)
    min_value: Optional[float] = p_regular(101, default=None, require=False, array=False)
    max_value: Optional[float] = p_regular(102, default=None, require=False, array=False)
    step: Optional[float] = p_regular(103, default=None, require=False, array=False)
