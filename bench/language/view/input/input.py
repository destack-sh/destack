from typing import TYPE_CHECKING

from bench.language.core import Trait, VariableProperty, property_, trait_

from ..view import IsView

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@trait_(Trait.INPUT_VIEW)
class IsInputView(IsView):
    """An input View."""

    # appearance
    is_visible: VariableProperty[bool] = property_(60)
    opacity: VariableProperty[float] = property_(61)
