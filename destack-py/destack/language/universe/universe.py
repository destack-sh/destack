from destack.language import Node, NodeType, builtin_node


@builtin_node(NodeType.UNIVERSE, root_type=None, is_abstract=True)
class Universe(Node):
    """
    The Universe of Destack.
    An abstract container for useful constants.
    """

    pass
