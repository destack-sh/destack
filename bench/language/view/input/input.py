from typing import TYPE_CHECKING

from bench.language.core import NodeTrait, VariableProperty, node_trait_, p_regular

from ..view import ViewBase

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@node_trait_(NodeTrait.INPUT_VIEW)
class IsInputView(ViewBase):
    """An input View."""

    # appearance
    is_visible: VariableProperty[bool] = p_regular(60)
    opacity: VariableProperty[float] = p_regular(61)
