from typing import TYPE_CHECKING

from destack.language.core import TraitType, trait_

from ..view import IsView

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@trait_(TraitType.NODE_VIEW)
class IsNodeView(IsView):
    """A node View."""
