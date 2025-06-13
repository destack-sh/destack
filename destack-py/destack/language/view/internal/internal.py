from typing import TYPE_CHECKING

from destack.language.core import TraitType, builtin_trait

from ..view import View

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_trait(TraitType.INTERNAL_VIEW)
class InternalView(View):
    """A content View."""

    pass
