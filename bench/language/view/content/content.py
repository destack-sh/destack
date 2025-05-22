from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    IsArchivable,
    IsDeletable,
    Trait,
    VariableProperty,
    p_regular,
    trait_,
)

from ..view import IsView

if TYPE_CHECKING:
    from bench.language import Align

# pyright: reportIncompatibleVariableOverride=false


@trait_(Trait.CONTENT_VIEW)
class IsContentView(IsView, IsDeletable, IsArchivable):
    """A content View."""

    # layout
    align: Optional["Align"] = p_regular(53)

    # appearance
    is_visible: VariableProperty[bool] = p_regular(60)
    opacity: VariableProperty[float] = p_regular(61)
