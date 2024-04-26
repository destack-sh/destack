from typing import Optional

from bench.language.node import Node


def render(
    *nodes: Node,
    target="python",
    record_limit: int = 20,
    recursive: bool = True,
) -> Optional[str]:
    """
    Renders edits or nodes to code in a language.
    Nodes are coerced into create edits with all descendants.
    """
    raise NotImplementedError
