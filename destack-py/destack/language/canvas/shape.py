from destack.language.core import NodeType, builtin_node

from ..view import ContainerView


@builtin_node(NodeType.SHAPE, is_abstract=True)
class Shape(ContainerView):
    """A Shape is a View representing a Shape."""

    pass
