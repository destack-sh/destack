from typing import TYPE_CHECKING, Optional

from destack.language.core import NodeType, builtin_node, property_

from ..view import ContainerView

if TYPE_CHECKING:
    from destack.language import Stroke


@builtin_node(NodeType.SHAPE, is_abstract=True)
class Shape(ContainerView):
    """A Shape is a View representing a Shape."""

    stroke: Optional["Stroke"] = property_(80, is_repr=True)
