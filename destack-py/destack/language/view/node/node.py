from typing import TYPE_CHECKING

from destack.language.core import TraitType, builtin_trait

from ..view import View

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_trait(TraitType.NODE_VIEW)
class NodeView(View):
    """A node View."""
