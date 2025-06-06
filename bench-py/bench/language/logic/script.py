from bench.language.core import Node, NodeType, node_


@node_(NodeType.SCRIPT)
class Script(Node):
    """A Script."""

    pass
