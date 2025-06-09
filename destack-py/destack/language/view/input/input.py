from typing import TYPE_CHECKING, Optional

from destack.language.core import TraitType, property_, trait_

from ..view import IsView

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@trait_(TraitType.INPUT_VIEW)
class IsInputView(IsView):
    """An input View."""

    # appearance
    is_visible: Optional[bool] = property_(60)
    opacity: Optional[float] = property_(61)
