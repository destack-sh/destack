from typing import TYPE_CHECKING, Collection
from uuid import UUID

from bench.language.const import StructType
from bench.language.node import Node, Struct, struct

if TYPE_CHECKING:
    pass


class NodeVisitor:
    def __init__(self):
        self._reference_by_ck: dict[UUID, Node] = {}

    def __str__(self):
        return f"{len(self._reference_by_ck)} nodes"

    def __repr__(self):
        return f"<NodeVisitor {str(self)}>"

    @property
    def references(self) -> Collection["Node"]:
        return self._reference_by_ck.values()

    def visit_reference(self, node: "Node"):
        self._reference_by_ck[node.ck] = node


@struct(StructType.PROJECTION)
class Projection(Struct):
    """
    A projection into the graph.
    NOTE 'Projecting' is not quite right / complete yet, consider:
     - How do we filter and LoD this?
     - When do we inline out-of-line descendants (like Comments or local Records)?
        (esp. considering some out-of-line nodes would need to be fetched async)
     - How do we alias shadowed and anonymous nodes?
     - How do we make projections reproducible and inspectable in the editor?
    """


def render() -> str:
    raise NotImplementedError("nocheckin: render")
