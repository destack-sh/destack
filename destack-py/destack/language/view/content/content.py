from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    TraitType,
    property_,
    trait_,
)

from ..view import View

if TYPE_CHECKING:
    from destack.language import Align

# pyright: reportIncompatibleVariableOverride=false


@trait_(TraitType.CONTENT_VIEW)
class ContentView(View):
    """A content View."""

    # layout
    align: Optional["Align"] = property_(53)

    # appearance
    is_visible: Optional[bool] = property_(60)
    opacity: Optional[float] = property_(61)
