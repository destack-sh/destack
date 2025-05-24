from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    IsArchivable,
    IsDeletable,
    TraitType,
    VariableProperty,
    property_,
    trait_,
)

from ..view import IsView

if TYPE_CHECKING:
    from bench.language import Align

# pyright: reportIncompatibleVariableOverride=false


@trait_(TraitType.CONTENT_VIEW)
class IsContentView(IsView, IsDeletable, IsArchivable):
    """A content View."""

    # layout
    align: Optional["Align"] = property_(53)

    # appearance
    is_visible: VariableProperty[bool] = property_(60)
    opacity: VariableProperty[float] = property_(61)
