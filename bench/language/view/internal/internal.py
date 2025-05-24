from typing import TYPE_CHECKING

from bench.language.core import TraitType, trait_

from ..view import IsView

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@trait_(TraitType.INTERNAL_VIEW)
class IsInternalView(IsView):
    """A content View."""

    pass
