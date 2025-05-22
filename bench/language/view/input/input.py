from typing import TYPE_CHECKING

from bench.language.core import Trait, VariableProperty, p_regular, trait_

from ..view import IsView

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@trait_(Trait.INPUT_VIEW)
class IsInputView(IsView):
    """An input View."""

    # appearance
    is_visible: VariableProperty[bool] = p_regular(60)
    opacity: VariableProperty[float] = p_regular(61)
