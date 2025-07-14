from typing import TYPE_CHECKING, Optional

from destack.language import Node, NodeType, builtin_node, builtin_property_parent

if TYPE_CHECKING:
    from destack.language import Space


@builtin_node(NodeType.UNIVERSE, is_abstract=True)
class Universe(Node):
    """
    The Universe of Destack.
    An abstract container for useful constants.
    """

    parent: Optional["Space"] = builtin_property_parent()
