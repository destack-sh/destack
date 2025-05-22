from typing import TYPE_CHECKING

from bench.language.core import Trait, trait_

from ..view import IsView

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@trait_(Trait.INTERNAL_VIEW)
class IsInternalView(IsView):
    """A content View."""

    pass
