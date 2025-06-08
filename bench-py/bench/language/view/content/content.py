from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    TraitType,
    property_,
    trait_,
)

from ..view import IsView

if TYPE_CHECKING:
    from bench.language import Align

# pyright: reportIncompatibleVariableOverride=false


@trait_(TraitType.CONTENT_VIEW)
class IsContentView(IsView):
    """A content View."""

    # layout
    align: Optional["Align"] = property_(53)

    # appearance
    is_visible: Optional[bool] = property_(60)
    opacity: Optional[float] = property_(61)
