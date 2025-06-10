from typing import TYPE_CHECKING, Union

from destack.language.core import IsEntity, IsOwnable, Node, NodeType, node_, property_parent_
from destack.pb2 import StarData

if TYPE_CHECKING:
    from destack.language import IsStarable

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.STAR)
class Star(IsOwnable, IsEntity, Node[StarData]):
    """A Star is a relationship between someone and a Starred Node."""

    parent: Union["IsStarable", None] = property_parent_(node_is_customizable=True)
