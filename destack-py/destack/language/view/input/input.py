from typing import TYPE_CHECKING, Optional

from destack.language.core import TraitType, builtin_trait, property_

from ..view import View

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_trait(TraitType.INPUT_VIEW)
class InputView(View):
    """An input View."""

    # appearance
    is_visible: Optional[bool] = property_(60)
    opacity: Optional[float] = property_(61)
