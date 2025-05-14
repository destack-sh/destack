from typing import Optional

from bench.language.core import NodeType, node_, p_regular
from bench.pb2 import SliderViewData

from .view import InputViewBase


@node_(NodeType.SLIDER_VIEW)
class SliderView(InputViewBase[SliderViewData]):
    """A slider input View."""

    value: Optional[float] = p_regular(40, default=None, require=False, array=False)
    min_value: Optional[float] = p_regular(41, default=None, require=False, array=False)
    max_value: Optional[float] = p_regular(42, default=None, require=False, array=False)
    step: Optional[float] = p_regular(43, default=None, require=False, array=False)
