from typing import TYPE_CHECKING, Optional

from destack.language.core import NodeType, builtin_node, builtin_property

from ..view import View

if TYPE_CHECKING:
    from destack.language import Stroke


@builtin_node(NodeType.SHAPE, is_abstract=True)
class Shape(View):
    """A Shape is a View representing geometric Shapes."""

    stroke: Optional["Stroke"] = builtin_property(180, is_repr=True)


@builtin_node(NodeType.SHAPE2D, is_abstract=True)
class Shape2D(Shape):
    """A Shape2D represents 2-dimensional Shapes."""

    pass
