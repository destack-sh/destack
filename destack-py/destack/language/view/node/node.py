from typing import TYPE_CHECKING

from destack.language.core import TraitType, trait_

from ..view import View

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@trait_(TraitType.NODE_VIEW)
class NodeView(View):
    """A node View."""
