from typing import TYPE_CHECKING

from bench.language.core import NodeTrait, node_trait_

from ..view import ViewBase

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@node_trait_(NodeTrait.NODE_VIEW)
class IsNodeView(ViewBase):
    """A node View."""
