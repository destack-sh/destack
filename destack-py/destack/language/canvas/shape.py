from destack.language.core import NodeType, builtin_node

from ..view import ContainerView


@builtin_node(NodeType.SHAPE)
class Shape(ContainerView):
    """A Shape is a View representing a Shape."""

    pass
